# RelXen Arkadasa Devir ve Proje Bilgilendirme

Bu dosya, projeyi teslim alacak veya inceleyecek kişiye RelXen'in ne olduğunu, şu anda neleri yaptığını, hangi komutlarla çalıştığını ve hangi canlı işlem yollarına sahip olduğunu ayrıntılı anlatır. En güncel teknik durum için `docs/PROJECT_STATE.md`, günlük operatör akışı için `docs/RUNBOOK.md`, karar geçmişi için `docs/DECISIONS.md` okunabilir.

## 1. Proje Nedir?

RelXen, Binance USD-M Futures için geliştirilmiş yerel, tek kullanıcılı bir işlem panelidir. Strateji sinyali olarak kapalı mumlar üzerinde Average Sentiment Oscillator (ASO) kullanır. Uygulama hem paper trading çalıştırabilir hem de Binance TESTNET üzerinde gerçek testnet emirleri gönderebilir. MAINNET tarafında manuel canary ve session-scoped auto live altyapısı da vardır.

Teknoloji yapısı:

- Backend: Rust workspace, Axum HTTP/WebSocket server.
- Domain: ASO, sinyal, risk, paper engine, performans hesapları.
- Persistence: SQLite, otomatik migration, WAL mode.
- Frontend: React + Vite + TypeScript, Zustand, TanStack Query, `lightweight-charts`.
- Exchange: Binance USD-M Futures REST, WebSocket kline stream, user-data stream.
- Secret handling: OS secure storage veya açıkça seçilen local `.env` credential source.

## 2. Su Anki Genel Durum

- Paper Mode V1 release-candidate seviyesinde ve uçtan uca çalışıyor.
- TESTNET canlı emir akışı çalışıyor: `MARKET`, `LIMIT`, cancel, cancel-all-active-symbol, flatten, closed-candle TESTNET auto.
- MAINNET manuel canary akışı çalışıyor ama server flag, risk profile, fresh state ve exact confirmation istiyor.
- MAINNET auto altyapısı var: dry-run, live start, stop, status, watchdog, risk budget, evidence export, lessons, margin policy ve ASO position policy.
- MAINNET auto normal startup'ta kapalıdır; live mode için env config, risk budget, exact confirmation ve session-scoped start gerekir.
- Son teknik odak: operator-stop MAINNET auto run sonrası `shadow_stale` ve transient market-data reconnect false-positive stop hardening yapıldı. Sıradaki iş live run açmak değil; MAINNET auto idle kalırken verification gate ve disabled-live-auto smoke tamamlamak.

## 3. Ne Yapabiliyor?

### 3.1 Paper Trading

- `BTCUSDT` ve `BTCUSDC` destekler.
- Tek aktif sembol ve tek açık pozisyon mantığıyla çalışır.
- Binance Futures REST'ten tarihsel kline yükler.
- Binance WebSocket kline stream ile canlı mum günceller.
- Kapalı mumlardan ASO hesaplar.
- ASO crossover sinyallerinden LONG/SHORT paper trade üretir.
- Paper wallet, paper position, trade history, signal history ve performans verisini SQLite'ta tutar.
- Runtime koparsa bounded REST repair yapar; süreklilik kanıtlanamazsa `resync_required` yayınlar.
- Dashboard üzerinden runtime start/stop, settings apply, paper close-all ve paper reset yapılabilir.

### 3.2 Dashboard

- Backend `web/dist` içindeki build'i `/` üzerinden servis eder.
- İlk açılışta `/api/bootstrap` snapshot'ı alır.
- `/api/ws` ile WebSocket event stream'e bağlanır.
- Chart, ASO, sinyal, pozisyon, wallet, performans, runtime state ve logları gösterir.
- LIVE ACCESS paneli credential, readiness, shadow, preview, preflight, kill switch, TESTNET execution, TESTNET auto, MAINNET canary ve MAINNET auto durumlarını gösterir.

### 3.3 Credential ve Live Readiness

