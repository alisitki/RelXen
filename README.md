# RelXen

RelXen is a clean-room ASO-based Binance Futures trading dashboard. It is a local single-user app with a Rust backend, SQLite persistence, masked live credential metadata, live shadow/preflight foundations, a constrained Binance USDⓈ-M Futures executor, and a lightweight React dashboard served statically by the backend.

Paper Mode V1 is release-candidate complete. Post-v1 live capabilities now include credential metadata, OS secure storage, optional local `.env` credential loading, Binance read-only validation, account snapshots, symbol rules, user-data-stream shadow reconciliation, precision-aware order intents, testnet `order/test` preflight validation, constrained TESTNET `MARKET` / `LIMIT` placement/cancel/flatten, closed-candle TESTNET auto-execution, kill switch controls, and manual MAINNET canary execution behind an explicit server-side canary gate. A real TESTNET soak run was completed on 2026-04-23. On 2026-04-24, reference-price freshness was hardened and one guarded MAINNET `BTCUSDT` non-marketable `LIMIT` canary submitted, canceled, reconciled, and restart-repair checked under `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`. A later second-canary readiness dry-run built a fresh non-marketable preview without submitting an order under `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`, then a second bounded MAINNET `BTCUSDT` canary submitted and canceled under `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`. The follow-up cancel endpoint ergonomics fix makes the route path `order_ref` sufficient for `POST /api/live/orders/:order_ref/cancel` while preserving exact confirmation gates. MAINNET auto infrastructure now exists for default-off status, dry-run decisions, risk budget, watchdog state, evidence export, and lesson reports. The first credential-selected operator-DB dry-run is `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`; it recorded a would-submit dry-run decision and submitted no order. Live MAINNET auto execution remains disabled by default.

The RC dashboard has been cleaned up for operator/friend review with a top safety summary and clearer LIVE ACCESS sections. This UI pass did not add trading behavior or submit any order.

Mainnet Auto Live Support v1 is now implemented as a gated code path only. `POST /api/live/mainnet-auto/start` requires server live config, `RELXEN_MAINNET_AUTO_MODE=live`, `BTCUSDT`, `MARKET`, a 15-minute duration, the exact session confirmation `START MAINNET AUTO LIVE BTCUSDT 15M`, fresh live gates, risk budget, watchdog, and evidence/lesson logging. The operator helper accepts the explicit live-trial flags and verifies the running server is already session-scoped for live auto before calling the existing start endpoint. No real MAINNET auto session has been run by Codex, and MAINNET auto remains disabled by default.

## V1 Scope

- Paper trading remains supported and isolated from live exchange truth.
- TESTNET live execution is implemented for explicit operator-submitted `MARKET` / `LIMIT` orders, cancel, cancel-all-active-symbol, flatten, and opt-in closed-candle auto-execution.
- MAINNET manual canary execution uses the same ACK-plus-authoritative-reconciliation path, but it is disabled by default and requires `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true`, explicitly selected and validated mainnet credentials, a configured risk profile, fresh shadow/rules/account state, arming, a non-marketable `LIMIT` preview, and exact operator confirmation.
- Supported symbols are `BTCUSDT` and `BTCUSDC`.
- Exactly one active symbol and one open position are supported at a time.
- Market data comes from Binance Futures REST klines and WebSocket klines.
- Historical loading uses explicit `startTime` / `endTime` ranged REST requests.
- Signals are generated only from closed candles using the Average Sentiment Oscillator.
- Runtime recovery reconciles bounded REST ranges after reconnects and emits `resync_required` when deterministic continuity cannot be proven.
- Live credentials use an OS secure-storage abstraction by default; local `.env` credentials can be enabled for operator convenience. SQLite stores masked metadata and source only.
- Live readiness can validate credentials, fetch read-only account snapshots and symbol rules, check dedicated position-mode and multi-assets-mode endpoints, arm live mode, start/stop shadow sync, build `MARKET` / `LIMIT` intent previews, run testnet preflight checks, submit confirmed TESTNET or gated MAINNET canary orders, cancel confirmed orders, flatten an active-symbol position when reconciliation is coherent, engage/release the kill switch, start/stop TESTNET auto-execution, and prepare a default-off MAINNET auto live session path for a future explicit run.

