# RelXen

RelXen is a clean-room ASO-based Binance Futures trading dashboard. It is a local single-user app with a Rust backend, SQLite persistence, OS secure-storage-backed live credential metadata, live shadow/preflight foundations, a constrained Binance USDⓈ-M Futures testnet executor, and a lightweight React dashboard served statically by the backend.

Paper Mode V1 is release-candidate complete. Post-v1 live foundations now include credential metadata, OS secure storage, Binance read-only validation, account snapshots, symbol rules, user-data-stream shadow reconciliation, precision-aware order intents, testnet `order/test` preflight validation, and constrained TESTNET-only `MARKET` / `LIMIT` placement, cancel, and flatten flows. MAINNET execution remains blocked and not implemented.

## V1 Scope

- Paper trading remains supported and isolated from live exchange truth.
- TESTNET-only live execution is implemented for explicit operator-submitted `MARKET` / `LIMIT` orders, cancel, cancel-all-active-symbol, and flatten. MAINNET execution is explicitly blocked.
- Supported symbols are `BTCUSDT` and `BTCUSDC`.
- Exactly one active symbol and one open position are supported at a time.
- Market data comes from Binance Futures REST klines and WebSocket klines.
- Historical loading uses explicit `startTime` / `endTime` ranged REST requests.
- Signals are generated only from closed candles using the Average Sentiment Oscillator.
- Runtime recovery reconciles bounded REST ranges after reconnects and emits `resync_required` when deterministic continuity cannot be proven.
- Live credentials use an OS secure-storage abstraction; SQLite stores masked metadata only.
- Live readiness can validate credentials, fetch read-only account snapshots and symbol rules, arm live mode, start/stop shadow sync, build `MARKET` / `LIMIT` intent previews, run testnet preflight checks, submit confirmed TESTNET orders, cancel confirmed TESTNET orders, and flatten a TESTNET active-symbol position when reconciliation is coherent.

## Workspace Layout

- `crates/domain`: pure candle, ASO, signal, risk, paper-engine, and performance logic.
- `crates/app`: ports, history planning, bootstrap/rebuild flow, runtime orchestration, live readiness, testnet execution gating, and application services.
- `crates/infra`: Binance adapters, SQLite repositories/migrations, secure storage, testnet execution adapter, event bus, and system metrics.
- `crates/server`: Axum REST API, WebSocket endpoint, env config, tracing setup, and static serving.
- `web`: React + Vite + TypeScript dashboard using Zustand, TanStack Query, and `lightweight-charts`.

## Where To Start Reading

- Current release status: [docs/V1_RELEASE_STATUS.md](docs/V1_RELEASE_STATUS.md)
- Local runbook: [docs/RUNBOOK.md](docs/RUNBOOK.md)
- Architecture overview: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Current project memory: [docs/PROJECT_STATE.md](docs/PROJECT_STATE.md)
- Post-v1 live readiness entrypoint: [docs/LIVE_READINESS.md](docs/LIVE_READINESS.md)

## Live Readiness Docs

- [Live Execution Boundary](docs/LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](docs/SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](docs/PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](docs/LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](docs/LIVE_IMPLEMENTATION_PLAN.md)

## Live Access Status

The dashboard includes a `LIVE ACCESS` panel. It supports masked credential metadata CRUD, active credential selection, Binance USDⓈ-M Futures read-only validation, readiness refresh, arm/disarm controls, live shadow stream controls, intent preview, testnet preflight, explicit TESTNET order execution, TESTNET cancel, TESTNET cancel-all-active-symbol, and TESTNET flatten. Supported execution symbols remain `BTCUSDT` and `BTCUSDC`; supported actual order types are `MARKET` and `LIMIT`.

Raw API secrets are never returned by HTTP APIs and are not stored in SQLite. Normal runtime uses OS secure storage through the infra secret-store adapter; tests use in-memory stores. If the OS secure store is unavailable, paper mode remains usable and live readiness fails closed.

Preflight success is reported as `PREFLIGHT PASSED`, never as an order placement. Actual placement requires the separate `Execute TESTNET Preview` action, explicit confirmation, validated testnet credentials, fresh shadow state, fresh rules/account snapshots, one-way mode, supported symbol/timeframe, and a non-stale preview. Mainnet execution has no bypass in this build.

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

SQLite migrations run automatically when the repository connects. The app enables WAL mode, `synchronous = normal`, and a busy timeout.

The live credential path stores only metadata in SQLite. Raw live secrets are stored behind the OS secure-storage adapter for normal runtime and are never echoed to the frontend.

Live shadow state and preflight results are cached in SQLite as operator-visible snapshots. They are not exchange-authoritative execution records and do not imply an order was placed.

TESTNET live order and fill records are persisted separately and are reconciled from exchange responses, REST repair, and user-data stream events. Raw secrets are never persisted in SQLite.

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
- `testnet_submit_pending`: a TESTNET order was submitted and final lifecycle is waiting on exchange reconciliation.
- `testnet_order_open`: the exchange reports a working TESTNET order.
- `testnet_partially_filled` / `testnet_filled`: fills were recorded from authoritative exchange updates.
- `testnet_cancel_pending`: cancel was requested and final lifecycle is waiting on exchange reconciliation.
- `execution_degraded`: submission, stream, or repair state is ambiguous; new submissions fail closed.
- `mainnet_execution_blocked`: MAINNET order placement and cancel remain unavailable in this repository state.

Critical UI meaning is also represented textually, for example `▲ LONG`, `▼ SHORT`, `■ FLAT`, `CONNECTED`, and `DISCONNECTED`.

## Deferred Work

- Mainnet matching-engine order placement and cancel.
- Conditional/algo orders such as STOP, TAKE_PROFIT, and trailing orders.
- Autonomous strategy-driven testnet/live execution.
- Full kill-switch incident workflow beyond fail-closed blocking and manual testnet flatten.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace and optimization tooling.