- Credential metadata SQLite'ta maskeli tutulur.
- Raw secret, normal secure-store modunda OS secure storage içinde durur.
- `RELXEN_CREDENTIAL_SOURCE=env` verilirse `.env` içindeki credential process environment'tan okunur; SQLite'a raw secret yazılmaz.
- TESTNET env credential, env-source authoritative modda startup'ta seçilebilir.
- MAINNET env credential otomatik seçilmez; açık seçim/validation gerekir.
- Binance signed read-only validation yapar.
- Active symbol için account snapshot ve symbol rules çeker.
- Dedicated position mode ve multi-assets mode endpointlerini kontrol eder.
- One-way ve single-asset varsayımı dışındaki durumlarda execution gate fail-closed çalışır.

### 3.4 Live Shadow ve Reconciliation

- Binance listenKey oluşturur, keepalive/close lifecycle yönetir.
- User-data stream eventlerini işler.
- Account, position, open-order, fill ve account-config değişimlerini shadow state'e yansıtır.
- Shadow state stale/degraded/ambiguous olursa execution gate bloklar.
- Manual `Refresh Shadow`, read-only REST shadow refresh yanında bounded recent-window execution repair de yapar.
- Binance order submission `ACK` kullanır; final order/fill/account gerçeği user-data stream ve REST repair ile belirlenir.

### 3.5 Intent Preview ve Preflight

- `MARKET` veya `LIMIT` order intent preview üretir.
- Decimal/rules-aware quantity ve price rounding yapar.
- Tick size, step size, min quantity, min notional, symbol status, balance, leverage ve mode kontrollerini uygular.
- TESTNET `POST /fapi/v1/order/test` preflight çalıştırır.
- Preflight result SQLite'a yazılır ve dashboard'da listelenir.
- Preflight order placement değildir; gerçek emir `execute` ile ayrı yapılır.

### 3.6 TESTNET Execution

- Açık confirmation ile TESTNET `MARKET` / `LIMIT` emir gönderebilir.
- RelXen-created open order cancel yapabilir.
- Active-symbol open order'lar için cancel-all yapabilir.
- Shadow pozisyon net ve güvenliyse reduce-only MARKET flatten yapabilir.
- Closed-candle ASO sinyalleriyle TESTNET auto executor çalıştırabilir.
- Duplicate signal/open-time intent suppression persisted olarak tutulur.
- Kill switch yeni live submission'ları anında bloklar.

### 3.7 MAINNET Manuel Canary

- Manual MAINNET canary path implemente edilmiştir.
- Server tarafında `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` gerekir.
- Mainnet credential açıkça seçilip validate edilmelidir.
- Risk profile configured olmalıdır.
- Shadow, rules, account, reference price fresh olmalıdır.
- One-way, single-asset mode, supported symbol ve risk cap gate'leri geçmelidir.
- Preview non-marketable `LIMIT` olmalıdır.
- UI/API tarafından verilen exact confirmation text gönderilmelidir.
- Daha önce gerçek MAINNET canary emirleri submit/cancel/reconcile edilmiş ve evidence altında kayıtlıdır.

### 3.8 MAINNET Auto Dry-Run

- MAINNET auto dry-run altyapısı vardır.
- Dry-run, ASO decision/event journal yazar.
- Risk budget, watchdog status, current blockers, decisions, latest lessons ve evidence export verir.
- Dry-run canlı emir endpoint'i çağırmaz.
- Evidence `artifacts/mainnet-auto/<timestamp>/` altına yazılır.

### 3.9 MAINNET Auto Live

- MAINNET auto live start surface vardır.
- Live start endpoint'i: `POST /api/live/mainnet-auto/start`.
- Script: `scripts/run_mainnet_auto_live_trial.sh`.
- Desteklenen V1 live shape:
  - Symbol: `BTCUSDT`
  - Order type: `MARKET`
  - Runtime: `15`, `60` veya `0/operator-stop`
  - Confirmation:
    - `START MAINNET AUTO LIVE BTCUSDT 15M`
    - `START MAINNET AUTO LIVE BTCUSDT 60M`
    - `START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP`
  - Max notional: `80`
  - Max session loss: `5`
  - Max orders/fills operator batch'te `20`
  - Max leverage explicit budget ile verilir ve hard cap `100x`
  - Flat start ve flat stop gate'leri vardır
  - Evidence ve lesson report gate'leri vardır
  - Margin policy `isolated`, `cross` veya `any`
  - Position policy `crossover_only`, `always_in_market`, `flat_allowed`