## Workspace Layout

- `crates/domain`: pure candle, ASO, signal, risk, paper-engine, and performance logic.
- `crates/app`: ports, history planning, bootstrap/rebuild flow, runtime orchestration, live readiness, testnet/mainnet-canary execution gating, kill switch, auto-executor, and application services.
- `crates/infra`: Binance adapters, SQLite repositories/migrations, secure storage, execution adapter, event bus, and system metrics.
- `crates/server`: Axum REST API, WebSocket endpoint, env config, tracing setup, and static serving.
- `web`: React + Vite + TypeScript dashboard using Zustand, TanStack Query, and `lightweight-charts`.

## Where To Start Reading

- Current release status: [docs/V1_RELEASE_STATUS.md](docs/V1_RELEASE_STATUS.md)
- Local runbook: [docs/RUNBOOK.md](docs/RUNBOOK.md)
- Testnet soak drill: [docs/TESTNET_SOAK_RUNBOOK.md](docs/TESTNET_SOAK_RUNBOOK.md)
- Latest soak report: [docs/LATEST_TESTNET_SOAK_REPORT.md](docs/LATEST_TESTNET_SOAK_REPORT.md)
- Latest mainnet canary report: [docs/LATEST_MAINNET_CANARY_REPORT.md](docs/LATEST_MAINNET_CANARY_REPORT.md)
- Operator handoff: [docs/OPERATOR_HANDOFF.md](docs/OPERATOR_HANDOFF.md)
- Final RC snapshot: [docs/FINAL_RC_SNAPSHOT.md](docs/FINAL_RC_SNAPSHOT.md)
- Architecture overview: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Current project memory: [docs/PROJECT_STATE.md](docs/PROJECT_STATE.md)
- Post-v1 live readiness entrypoint: [docs/LIVE_READINESS.md](docs/LIVE_READINESS.md)
- Mainnet auto dry-run runbook: [docs/MAINNET_AUTO_RUNBOOK.md](docs/MAINNET_AUTO_RUNBOOK.md)
- Mainnet auto lessons guide: [docs/MAINNET_AUTO_LESSONS_GUIDE.md](docs/MAINNET_AUTO_LESSONS_GUIDE.md)

## Live Readiness Docs

- [Operator Handoff](docs/OPERATOR_HANDOFF.md)
- [Final RC Snapshot](docs/FINAL_RC_SNAPSHOT.md)
- [Live Execution Boundary](docs/LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](docs/SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](docs/PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](docs/LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](docs/LIVE_IMPLEMENTATION_PLAN.md)
- [Mainnet Canary Checklist](docs/MAINNET_CANARY_CHECKLIST.md)
- [Latest Mainnet Canary Report](docs/LATEST_MAINNET_CANARY_REPORT.md)

## Live Access Status

The dashboard includes a `LIVE ACCESS` panel. It supports masked credential metadata CRUD, active credential selection, Binance USDⓈ-M Futures read-only validation, readiness refresh, arm/disarm controls, live shadow stream controls, kill switch controls, risk-profile configuration, TESTNET auto start/stop, intent preview, testnet preflight, explicit TESTNET order execution, gated MAINNET canary execution, cancel, cancel-all-active-symbol, and flatten. Supported execution symbols remain `BTCUSDT` and `BTCUSDC`; supported actual order types are `MARKET` and `LIMIT`.

Raw API secrets are never returned by HTTP APIs and are not stored in SQLite. Normal production-minded runtime uses OS secure storage through the infra secret-store adapter; tests use in-memory stores. Local operators may opt into `.env` credentials with `RELXEN_CREDENTIAL_SOURCE=env`; in that authoritative mode the TESTNET env credential is selected at startup ahead of any persisted secure-store TESTNET selection, while MAINNET env credentials still require explicit selection. `.env` is gitignored, placeholder-only values belong in `.env.example`, and OS secure storage remains the preferred production-grade secret path. If the configured secret source is unavailable or incomplete, paper mode remains usable and live readiness fails closed.

