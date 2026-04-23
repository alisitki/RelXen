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
- Fixture-backed Binance adapter tests.
- Real SQLite restart/rebuild tests.
- Real HTTP/WebSocket server-boundary tests.
- Browser-style frontend tests for critical operator flows.
- Paper Mode V1 release-status and runbook docs.

## Completed Live-Foundation Items

- OS secure-storage abstraction with normal runtime backend and in-memory test backend.
- Masked live credential metadata CRUD with active credential selection.
- SQLite live metadata persistence without raw secret storage.
- Binance USDⓈ-M Futures signed read-only credential validation.
- Read-only account snapshot and active-symbol rules retrieval for `BTCUSDT` / `BTCUSDC`.
- Live readiness, blocking reasons, warnings, arming/disarming, and start-gating.
- Live status bootstrap payload, REST APIs, websocket update event, and frontend LIVE ACCESS panel.

## Completed Live-Shadow/Preflight Items

- Binance USDⓈ-M listenKey create/keepalive/close lifecycle.
- User-data stream parsing for account, order-trade, account-config, expiration, and unknown events.
- Read-only shadow account, position, open-order, stream, stale, degraded, and ambiguity state.
- REST shadow refresh and fail-closed degraded state handling.
- Decimal-based live order-intent preview for `BTCUSDT` / `BTCUSDC` and `MARKET` / `LIMIT`.
- Exchange-rule checks for tick size, step size, min qty, min notional, symbol status, and unsupported account modes.
- Testnet-only `order/test` preflight with persisted results and explicit no-order-placed messaging.

## Completed Constrained Testnet Executor Items

- TESTNET-only `MARKET` / `LIMIT` order submission through Binance USDⓈ-M new-order endpoint.
- TESTNET-only order cancel flow for RelXen-created orders.
- TESTNET-only cancel-all-active-symbol flow with backend confirmation requirement.
- TESTNET-only manual flatten flow that cancels open active-symbol orders and submits a reduce-only MARKET close when safe.
- Mainnet execution hard block with explicit operator-facing state.
- Local live order/fill persistence and execution state cache.
- User-data `ORDER_TRADE_UPDATE` reconciliation into live orders and fills.
- REST fallback methods for order query, open orders, and user trades.
- REST APIs, websocket events, and frontend controls for execute/cancel/cancel-all/flatten/orders/fills.
- Mocked infra/app/server/frontend tests for testnet execution, cancel, mainnet block, fill reconciliation, and operator UX.

## Immediate Next Task

Add an explicit kill switch and strategy-driven closed-candle TESTNET auto-executor behind arming, duplicate-signal suppression, and exchange-authoritative fill reconciliation.

## Deferred Live Execution Work

- Constrained mainnet execution slice only after testnet auto-execution and kill-switch evidence are green.
- Conditional/algo orders such as STOP, TAKE_PROFIT, and trailing orders.
- Hedge mode and multi-assets mode support if explicitly designed and tested.
- Portfolio-level exposure controls beyond the active symbol.
- Broker-grade audit/export reporting.

## Not-Now Items

- Mainnet trading controls before testnet executor evidence and kill-switch drills.
- Plaintext secret storage.
- Treating paper-engine state as exchange-authoritative truth.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace.
- Optimization engine.