- Live session çalışırken closed-candle ASO sinyalleri internal `MainnetAutoLive` policy ile emir submit edebilir.
- Reverse/flat-stop path reduce-only close ve flat reconciliation kullanır.
- Watchdog max runtime, market-data stale, shadow stale, kill switch, max orders/fills/loss gibi nedenlerle stop/flat-stop yapabilir.

### 3.10 ASO Modlari ve MAINNET Auto Position Policy

Burada iki ayrı ayar var; isimler benzer olduğu için karışabiliyor.

ASO indicator mode, ASO'nun bulls/bears değerlerini nasıl hesapladığını belirler. Bu paper, chart, signal ve auto kararlarının beslendiği gösterge ayarıdır:

- `intrabar`: ASO'yu mum içi fiyat aralığı mantığıyla hesaplar.
- `group`: ASO'yu grup/rolling pencere mantığıyla hesaplar.
- `both`: intrabar ve group bilgisini beraber kullanır. Varsayılan ayar budur ve son operator-stop örneklerinde `ASO mode both` olarak geçer.

Bu ayar dashboard settings panelinden değiştirilebilir. API/settings tarafındaki alan adı:

```json
{
  "aso_mode": "both"
}
```

MAINNET auto position policy ise ASO çıktısı geldikten sonra live auto'nun pozisyon isteğini nasıl yorumlayacağını belirler. Env ve live helper flag'i:

```sh
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only
scripts/run_mainnet_auto_live_trial.sh --position-policy crossover_only
```

Desteklenen position policy modları:

- `crossover_only`: konservatif default. Klasik closed-candle crossover sinyali yoksa pozisyon açmaya çalışmaz. Mevcut açık pozisyon/open order varsa yeni entry bloklanır.
- `always_in_market`: son kapalı ASO durumunda bulls > bears ise LONG, bears > bulls ise SHORT ister. Pozisyon yoksa entry açabilir; ters yön isterse önce reduce-only close ile mevcut pozisyonu kapatır, flat reconciliation bekler, sonra karşı yöne entry dener.
- `flat_allowed`: ASO zayıfsa flat/no-trade kalabilir. `RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD` ve `RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD` ile zayıf bölge filtrelenir. Güçlü LONG/SHORT yoksa yeni pozisyon açmayabilir; mevcut pozisyonda zayıf state için otomatik stop-loss/take-profit uydurmaz.

Örnekler:

```sh
# Konservatif default
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only

# Sürekli piyasada kalmaya çalışan mod
RELXEN_MAINNET_AUTO_POSITION_POLICY=always_in_market

# Zayıf ASO durumunda flat kalabilen mod
RELXEN_MAINNET_AUTO_POSITION_POLICY=flat_allowed
RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD=6
RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD=54
```

Canlı helper örnekleri:

```sh
scripts/run_mainnet_auto_live_trial.sh \
  --position-policy always_in_market \
  --confirm "START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP"

scripts/run_mainnet_auto_live_trial.sh \
  --position-policy flat_allowed \
  --aso-delta-threshold 6 \
  --aso-zone-threshold 54 \
  --confirm "START MAINNET AUTO LIVE BTCUSDT 60M"
```

## 4. Ne Yapmiyor?

- MAINNET auto'yu normal startup'ta kendiliğinden açmaz.
- Hidden mainnet bypass yoktur.
- Conditional/algo order yok: stop-loss, take-profit, trailing stop vb.
- Hedge mode desteği yok.
- Multi-assets mode desteği yok.
- Çoklu sembol eşzamanlı runtime yok.
- Multi-user/auth yok.
- Tauri packaging yok.
- Liquidation heatmap veya liquidation-context strategy input'u yok.
- Strategy marketplace veya optimization engine yok.

## 5. Klasor Yapisi