Preflight success is reported as `PREFLIGHT PASSED`, never as an order placement. Actual placement requires a separate execute action, explicit confirmation, validated credentials, fresh shadow state, fresh rules/account snapshots, dedicated one-way and single-asset-mode checks, supported symbol/timeframe, a configured risk profile for MAINNET canary, and a non-stale preview. MAINNET canary previews must be non-marketable `LIMIT` orders after tick-size rounding; `MARKET` is blocked for the first canary. Real submissions request Binance `ACK` and rely on user-data stream plus bounded recent-window REST repair for final order/fill/account truth.

## Testnet Soak Evidence

Operational testnet evidence is captured with read-only export scripts that call the existing API surface:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_live_evidence.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_testnet_soak.sh
```

Artifacts are written under `artifacts/testnet-soak/<timestamp>/` and are ignored by git. The scripts export masked credential summaries but never raw secrets. They do not create credentials, arm execution, or place orders; the operator performs those actions through the existing UI/API gates. The latest recorded status is in [docs/LATEST_TESTNET_SOAK_REPORT.md](docs/LATEST_TESTNET_SOAK_REPORT.md), and the current real evidence bundle is `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`. If no valid TESTNET credential is available, the real exchange drill must still be marked not exercised rather than faked.

## Quick Start

```sh
cp .env.example .env
cd web
npm install
npm run build
cd ..
cargo run -p relxen-server
```

Then open `http://localhost:3000/`. The backend also exposes:

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl http://localhost:3000/api/live/status
curl http://localhost:3000/api/live/orders
curl http://localhost:3000/api/live/fills
```

The production/local run mode serves the built frontend from `RELXEN_FRONTEND_DIST` at `/`. The Vite dev server is optional for frontend-only iteration, but the intended integrated run mode is the Axum backend serving `web/dist`.

## Environment

Defaults are documented in `.env.example`.

- `RELXEN_BIND`: bind address, default `[::]:3000`.
- `RELXEN_DATABASE_URL`: SQLite URL, default `sqlite://var/relxen.sqlite3`.
- `RELXEN_FRONTEND_DIST`: built frontend directory, default `web/dist`.
- `RELXEN_LOG_LEVEL`: tracing filter, default `info,relxen=debug`.
- `RELXEN_AUTO_START`: start the market stream after bootstrap, default `true`.
- `RELXEN_CREDENTIAL_SOURCE`: set to `env` to load local operator credentials from `.env`. This setting is authoritative.
- `RELXEN_ENABLE_ENV_CREDENTIALS`: compatibility alias; `true` enables env credentials only when `RELXEN_CREDENTIAL_SOURCE` is unset.
- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION`: enables manual MAINNET canary submission path when every other gate passes, default `false`.
- `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION`: enables live MAINNET auto only when a later explicit live-run batch also arms and starts it with exact confirmation; default `false`.
- `RELXEN_MAINNET_AUTO_MODE`: mainnet-auto run mode, default `dry_run`. Dry-run records decisions and evidence but never submits orders.
- `RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES`, `RELXEN_MAINNET_AUTO_MAX_ORDERS`, `RELXEN_MAINNET_AUTO_MAX_FILLS`, `RELXEN_MAINNET_AUTO_MAX_NOTIONAL`, `RELXEN_MAINNET_AUTO_MAX_DAILY_LOSS`: typed mainnet-auto session budget inputs. The planned first live trial uses 15 minutes, emergency order/fill caps of 20, notional cap 80, and loss cap 5.
- `RELXEN_MAINNET_AUTO_REQUIRE_FLAT_START`, `RELXEN_MAINNET_AUTO_REQUIRE_FLAT_STOP`, `RELXEN_MAINNET_AUTO_REQUIRE_MANUAL_CANARY_EVIDENCE`, `RELXEN_MAINNET_AUTO_EVIDENCE_REQUIRED`, `RELXEN_MAINNET_AUTO_LESSON_REPORT_REQUIRED`: fail-closed mainnet-auto safety requirements.
- `RELXEN_ENABLE_TESTNET_DRILL_HELPERS`: enables explicit TESTNET-only drill helpers for bounded soak validation, default `false`.
- `BINANCE_TESTNET_API_KEY`, `BINANCE_TESTNET_API_SECRET_KEY`, `BINANCE_MAINNET_API_KEY`, `BINANCE_MAINNET_API_SECRET_KEY`: optional env credential values when env source is enabled. `.env.example` contains placeholders only; never commit `.env`.

SQLite migrations run automatically when the repository connects. The app enables WAL mode, `synchronous = normal`, and a busy timeout.

The live credential path stores only metadata in SQLite. Raw secure-store secrets live behind the OS secure-storage adapter, and raw env secrets are read from process environment only when explicitly enabled. Neither path echoes raw secrets to the frontend.

Live shadow state and preflight results are cached in SQLite as operator-visible snapshots. They are not exchange-authoritative execution records and do not imply an order was placed.

Live order and fill records are persisted separately and reconciled from ACK submissions, user-data stream events, and bounded recent-window REST repair. Raw secrets are never persisted in SQLite.

## Build And Test

Release gate commands:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm test
cd web && npm run build
cargo build --workspace --release
```

