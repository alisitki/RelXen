# Project State

## Current Phase

Constrained Testnet Executor v1 complete.

## Current Status

Paper Mode V1 remains release-candidate complete. The repository is runnable end-to-end as a local single-user Binance Futures paper-trading dashboard with deterministic ranged history loading, SQLite persistence, paper execution, restart-safe persistence, WebSocket recovery, and a React dashboard served statically by the backend.

Post-v1 live work now includes a constrained TESTNET-only execution slice. RelXen can store masked credential metadata with raw secrets behind the OS secure-storage abstraction, validate Binance USDⓈ-M Futures credentials, refresh read-only account snapshots and symbol rules, run listenKey/user-data shadow sync, build decimal/rules-aware `MARKET` / `LIMIT` order intents, run testnet `order/test` preflight, submit explicit operator-confirmed TESTNET orders, cancel RelXen-created TESTNET orders, cancel all active-symbol TESTNET orders with explicit confirmation, and flatten an active-symbol TESTNET position when shadow state is coherent.

MAINNET execution remains blocked and not implemented. Conditional/algo orders, hedge mode, multi-assets mode, autonomous strategy-driven live execution, Tauri packaging, auth, multi-user support, and multi-symbol concurrent runtime remain out of scope.

## Completed In This Phase

- Added domain/app models for execution states, order records, fill records, execution requests/results, cancel results, flatten results, and execution availability.
- Added TESTNET-only Binance `POST /fapi/v1/order`, `DELETE /fapi/v1/order`, order query, open-order query, and user-trade fallback adapter methods.
- Added fail-closed execution gating for validated credentials, testnet environment, arming, fresh shadow stream, coherent shadow/account mode, supported symbol/timeframe, fresh rules/account snapshots, non-stale preview hash, and no active order ambiguity.
- Added local submission records and exchange-reconciled order/fill records without inferring fills from local assumptions.
- Added user-data `ORDER_TRADE_UPDATE` reconciliation into persisted live orders/fills and websocket events.
- Added TESTNET cancel, cancel-all-active-symbol, and manual flatten flow with explicit confirmations.
- Added REST endpoints and frontend controls for execute, cancel, cancel-all, flatten, live orders, and live fills.
- Added SQLite persistence for live orders, live fills, and execution state cache.
- Added mocked adapter/app/server/frontend tests for testnet execution, cancel, mainnet block, fill reconciliation, and operator execution UX.

## Current Focus

The project is in a constrained testnet-executor state. Paper mode remains intact. Live work must continue in small fail-closed slices and must not enable mainnet until testnet evidence, reconciliation behavior, and operator safety controls are proven.

## declared_next_task

Add an explicit kill switch and strategy-driven closed-candle TESTNET auto-executor behind arming, duplicate-signal suppression, and exchange-authoritative fill reconciliation.

## done_when

- Operator can enable or disable testnet auto-execution separately from manual testnet execution.
- Only closed-candle signals can create live testnet intents, and duplicate signal/order submission is prevented by persisted intent/order keys.
- Kill switch blocks every new testnet submission immediately while preserving safe cancel/flatten behavior where deterministic.
- User-data/REST reconciliation remains authoritative for order/fill/position state after every auto-submitted order.
- Tests cover armed, disarmed, duplicate, stale shadow, kill-switch, rejected, partial-fill, and reconnect repair paths without enabling mainnet.

## Not Now

- Mainnet live order execution.
- Conditional/algo orders.
- Hedge mode or multi-assets mode support.
- Plaintext secret persistence.
- Using paper engine state as live account truth.
- Tauri packaging.
- Auth or multi-user support.
- Multi-symbol concurrent runtime.

## Known Blockers

- None for the declared next task.

## Key References

- [V1 Release Status](./V1_RELEASE_STATUS.md)
- [Live Readiness](./LIVE_READINESS.md)
- [Live Execution Boundary](./LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](./SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](./PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](./LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](./LIVE_IMPLEMENTATION_PLAN.md)