- `crates/domain`: candle, timeframe, symbol, ASO, signal, risk, paper engine, performance.
- `crates/app`: ports, runtime orchestration, bootstrap, live readiness, shadow, execution gates, auto executor.
- `crates/infra`: Binance adapters, SQLite repositories/migrations, secure storage, event bus, metrics.
- `crates/server`: Axum routes, config/env loading, tracing, static frontend serving.
- `web`: React dashboard.
- `scripts`: evidence export, soak, dry-run, mainnet-auto status/live helper scriptleri.
- `docs`: runbook, state, backlog, decisions, live safety/readiness raporları.
- `artifacts`: local evidence output; normalde gitignored.
- `var`: local SQLite/runtime data; normalde gitignored.

## 6. Kurulum

Gerekenler:

- Rust toolchain
- Node.js + npm
- `curl`
- `jq`
- Binance TESTNET/MAINNET credential sadece ilgili live akış denenirse gerekir

İlk kurulum:

```sh
cp .env.example .env
cd web
npm install
npm run build
cd ..
cargo run -p relxen-server
```

Tarayıcı:

```text
http://localhost:3000/
```

## 7. Ortam Degiskenleri

Temel runtime:

```sh
RELXEN_BIND=[::]:3000
RELXEN_DATABASE_URL=sqlite://var/relxen.sqlite3
RELXEN_FRONTEND_DIST=web/dist
RELXEN_LOG_LEVEL=info,relxen=debug
RELXEN_AUTO_START=true
```

Credential source:

```sh
RELXEN_CREDENTIAL_SOURCE=env
BINANCE_TESTNET_API_KEY=...
BINANCE_TESTNET_API_SECRET_KEY=...
BINANCE_MAINNET_API_KEY=...
BINANCE_MAINNET_API_SECRET_KEY=...
```

MAINNET canary:

```sh
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false
```

MAINNET auto:

```sh
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false
RELXEN_MAINNET_AUTO_MODE=dry_run
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=15
RELXEN_MAINNET_AUTO_MAX_ORDERS=1
RELXEN_MAINNET_AUTO_MAX_FILLS=1
RELXEN_MAINNET_AUTO_MAX_NOTIONAL=80
RELXEN_MAINNET_AUTO_MAX_DAILY_LOSS=5
RELXEN_MAINNET_AUTO_REQUIRE_FLAT_START=true
RELXEN_MAINNET_AUTO_REQUIRE_FLAT_STOP=true
RELXEN_MAINNET_AUTO_REQUIRE_MANUAL_CANARY_EVIDENCE=true
RELXEN_MAINNET_AUTO_EVIDENCE_REQUIRED=true
RELXEN_MAINNET_AUTO_LESSON_REPORT_REQUIRED=true
RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=isolated
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only
RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD=5
RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD=55
```

TESTNET drill helper:

```sh
RELXEN_ENABLE_TESTNET_DRILL_HELPERS=false
```

## 8. Build, Test ve Calistirma Komutlari

Integrated server:

```sh
cargo run -p relxen-server
```

Frontend build:

```sh
cd web
npm run build
```

Frontend dev server:

```sh
cd web
npm run dev
```

Release gate:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm test
cd web && npm run build
cargo build --workspace --release
```

Shell/script syntax check örnekleri:

```sh
bash -n scripts/export_live_evidence.sh
bash -n scripts/run_testnet_soak.sh
bash -n scripts/run_mainnet_auto_dry_run.sh
bash -n scripts/show_mainnet_auto_status.sh
bash -n scripts/run_mainnet_auto_live_trial.sh
```

Whitespace check:

```sh
git diff --check
```

## 9. Temel API Komutlari

Health/snapshot:

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl -I http://localhost:3000/
```

Runtime:

```sh
curl -X POST http://localhost:3000/api/runtime/start
curl -X POST http://localhost:3000/api/runtime/stop
```

Settings/trade/signal/log:

```sh
curl http://localhost:3000/api/settings
curl http://localhost:3000/api/trades?limit=100
curl http://localhost:3000/api/signals?limit=100
curl http://localhost:3000/api/logs?limit=100
```

Paper:

