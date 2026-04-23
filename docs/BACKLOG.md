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
- Bounded TESTNET soak runbook covering credential readiness, shadow sync, preview, preflight, real TESTNET execution, cancel, flatten, kill switch, restart repair, reconnect repair, auto-executor proof, and recent-window repair limits.
- Secret-safe evidence export script using existing read-only API endpoints.
- Guided operator soak wrapper with checkpoint capture and no built-in order placement.
- Mainnet canary checklist with explicit hard preconditions and no-go conditions.
- Latest soak report updated with real exchange evidence, targeted fixes, and a conditional-go recommendation.
- TESTNET-only, default-off drill helper for replaying the latest persisted closed signal through the existing auto executor when no natural signal appears during a bounded soak window.
- Manual shadow refresh now performs bounded recent-window execution repair.
- Recent-window repaired fills now backfill local order references when authoritative exchange trade data can be matched to a repaired order.
- Git ignore policy for generated soak artifacts under `artifacts/testnet-soak/`.

## Immediate Next Task

Run one single-order manual MAINNET canary session with the existing gates and capture a matching evidence bundle plus report update.

## Deferred Live Execution Work

- Broader mainnet enablement policy after the bounded manual canary evidence.
- Conditional/algo orders such as STOP, TAKE_PROFIT, and trailing orders.
- Hedge mode and multi-assets mode support if explicitly designed and tested.
- Portfolio-level exposure controls beyond the active symbol.
- Broker-grade audit/export reporting.
- Automated incident drill reporting and operator attestations beyond the current soak evidence bundle.

## Not-Now Items

- MAINNET auto-execution.
- Plaintext secret storage.
- Treating paper-engine state as exchange-authoritative truth.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace.
- Optimization engine.
