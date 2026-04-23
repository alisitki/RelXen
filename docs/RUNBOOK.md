# Runbook

## First Run

1. Install Rust and Node.js.
2. Copy `.env.example` to `.env`.
3. Install and build the dashboard:

```sh
cd web
npm install
npm run build
cd ..
```

4. Start the integrated backend/frontend server:

```sh
cargo run -p relxen-server
```

5. Open `http://localhost:3000/`.

## Environment Variables

- `RELXEN_BIND`: backend bind address. Default is `[::]:3000`.
- `RELXEN_DATABASE_URL`: SQLite URL. Default is `sqlite://var/relxen.sqlite3`.
- `RELXEN_FRONTEND_DIST`: built frontend directory. Default is `web/dist`.
- `RELXEN_LOG_LEVEL`: tracing filter. Default is `info,relxen=debug`.
- `RELXEN_AUTO_START`: whether bootstrap should start the WebSocket runtime. Default is `true`.

## Database And Migrations

The backend creates the SQLite parent directory if needed and runs SQLx migrations on connect. Runtime persistence uses real SQLite tables for settings, klines, signals, trades, paper wallets, paper positions, logs, live credential metadata, shadow snapshots, preflight results, TESTNET live orders, TESTNET live fills, and execution state cache.

Live shadow snapshots and preflight results are cached for operator visibility. Preflight never means an order was placed. TESTNET live order/fill records are separate execution records and must be interpreted through exchange reconciliation status.

Raw live API secrets are not stored in SQLite. Normal runtime stores secret material through the OS secure-storage adapter; tests use in-memory secret stores.

SQLite is configured for WAL mode, `synchronous = normal`, and a busy timeout. To reset local paper state safely, use the UI `Reset Paper` action or:

```sh
curl -X POST http://localhost:3000/api/paper/reset
```

Deleting the SQLite file is only recommended when you intentionally want a clean local instance.