```sh
curl -X POST http://localhost:3000/api/paper/close-all
curl -X POST http://localhost:3000/api/paper/reset
```

Live status:

```sh
curl http://localhost:3000/api/live/status
curl http://localhost:3000/api/live/readiness
curl http://localhost:3000/api/live/orders?limit=50
curl http://localhost:3000/api/live/fills?limit=100
curl http://localhost:3000/api/live/preflights?limit=50
```

## 10. Credential API Ornekleri

Credential listele:

```sh
curl http://localhost:3000/api/live/credentials
```

Credential oluştur:

```sh
curl -X POST http://localhost:3000/api/live/credentials \
  -H 'content-type: application/json' \
  -d '{
    "alias": "testnet-local",
    "environment": "testnet",
    "api_key": "REPLACE_ME",
    "api_secret": "REPLACE_ME"
  }'
```

Credential seç:

```sh
curl -X POST http://localhost:3000/api/live/credentials/<credential_id>/select
```

Credential validate et:

```sh
curl -X POST http://localhost:3000/api/live/credentials/<credential_id>/validate
```

Credential update/delete:

```sh
curl -X PUT http://localhost:3000/api/live/credentials/<credential_id> \
  -H 'content-type: application/json' \
  -d '{"alias":"new-alias"}'

curl -X DELETE http://localhost:3000/api/live/credentials/<credential_id>
```

## 11. Live Readiness, Shadow, Preview, Preflight

Readiness refresh:

```sh
curl -X POST http://localhost:3000/api/live/readiness/refresh
```

Arm/disarm:

```sh
curl -X POST http://localhost:3000/api/live/arm
curl -X POST http://localhost:3000/api/live/disarm \
  -H 'content-type: application/json' \
  -d '{"reason":"operator_disarm"}'
```

Start check:

```sh
curl -X POST http://localhost:3000/api/live/start-check
```

Live mode preference:

```sh
curl -X POST http://localhost:3000/api/live/mode \
  -H 'content-type: application/json' \
  -d '{"mode_preference":"paper"}'
```

Shadow:

```sh
curl -X POST http://localhost:3000/api/live/shadow/start
curl -X POST http://localhost:3000/api/live/shadow/refresh
curl -X POST http://localhost:3000/api/live/shadow/stop
```

Preview:

```sh
curl 'http://localhost:3000/api/live/intent/preview?order_type=MARKET'
curl 'http://localhost:3000/api/live/intent/preview?order_type=LIMIT&limit_price=78000'
```

Preflight:

```sh
curl -X POST http://localhost:3000/api/live/preflight
curl http://localhost:3000/api/live/preflights?limit=50
```

## 12. TESTNET Execution API

Execute current preview:

```sh
curl -X POST http://localhost:3000/api/live/execute \
  -H 'content-type: application/json' \
  -d '{
    "intent_id": null,
    "confirm_testnet": true
  }'
```

Cancel order:

```sh
curl -X POST http://localhost:3000/api/live/orders/<order_ref>/cancel \
  -H 'content-type: application/json' \
  -d '{"confirm_testnet":true}'
```

Cancel all active-symbol orders:

```sh
curl -X POST http://localhost:3000/api/live/cancel-all \
  -H 'content-type: application/json' \
  -d '{"confirm_testnet":true}'
```

Flatten active-symbol position:

```sh
curl -X POST http://localhost:3000/api/live/flatten \
  -H 'content-type: application/json' \
  -d '{"confirm_testnet":true}'
```

Start/stop TESTNET auto:

```sh
curl -X POST http://localhost:3000/api/live/auto/start \
  -H 'content-type: application/json' \
  -d '{"confirm_testnet_auto":true}'

curl -X POST http://localhost:3000/api/live/auto/stop
```

TESTNET drill helper:

```sh
curl -X POST http://localhost:3000/api/live/drill/auto/replay-latest-signal \
  -H 'content-type: application/json' \
  -d '{"confirm_testnet_drill":true}'
```

## 13. Kill Switch ve Risk Profile

Kill switch:

```sh
curl -X POST http://localhost:3000/api/live/kill-switch/engage \
  -H 'content-type: application/json' \
  -d '{"reason":"operator_engaged"}'

curl -X POST http://localhost:3000/api/live/kill-switch/release \
  -H 'content-type: application/json' \
  -d '{"reason":"operator_released"}'
```

Manual MAINNET canary için risk profile örneği:

```sh
curl -X PUT http://localhost:3000/api/live/risk-profile \
  -H 'content-type: application/json' \
  -d '{
    "configured": true,
    "profile_name": "conservative-mainnet-canary",
    "limits": {
      "max_notional_per_order": "80",
      "max_open_notional_active_symbol": "80",
      "max_leverage": "5",
      "max_orders_per_session": 1,
      "max_fills_per_session": 1,
      "max_consecutive_rejections": 1,
      "max_daily_realized_loss": "5"
    },
    "updated_at": 0
  }'
```

## 14. MAINNET Canary API

Canary server flag ile başlat:

```sh
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true cargo run -p relxen-server
```

Sonra tipik sıra:

```sh
curl http://localhost:3000/api/live/status
curl -X POST http://localhost:3000/api/live/credentials/<mainnet_credential_id>/select
curl -X POST http://localhost:3000/api/live/credentials/<mainnet_credential_id>/validate
curl -X POST http://localhost:3000/api/live/readiness/refresh
curl -X POST http://localhost:3000/api/live/arm
curl -X POST http://localhost:3000/api/live/shadow/start
curl 'http://localhost:3000/api/live/intent/preview?order_type=LIMIT&limit_price=<non_marketable_price>'
```

Execute payload shape:

```sh
curl -X POST http://localhost:3000/api/live/execute \
  -H 'content-type: application/json' \
  -d '{
    "intent_id": null,
    "confirm_testnet": false,
    "confirm_mainnet_canary": true,
    "confirmation_text": "<UI veya /api/live/status icindeki exact confirmation>"
  }'
```

MAINNET canary cancel/flatten payload shape:

```sh
curl -X POST http://localhost:3000/api/live/orders/<order_ref>/cancel \
  -H 'content-type: application/json' \
  -d '{
    "confirm_testnet": false,
    "confirm_mainnet_canary": true,
    "confirmation_text": "<exact confirmation>"
  }'

curl -X POST http://localhost:3000/api/live/flatten \
  -H 'content-type: application/json' \
  -d '{
    "confirm_testnet": false,
    "confirm_mainnet_canary": true,
    "confirmation_text": "<exact confirmation>"
  }'
```

## 15. MAINNET Auto Dry-Run

Status:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --precheck
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --summary
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --flat-check
```

Heartbeat:

```sh
RELXEN_BASE_URL=http://localhost:3000 RELXEN_HEARTBEAT_SECONDS=5 scripts/show_mainnet_auto_status.sh --heartbeat
```

Start dry-run + export evidence:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_mainnet_auto_dry_run.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_mainnet_auto_evidence.sh
```

Raw API:

```sh
curl http://localhost:3000/api/live/mainnet-auto/status
curl -X POST http://localhost:3000/api/live/mainnet-auto/dry-run/start
curl -X POST http://localhost:3000/api/live/mainnet-auto/dry-run/stop
curl http://localhost:3000/api/live/mainnet-auto/decisions?limit=100
curl http://localhost:3000/api/live/mainnet-auto/lessons/latest
curl -X POST http://localhost:3000/api/live/mainnet-auto/export-evidence
```

## 16. MAINNET Auto Live

Server env örneği, 15 dakika live:

```sh
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true \
RELXEN_MAINNET_AUTO_MODE=live \
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=15 \
RELXEN_MAINNET_AUTO_MAX_ORDERS=20 \
RELXEN_MAINNET_AUTO_MAX_FILLS=20 \
RELXEN_MAINNET_AUTO_MAX_NOTIONAL=80 \
RELXEN_MAINNET_AUTO_MAX_DAILY_LOSS=5 \
RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=isolated \
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only \
cargo run -p relxen-server
```

15 dakika live start:

