# Backlog

## Completed V1 Items

- Clean-room Rust workspace with domain/app/infra/server layering.
- React + Vite dashboard served statically by the backend.
- Supported symbols limited to `BTCUSDT` and `BTCUSDC`.
- Single active symbol and one open paper position at a time.
- Binance Futures REST/WebSocket market-data ingestion.
- Explicit ranged history loading for bootstrap, rebuild, runtime start, and reconnect recovery.
- ASO indicator, closed-candle signal generation, and paper engine.
- SQLite persistence for settings, klines, signals, trades, wallets, positions, and logs.
- Runtime WebSocket deltas, bootstrap snapshots, and deterministic `resync_required`.
- Realtime paper trade history events.
- Operator status UX for connection age, stale state, rebuild/history sync, and command feedback.
- Fixture-backed Binance adapter tests, real SQLite restart/rebuild tests, and server/frontend failure UX tests.
- Paper Mode V1 release-status and runbook docs.

## Completed Live-Foundation Items

- OS secure-storage abstraction with normal runtime backend and in-memory test backend.
- Env-backed local credential source with masked source metadata, authoritative TESTNET env startup selection, compatibility-alias fallback behavior, and explicit MAINNET selection requirement.
- Masked live credential metadata CRUD with active credential selection.
- SQLite live metadata persistence without raw secret storage.
- Binance USDⓈ-M Futures signed read-only credential validation.
- Read-only account snapshot and active-symbol rules retrieval for `BTCUSDT` / `BTCUSDC`.
- Live readiness, blocking reasons, warnings, arming/disarming, and start-gating.
- Live status bootstrap payload, REST APIs, websocket update events, and frontend LIVE ACCESS panel.

## Completed Live-Shadow/Preflight Items

- Binance USDⓈ-M listenKey create/keepalive/close lifecycle.
- User-data stream parsing for account, order-trade, account-config, expiration, and unknown events.
- Read-only shadow account, position, open-order, stream, stale, degraded, and ambiguity state.
- REST shadow refresh and fail-closed degraded state handling.
- Decimal-based live order-intent preview for `BTCUSDT` / `BTCUSDC` and `MARKET` / `LIMIT`.
- Exchange-rule checks for tick size, step size, min qty, min notional, symbol status, and unsupported account modes.
- Testnet-only `order/test` preflight with persisted results and explicit no-order-placed messaging.

## Completed Constrained Testnet Executor Items

- TESTNET `MARKET` / `LIMIT` order submission through Binance USDⓈ-M new-order endpoint.
- TESTNET order cancel, cancel-all-active-symbol, and manual flatten.
- Local live order/fill persistence and execution state cache.
- User-data `ORDER_TRADE_UPDATE` reconciliation into live orders and fills.
- REST fallback methods for order query, open orders, and recent user trades.
- REST APIs, websocket events, and frontend controls for execute/cancel/cancel-all/flatten/orders/fills.

## Completed Mainnet-Readiness Hardening Items

- Kill switch with API/bootstrap/websocket/frontend visibility.
- TESTNET closed-candle auto-executor with explicit start/stop controls.
- Persisted duplicate signal/intent suppression for auto-execution.
- ACK-only real submission handling with exchange-authoritative reconciliation.
- Dedicated Binance position-mode and multi-assets-mode checks.
- Forced user-data reconnect and REST repair before the 24-hour stream limit.
- Recent-window-only execution repair policy due to Binance query retention limits.
- Operator-configured risk profile required before MAINNET canary readiness.
- Manual MAINNET canary execution path behind `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` by default, exact confirmation text, arming, risk profile, fresh shadow/rules/account state, and all normal gates.
- MAINNET auto-execution remains blocked.

## Completed Soak/Evidence Items