## Runtime States

- `CONNECTED`: market-data stream is healthy.
- `RECONNECTING <age>`: the backend is reopening the stream.
- `STALE <age>`: the stream is stale or a deterministic resync is pending.
- `RESYNCED`: bounded REST recovery completed and normal deltas are resuming.
- `DISCONNECTED`: runtime is stopped or no stream is active.
- `HISTORY SYNC` / `REBUILDING`: bootstrap or settings-triggered history work is active.
- `testnet_execution_ready`: all local gates pass for the displayed TESTNET preview.
- `testnet_auto_running`: closed-candle TESTNET auto-execution is explicitly running and duplicate signal intents are suppressed.
- `kill_switch_engaged`: new live submissions are blocked immediately.
- `mainnet_canary_ready`: the server canary flag, risk profile, arming, preview, shadow, rules, account, and confirmation gates can allow a manual MAINNET canary action.
- `mainnet_manual_execution_enabled`: exact operator confirmation for the current MAINNET preview is available; MAINNET auto remains unavailable.
- `dry_run_running`: MAINNET auto dry-run is recording decision/evidence events without submitting orders.
- `watchdog_stopped`: MAINNET auto has stopped with a persisted watchdog/operator reason.
- `testnet_submit_pending`: a TESTNET order was submitted and final lifecycle is waiting on exchange reconciliation.
- `testnet_order_open`: the exchange reports a working TESTNET order.
- `testnet_partially_filled` / `testnet_filled`: fills were recorded from authoritative exchange updates.
- `testnet_cancel_pending`: cancel was requested and final lifecycle is waiting on exchange reconciliation.
- `execution_degraded`: submission, stream, or repair state is ambiguous; new submissions fail closed.
- `mainnet_execution_blocked`: MAINNET execution is disabled by server canary policy or another fail-closed gate.

Critical UI meaning is also represented textually, for example `▲ LONG`, `▼ SHORT`, `■ FLAT`, `CONNECTED`, and `DISCONNECTED`.

## Deferred Work

- Broad MAINNET enablement beyond the explicit manual canary gate.
- Conditional/algo orders such as STOP, TAKE_PROFIT, and trailing orders.
- Live MAINNET auto-execution by default. The gated support path exists, but no real live-auto session has been run and a separate explicit execution task is still required.
- Broader incident automation beyond the documented soak evidence workflow.
- Liquidation heatmap/liquidation-context module; ASO remains the active strategy signal and no new live decision layer is added in the post-canary safety-hardening flow.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace and optimization tooling.