```sh
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true \
RELXEN_MAINNET_AUTO_MODE=live \
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=15 \
RELXEN_BASE_URL=http://localhost:3000 \
scripts/run_mainnet_auto_live_trial.sh \
  --symbol BTCUSDT \
  --duration-minutes 15 \
  --max-leverage 5 \
  --max-notional 80 \
  --max-session-loss-usdt 5 \
  --max-orders 20 \
  --max-fills 20 \
  --order-type MARKET \
  --allowed-margin-type isolated \
  --position-policy crossover_only \
  --confirm "START MAINNET AUTO LIVE BTCUSDT 15M"
```

60 dakika live start:

```sh
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true \
RELXEN_MAINNET_AUTO_MODE=live \
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=60 \
RELXEN_BASE_URL=http://localhost:3000 \
scripts/run_mainnet_auto_live_trial.sh \
  --symbol BTCUSDT \
  --duration-minutes 60 \
  --max-leverage 20 \
  --max-notional 80 \
  --max-session-loss-usdt 5 \
  --max-orders 20 \
  --max-fills 20 \
  --order-type MARKET \
  --allowed-margin-type isolated \
  --position-policy flat_allowed \
  --aso-delta-threshold 6 \
  --aso-zone-threshold 54 \
  --confirm "START MAINNET AUTO LIVE BTCUSDT 60M"
```

Operator-stop live start:

```sh
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true \
RELXEN_MAINNET_AUTO_MODE=live \
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=0 \
RELXEN_BASE_URL=http://localhost:3000 \
scripts/run_mainnet_auto_live_trial.sh \
  --symbol BTCUSDT \
  --duration-minutes operator-stop \
  --max-leverage 20 \
  --max-notional 80 \
  --max-session-loss-usdt 5 \
  --max-orders 20 \
  --max-fills 20 \
  --order-type MARKET \
  --allowed-margin-type isolated \
  --position-policy always_in_market \
  --confirm "START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP"
```

Raw risk budget API shape:

```sh
curl -X PUT http://localhost:3000/api/live/mainnet-auto/risk-budget \
  -H 'content-type: application/json' \
  -d '{
    "configured": true,
    "budget_id": "mainnet-auto-live-15m-v1",
    "max_notional_per_order": "80",
    "max_total_session_notional": "80",
    "max_open_notional": "80",
    "max_orders_per_session": 20,
    "max_fills_per_session": 20,
    "max_consecutive_losses": 1,
    "max_consecutive_rejections": 1,
    "max_daily_realized_loss": "5",
    "max_position_age_seconds": 900,
    "max_runtime_minutes": 15,
    "max_leverage": "5",
    "require_flat_start": true,
    "require_flat_stop": true,
    "allowed_symbols": ["BTCUSDT"],
    "allowed_order_types": ["MARKET"],
    "require_fresh_reference_price": true,
    "require_fresh_shadow": true,
    "require_fresh_user_data_stream": true,
    "require_evidence_logging": true,
    "require_lessons_report": true,
    "updated_at": 0
  }'
```

Raw start API shape:

```sh
curl -X POST http://localhost:3000/api/live/mainnet-auto/start \
  -H 'content-type: application/json' \
  -d '{
    "symbol": "BTCUSDT",
    "duration_minutes": 15,
    "order_type": "MARKET",
    "confirmation_text": "START MAINNET AUTO LIVE BTCUSDT 15M",
    "allowed_margin_type": "isolated",
    "position_policy": "crossover_only",
    "aso_delta_threshold": "5",
    "aso_zone_threshold": "55"
  }'
```

Stop:

```sh
curl -X POST http://localhost:3000/api/live/mainnet-auto/stop
```

## 17. Evidence Komutlari

TESTNET/live evidence export:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_live_evidence.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_testnet_soak.sh
```

MAINNET auto evidence export:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_mainnet_auto_evidence.sh
```

Evidence klasörleri:

- `artifacts/testnet-soak/<timestamp>/`
- `artifacts/mainnet-canary/<timestamp>/`
- `artifacts/mainnet-auto/<timestamp>/`

Bu klasörler local evidence çıktısıdır ve normal repo paylaşımında ignored kalır.

## 18. Son Kayitli Evidence Ornekleri