- Real TESTNET credential creation, selection, and validation through the secure-store flow.
- Real TESTNET readiness/shadow bootstrap, preview, preflight, manual execution, cancel, flatten, kill switch, restart repair, reconnect repair, and auto duplicate-suppression evidence.
- Real evidence bundle generated under `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`.
- Evidence export now includes masked credential summaries to make credential blockers auditable without exposing secrets.
- Evidence export includes credential source metadata and can write the required masked MAINNET canary before/after snapshots.
- Bounded TESTNET soak runbook covering credential readiness, shadow sync, preview, preflight, real TESTNET execution, cancel, flatten, kill switch, restart repair, reconnect repair, auto-executor proof, and recent-window repair limits.
- Secret-safe evidence export script using existing read-only API endpoints.
- Guided operator soak wrapper with checkpoint capture and no built-in order placement.
- Mainnet canary checklist with explicit hard preconditions and no-go conditions.
- Latest soak report updated with real exchange evidence, targeted fixes, env validation evidence, and the current MAINNET NO-GO recommendation.
- TESTNET-only, default-off drill helper for replaying the latest persisted closed signal through the existing auto executor when no natural signal appears during a bounded soak window.
- Manual shadow refresh now performs bounded recent-window execution repair.
- Recent-window repaired fills now backfill local order references when authoritative exchange trade data can be matched to a repaired order.
- Git ignore policy for generated soak artifacts under `artifacts/testnet-soak/`.
- Env-backed TESTNET and MAINNET credential validation without raw-secret persistence or secure-store prompts.
- MAINNET canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T070419Z-balance-blocked/`.
- MAINNET leverage-gated canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T073409Z-leverage-gated/`.
- MAINNET balance-funded canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T083256Z-balance-funded/`.
- MAINNET leverage-fixed canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T084721Z-leverage-fixed/`.
- MAINNET leverage-fixed canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T085756Z-leverage-fixed/` after reference price became unavailable post kill-switch drill.
- Reference-price freshness hardening with explicit internal-market/REST mark-price resolver, preview diagnostics, and final submit refresh enforcement.
- Successful guarded MAINNET canary evidence bundle under `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`: one `BTCUSDT` non-marketable `LIMIT` order submitted, canceled, reconciled flat, restart-repair checked, and canary flag disabled afterward.
- Post-canary audit of `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`: no raw secrets found in scanned repository/evidence surfaces, no MAINNET fill, no open BTCUSDT position, no active canary flag after safe-default restart, and canary order/fill evidence scoped to the single MAINNET canary while preserving original all-recent exports separately.
- Second MAINNET canary readiness dry-run under `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`: no real order submitted, `env-mainnet` validated, mainnet shadow refreshed, previous canary remained canceled/no-fill, kill switch engage/release passed, MAINNET canary flag stayed disabled, MAINNET auto stayed stopped, and a fresh non-marketable `BUY LIMIT BTCUSDT 0.001 @ 77800` preview passed with required margin `15.56` against available `USDT=25.0902305`.
- Second bounded MAINNET canary execution under `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`: one real `BUY LIMIT BTCUSDT 0.001 @ 77800` order submitted with ACK, canceled, reconciled with `executed_qty=0.000`, no fill, flat final position, restart repair passed, and mainnet canary disabled afterward. The run exposed a cancel payload ergonomics issue: the first cancel request omitted body `order_ref` and was rejected before a retry canceled the same order.
- Cancel endpoint body ergonomics fixed: `POST /api/live/orders/:order_ref/cancel` now uses the path `order_ref` as authoritative, accepts omitted or matching optional body `order_ref`, rejects mismatched body `order_ref`, and preserves confirmation gates.
- Post-fix safe-default smoke confirmed health/bootstrap/live-status/live-credentials/static frontend, MAINNET canary disabled, MAINNET auto stopped, previous MAINNET BTCUSDT orders still canceled, no MAINNET BTCUSDT fills, flat BTCUSDT account snapshot position, and no raw env secrets in smoke API payloads.
- Mainnet canary closure / operator handoff completed: evidence bundles were reviewed, final live state was smoke-checked with MAINNET canary disabled and no unexpected exposure, final status docs were aligned, and `docs/OPERATOR_HANDOFF.md` was created.
- Release-candidate cleanup completed: git/worktree hygiene, ignored artifact policy, generated-output policy, final safe-smoke status, and known residual risks were captured in `docs/FINAL_RC_SNAPSHOT.md`.
- Shareable RC UI cleanup completed: top safety summary, clearer LIVE ACCESS grouping, friendlier safety language, and frontend regression coverage for MAINNET auto blocked / canary disabled / preflight-not-execution / canceled-not-filled states.
- Shadow/reconciliation environment mismatch now blocks readiness, preview, and execution gates.
- MAINNET auto infrastructure v1 added default-off typed config gates, persisted risk budget/state/decision/watchdog/lesson metadata, dry-run start/stop/status APIs, live-start fail-closed blocking, headless helper scripts, evidence export, and lesson reports. No TESTNET or MAINNET order is submitted by the dry-run path.
- Credential-selected MAINNET auto dry-run completed on the operator DB under `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`: `env-mainnet` was selected/validated, mainnet readiness/shadow refreshed, one dry-run decision recorded `dry_run_would_submit`, live-start remained `config_blocked`, and no live order/fill was submitted.
- Mainnet Auto Live Support v1 implemented the gated live path: typed `BTCUSDT` / 15-minute / `MARKET` start request, exact session confirmation, server config gates, live-running session state, closed-candle ASO signal handling through mocked adapter tests, one-position/one-in-flight enforcement, runtime watchdog, live evidence/lesson support, and headless live-trial/status scripts.
- Mainnet Auto Live Support verification gate rerun completed without starting a real MAINNET auto session, submitting TESTNET/MAINNET orders, or calling real cancel/flatten. Mocked live-ASO tests now prove existing MAINNET shadow open-position and open-order state produces separate `open_position` / `open_order` blockers before any submit attempt.
- Operator-start command preparation completed for the 15-minute live trial: the live helper now accepts the explicit batch flags, checks the running server is already in session-scoped live-auto mode, configures the bounded risk budget, and calls the existing start endpoint. No live session was started and no order was submitted during this prep.
- Mainnet-auto persisted-state startup compatibility fixed: legacy operator DB `mainnet_auto_state` singleton JSON without the newer `watchdog` field now decodes with defaults, and a session-scoped live-auto server smoke reached `/api/health` without starting the auto session.
- Mainnet-auto status helper jq compatibility fixed: `show_mainnet_auto_status.sh --precheck` now uses parenthesized jq expressions and null/default guards so jq 1.7 does not fail before rendering read-only status; recent BTCUSDT fills are filtered to MAINNET for live precheck output.
- First 15-minute MAINNET auto live trial completed on 2026-04-25 under `artifacts/mainnet-auto/1777099647957-mnauto_live_39b61e12f8084f669b334420a3f105ac/`: session `mnauto_live_39b61e12f8084f669b334420a3f105ac`, `BTCUSDT`, `MARKET`, max leverage `5`, notional cap `80`, max session loss `5`, watchdog stop `max_runtime_reached`, zero signals, zero decisions, zero submitted orders, zero fills, realized PnL `0`, fees `0`, final open orders `0`, and final BTCUSDT position flat. Mainnet auto remains default-off outside explicit session-scoped runs.
- Mainnet-auto evidence export now scopes live `orders.json` / `fills.json` to the active live session and writes final verdict fields for stop reason, signals/decisions, order/fill counts, PnL/fees, final open orders, final position, and flat-stop outcome.
- Mainnet Auto Policy Support v1 implemented explicit margin-type policy and ASO position policy modes without submitting any order. Margin type is modeled as `cross`, `isolated`, or `unknown`; `RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE` defaults to `isolated`; unknown or non-allowed actual margin type blocks live MAINNET auto; status, heartbeat, evidence, and lessons expose margin policy. `RELXEN_MAINNET_AUTO_POSITION_POLICY` supports `crossover_only`, `always_in_market`, and `flat_allowed`; default remains `crossover_only`. Headless live-trial flags now carry policy/margin settings and mocked tests cover margin gates, policy decisions, blockers, evidence files, and default-off safety.
- Second `always_in_market` MAINNET auto live trial completed degraded under `artifacts/mainnet-auto/1777104375086-mnauto_live_0518464591cd473fbdac1e34675c1cae/`: one real `BUY MARKET BTCUSDT 0.001` order submitted and filled at average price `77493.50000`; subsequent desired SHORT evaluations were blocked as `reversal_unsupported`; watchdog stopped at max runtime; no open orders remained; final position stayed LONG `0.001`; flat-stop failed; kill switch was engaged afterward. A focused Binance parser fix now derives isolated/cross margin type from the USD-M `isolated` field when `marginType` is absent.
- Manual post-run cleanup flattened the open MAINNET position through the existing canary-gated reduce-only path under `artifacts/mainnet-canary/20260425T081553Z-mainnet-auto-manual-flatten/`: one `SELL MARKET BTCUSDT 0.001` reduce-only order filled at average price `77513.60000`; final BTCUSDT position amount is `0`; open BTCUSDT orders are `0`.
- MAINNET auto reverse/flat-stop hardening implemented without submitting a live order: `always_in_market` reverse now submits a reduce-only close, requires flat reconciliation, then submits the opposite entry; watchdog/operator stop uses the same auto-owned reduce-only close path for coherent flat-stop. `crossover_only` preserves its conservative open-position blocker. Mocked-adapter tests cover LONG-to-SHORT reverse, coherent flat-stop, 100x leverage budget acceptance, and >100x rejection.
- Operator-stopped 60-minute `flat_allowed` MAINNET auto live run completed flat under `artifacts/mainnet-auto/1777112199366-mnauto_live_00388618d8df47b8aaa97269e2128cb8/`: one `BUY MARKET BTCUSDT 0.001` filled at average price `77697.30000`, one reduce-only `SELL MARKET BTCUSDT 0.001` flat-stop filled at average price `77732.50000`, final BTCUSDT position `0`, open orders `0`, realized PnL `0.03520000`, and fees `0.07771490`. The run exposed that the market-data runtime can remain stuck at `opening Binance kline stream`, so it was stopped and not trusted as an active closed-candle feed.
- MAINNET auto evidence/runtime hardening now records `stopped_at` after flat-stop reconciliation, performs bounded live repair before evidence export, includes auto-owned stop/reverse settlement records in a 30-second export grace window, and times out market-data stream subscribe/first-event waits after 15 seconds.
- Disabled-live-auto BTCUSDT market-data smoke completed without any order submission: REST history loaded, the first WebSocket attempts timed out visibly, reconnect/REST gap recovery later reached `connected`, and the latest closed candle plus stream message became fresh. Live-auto policy entry now waits for fresh market data, the live helper waits up to 120 seconds for a healthy BTCUSDT runtime before start, closed-candle freshness is timeframe-aware for `5m` and other intervals, and the MAINNET auto watchdog stops/flats with `market_data_stale` if a live session loses fresh market data.
- Operator-stop MAINNET auto runtime support is implemented without enabling live auto by default: `duration_minutes=0` / `RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=0` requires exact confirmation `START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP`, matching server config/risk/start request, and leaves `expires_at` unset while preserving kill switch, max loss, order/fill caps, flat-stop, margin policy, market-data freshness, shadow/reconciliation, evidence, and lesson gates.
- Operator-approved `5m` / ASO length `10` / ASO mode `both` / `always_in_market` / `20x isolated` operator-stop MAINNET auto attempt completed with safety intervention: session `mnauto_live_556d2a40030147b5ba015212f776193d` submitted one `SELL MARKET BTCUSDT 0.001` entry filled at average price `77626.30000`, then watchdog-stopped with `market_data_stale`. REST repair confirmed a SHORT `0.001` position after the stopped session, so a manual canary-gated reduce-only `BUY MARKET BTCUSDT 0.001` flatten filled at average price `77626.40000`. Final BTCUSDT position is `0` and open orders are `0`. Evidence: `artifacts/mainnet-auto/1777116752373-mnauto_live_556d2a40030147b5ba015212f776193d/` and `artifacts/mainnet-canary/20260425T113335Z-operator-stop-manual-flatten/`.
- Follow-up `5m` live-auto repair/reconciliation hardening completed without a live run or order submission. Before stale closed-candle stop, MAINNET auto now attempts a bounded REST repair of up to three missing closed candles; if repair cannot prove a complete contiguous gap, it still fails closed. Before auto-owned flat-stop/reverse close, MAINNET auto refreshes shadow and runs recent-window order repair so a just-ACKed `MARKET` fill is not misclassified as an unexpected open order. Mocked tests cover just-ACKed fill repair before flat-stop plus the existing stale-watchdog and `5m` freshness paths.
- Post-hardening safe-default smoke completed with MAINNET auto disabled and canary disabled. Health/bootstrap/live-status/mainnet-auto-status responded, MAINNET auto reported `disabled` / `dry_run` with `mainnet_auto_config_disabled`, BTCUSDT open orders were `0`, no order was submitted, and the smoke server was stopped.
- Operator-approved `5m` / ASO length `10` / ASO mode `both` / `always_in_market` / `20x isolated` operator-stop run completed flat under `artifacts/mainnet-auto/1777121228224-mnauto_live_10150facce4b478d8d47a063ea58fdc7/`: one `SELL MARKET BTCUSDT 0.001` entry filled at average price `77639.9`, six closed-candle hold decisions while ASO bears stayed above bulls, watchdog stop `shadow_stale`, one reduce-only `BUY MARKET BTCUSDT 0.001` flat-stop filled at average price `77517.2`, final BTCUSDT position `0`, open orders `0`, realized PnL `0.12270000`, and fees `0.07757854`.
- The latest `shadow_stale` stop is root-caused and hardened without a live run or order submission. Shadow freshness now uses the newest user-data event, REST sync, and shadow update timestamp instead of only user-data `last_event_time`; live start/signal/watchdog paths attempt read-only shadow repair before stale-running shadow can stop a session; order-trade events clear the stale flag; and transient market-data `reconnecting` is allowed inside the normal stream-message freshness window instead of immediately stopping an exposed session. Truly stale/down market data and failed shadow repair still fail closed.

## Immediate Next Task

Keep MAINNET auto idle; complete the verification gate and disabled-live-auto smoke for the shadow/market-data false-positive stop hardening before another open-ended operator-approved live run.

## Deferred Live Execution Work

- Broader mainnet enablement policy after the bounded manual canary evidence.
- Repeat or broader MAINNET auto live execution. Support exists and one bounded 15-minute run completed with no orders/fills, and policy modes now exist, but live mode remains config-gated, operator-armed, watchdog-protected, default-off, and session-scoped only.
- Conditional/algo orders such as STOP, TAKE_PROFIT, and trailing orders.
- Hedge mode and multi-assets mode support if explicitly designed and tested.
- Portfolio-level exposure controls beyond the active symbol.
- Broker-grade audit/export reporting.
- Automated incident drill reporting and operator attestations beyond the current soak evidence bundle.
- Liquidation heatmap/liquidation-context module. It needs separate source-quality and semantics design, and it must not become a live decision layer until mainnet safety hardening is reviewed separately.

## Not-Now Items

- Broad or always-on MAINNET auto-execution outside explicit bounded sessions.
- Liquidation heatmap/liquidation-context panels, APIs, or strategy inputs.
- Plaintext secret storage.
- Treating paper-engine state as exchange-authoritative truth.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace.
- Optimization engine.