## Health Checks

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl -I http://localhost:3000/
```

`/api/bootstrap` returns the complete typed snapshot used by the dashboard: metadata, runtime status, settings, candles, ASO points, recent signals/trades/logs, current position, wallets, performance, connection state, live status, and system metrics.

Live-foundation checks:

```sh
curl http://localhost:3000/api/live/status
curl http://localhost:3000/api/live/credentials
curl -X POST http://localhost:3000/api/live/readiness/refresh
curl -X POST http://localhost:3000/api/live/start-check
curl -X POST http://localhost:3000/api/live/shadow/refresh
curl http://localhost:3000/api/live/intent/preview
curl http://localhost:3000/api/live/preflights
curl http://localhost:3000/api/live/orders
curl http://localhost:3000/api/live/fills
```

## Runtime States

- `CONNECTED`: WebSocket deltas are flowing normally.
- `RECONNECTING <age>`: the runtime is reopening the Binance stream.
- `STALE <age>`: data is stale or deterministic recovery is not yet complete.
- `RESYNCED`: bounded REST recovery completed and market-data deltas are resuming.
- `DISCONNECTED`: runtime is stopped or disconnected.
- `HISTORY SYNC`: bootstrap/history loading is active.
- `REBUILDING`: settings apply triggered a deterministic series rebuild.

If `resync_required` is emitted, the frontend reloads `/api/bootstrap` and rebuilds chart data with `setData()`. Normal steady-state candle/ASO updates remain incremental.

## LIVE ACCESS Flow

The LIVE ACCESS panel supports paper-mode operation, read-only shadow/preflight work, and constrained TESTNET-only manual execution. MAINNET execution remains blocked.

1. Create a credential with alias, environment (`testnet` or `mainnet`), API key, and API secret.
2. RelXen stores raw secret material in OS secure storage and stores only masked metadata in SQLite.
3. Select the active credential.
4. Run `Validate` to perform a signed read-only Binance USDⓈ-M Futures account check.
5. Run `Refresh Readiness` to fetch symbol rules and a read-only account snapshot for the active symbol.
6. If all gates pass, the state becomes `ready_read_only` and arming is enabled.
7. Use `Start Shadow Sync` to open a Binance USDⓈ-M user-data listenKey stream and maintain a read-only shadow view.
8. Use `Build Preview` to compute a precision-aware `MARKET` or `LIMIT` order intent from current settings, rules, shadow account state, and the latest closed signal when available.
9. Use `Run Preflight` to submit the serialized payload to Binance testnet `order/test`. This validates the signed payload but does not place an order.
10. Use `Execute TESTNET Preview` only when the UI shows `TESTNET EXECUTION READY`. Confirm the browser prompt. This sends a real TESTNET matching-engine order, not a mainnet order.
11. Use `Cancel Open TESTNET Order` or `Cancel All Active-Symbol Orders` to cancel RelXen-created TESTNET open orders. The backend also requires explicit testnet confirmation.
12. Use `Flatten TESTNET Position` only when shadow state is coherent. RelXen cancels active-symbol open orders first, then submits a reduce-only MARKET close intent when safe.

If OS secure storage is unavailable, the UI/API report `secure_store_unavailable` and paper mode remains usable. Never put live API secrets in `.env`, SQLite, frontend storage, logs, or screenshots.

## Live Readiness States

- `credentials_missing`: no active live credential is selected.
- `secure_store_unavailable`: the OS secure-storage backend could not be used.
- `validation_failed`: credential validation is missing, stale, invalid, or failed.
- `rules_unavailable`: active symbol rules are missing.
- `account_snapshot_unavailable`: read-only account snapshot is missing.
- `ready_read_only`: credentials, rules, account snapshot, and local gates are sufficient for read-only readiness.
- `armed_read_only`: the operator explicitly armed live mode; testnet execution still requires the execution gates below.
- `shadow_starting`: listenKey creation, REST shadow bootstrap, or stream attachment is in progress.
- `shadow_running`: user-data shadow sync is running and the shadow account snapshot is coherent enough for preflight work.
- `shadow_degraded`: the user-data stream or REST fallback became ambiguous or stale; live readiness fails closed.
- `preflight_ready`: an order-intent preview passed local precision/rules checks and may be validated through testnet `order/test`.
- `preflight_blocked`: a preview or preflight is blocked locally, for example due to stale shadow state, unsupported mode, mainnet preflight, or precision/rule validation failure.
- `testnet_execution_ready`: a confirmed TESTNET order may be submitted for the displayed preview.
- `testnet_submit_pending`: RelXen submitted a TESTNET order and is waiting for authoritative exchange reconciliation.
- `testnet_order_open`: the exchange reports a working TESTNET order.
- `testnet_partially_filled`: the exchange reports partial fills.
- `testnet_filled`: the exchange reports the TESTNET order as filled.
- `testnet_cancel_pending`: RelXen submitted a cancel and is waiting for authoritative exchange reconciliation.
- `execution_degraded`: submission, stream, or repair state is ambiguous; new submissions fail closed.
- `mainnet_execution_blocked`: MAINNET order placement and cancel are intentionally unavailable.
- `start_blocked`: a live start was requested or checked, but current gates block the operation.

To switch back safely, click `Disarm` or set the execution preference to `PAPER MODE`. This does not affect paper wallets or paper positions.

## Live Shadow And Preflight Controls

- `Start Shadow Sync`: creates a listenKey, bootstraps shadow account state through REST, and attaches a user-data WebSocket stream.
- `Stop Shadow Sync`: closes the listenKey when possible and marks the shadow stream stopped.
- `Refresh Shadow`: refreshes shadow account state from read-only REST and clears stale shadow blockers only when the snapshot is coherent.
- `Build Preview`: builds an inspectable intent. Quantity and LIMIT price are rounded with live decimal logic and exchange symbol rules.
- `Run Preflight`: sends the preview payload to Binance testnet `POST /fapi/v1/order/test`. Mainnet preflight is blocked in this repository state.
- `Execute TESTNET Preview`: submits the displayed preview to Binance testnet `POST /fapi/v1/order` after explicit confirmation and fail-closed gates.
- `Cancel Open TESTNET Order`: sends `DELETE /fapi/v1/order` for a RelXen-created TESTNET order.
- `Cancel All Active-Symbol Orders`: cancels RelXen-created open TESTNET orders for the active symbol only.
- `Flatten TESTNET Position`: cancels open active-symbol TESTNET orders and submits a reduce-only MARKET close when shadow position state is deterministic.

If the stream expires, disconnects, or cannot be reconciled, the UI should show `SHADOW DEGRADED`, `execution_degraded`, `shadow_stream_down`, or `shadow_state_ambiguous`. Stop and restart shadow sync after checking credentials/connectivity. Do not interpret preflight success as exchange position truth.

After an ambiguous submission or reconnect, do not retry manually until `/api/live/status` shows a coherent order state or a degraded/blocked state with a clear repair outcome. RelXen queries order/open-order/user-trade fallback endpoints when needed and blocks new submissions if status remains unknown.

## Common Failure Notes

- History/rebuild failures mean contiguous closed-candle coverage could not be proven. The backend surfaces a typed history error and keeps the last valid visible state rather than silently running on ambiguous data.
- Settings apply can be temporarily disabled while `HISTORY SYNC` or `REBUILDING` is active.
- Paper commands can be temporarily disabled during history work. Command failures are normalized into operator-facing toasts.
- `RELXEN_AUTO_START=false` is useful for a backend smoke run that should bootstrap data but not open the market-data WebSocket stream.

## Paper-Mode Boundaries

RelXen v1 paper mode remains independent. Post-v1 RelXen can place/cancel/flatten TESTNET orders only through explicit operator actions and fail-closed gates. It does not place or cancel MAINNET orders, support conditional/algo orders, package Tauri, support auth, or run multiple symbols concurrently. Live credentials use OS secure storage for raw secrets and SQLite metadata only.