- Real TESTNET soak: `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`
- Env credential validation: `artifacts/testnet-soak/20260424T061338Z-env-credential-validation/`
- MAINNET canary 1: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`
- MAINNET canary 2: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`
- MAINNET auto dry-run: `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`
- MAINNET auto no-order 15m run: `artifacts/mainnet-auto/1777099647957-mnauto_live_39b61e12f8084f669b334420a3f105ac/`
- MAINNET auto degraded always-in-market run: `artifacts/mainnet-auto/1777104375086-mnauto_live_0518464591cd473fbdac1e34675c1cae/`
- MAINNET auto flat_allowed run: `artifacts/mainnet-auto/1777112199366-mnauto_live_00388618d8df47b8aaa97269e2128cb8/`
- Latest operator-stop run: `artifacts/mainnet-auto/1777121228224-mnauto_live_10150facce4b478d8d47a063ea58fdc7/`

## 19. Runtime State Terimleri

- `CONNECTED`: market-data stream sağlıklı.
- `RECONNECTING`: stream yeniden açılıyor.
- `STALE`: data stale veya deterministic recovery bekliyor.
- `RESYNCED`: REST repair sonrası stream devam ediyor.
- `DISCONNECTED`: runtime kapalı veya bağlantı yok.
- `ready_read_only`: credential/rules/account read-only gate'leri geçmiş.
- `armed_read_only`: operatör live mode'u arm etmiş.
- `shadow_running`: user-data shadow sync çalışıyor.
- `shadow_degraded`: shadow ambiguous/stale/degraded.
- `preflight_ready`: preview local gate'lerden geçmiş ve preflight denenebilir.
- `testnet_execution_ready`: TESTNET execute yapılabilir.
- `testnet_auto_running`: TESTNET auto açık.
- `kill_switch_engaged`: yeni live submission bloklu.
- `mainnet_canary_ready`: manual MAINNET canary gate'leri hazır.
- `mainnet_manual_execution_enabled`: exact confirmation ile manual MAINNET canary execute mümkün.
- `dry_run_running`: MAINNET auto dry-run açık.
- `live_running`: MAINNET auto live session açık.
- `watchdog_stopped`: MAINNET auto watchdog/operator reason ile durmuş.
- `execution_degraded`: submission/stream/repair state ambiguous.

## 20. Okuma Sirasi

1. `docs/FRIEND_HANDOFF.md`
2. `README.md`
3. `docs/RUNBOOK.md`
4. `docs/PROJECT_STATE.md`
5. `docs/BACKLOG.md`
6. `docs/DECISIONS.md`
7. `docs/ARCHITECTURE.md`
8. `docs/OPERATOR_HANDOFF.md`
9. `docs/LATEST_TESTNET_SOAK_REPORT.md`
10. `docs/LATEST_MAINNET_CANARY_REPORT.md`
11. `docs/MAINNET_AUTO_RUNBOOK.md`
12. `docs/MAINNET_AUTO_LIVE_TRIAL_PLAN.md`

## 21. Paylasirken Dikkat Edilecek Dosyalar

Paylaşılabilir kaynak/doküman:

- `README.md`
- `docs/*.md`
- `crates/**`
- `web/**`
- `scripts/**`
- `.env.example`
- `Cargo.toml`, `Cargo.lock`, `web/package.json`, `web/package-lock.json`

Paylaşılmaması gereken local/operator verisi:

- `.env`
- `var/`
- `target/`
- `web/dist/`
- `node_modules/`
- `artifacts/` içindeki raw local evidence, ayrıca özel olarak seçilip secret-scan yapılmadıysa

## 22. Yeni Alan Kisi Icin Kisa Akis

```sh
cp .env.example .env
cd web
npm install
npm run build
cd ..
cargo run -p relxen-server
```

Sonra:

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl http://localhost:3000/api/live/status
```

Dashboard:

```text
http://localhost:3000/
```

Buradan sonra kişi amacına göre paper, TESTNET, manual MAINNET canary veya MAINNET auto live akışlarından birini seçer. Hangi akışın hangi flag, endpoint ve confirmation istediği yukarıdaki bölümlerde yazılıdır.
