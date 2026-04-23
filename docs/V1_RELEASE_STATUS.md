# Paper Mode V1 Release Status

## Current Status

Paper Mode V1 is release-candidate complete.

In practical terms, the repository is ready to run locally as a single-user Binance Futures paper-trading dashboard. It can bootstrap market history, compute ASO, generate closed-candle signals, execute paper trades, persist state in SQLite, recover from bounded WebSocket interruptions, and serve the built frontend from the Axum backend.

Paper Mode V1 itself remains a paper-trading release candidate. Post-v1 live work now includes constrained TESTNET-only `MARKET` / `LIMIT` order placement, cancel, cancel-all-active-symbol, and flatten. MAINNET execution is not implemented and remains blocked.

## Included In V1

- Binance Futures market data via REST klines and WebSocket klines.
- Explicit ranged historical loading using aligned `startTime` / `endTime` windows.
- Supported symbols: `BTCUSDT` and `BTCUSDC`.
- One active symbol at a time.
- One open paper position at a time.
- ASO indicator with `intrabar`, `group`, and `both` modes.
- Closed-candle-only BUY/SELL signal generation.
- Paper engine with quote-separated wallets, fees, sizing, margin caps, reverse, close-all, reset, and mark-to-market.
- SQLite persistence for settings, klines, signals, trades, paper wallets, paper positions, and logs.
- Axum REST API, WebSocket API, and static frontend serving.
- React dashboard with chart, ASO, position, wallet, performance, connection, system, risk, trade history, and log panels.
- Reconnect recovery using bounded REST reconciliation and deterministic `resync_required`.
- Operator feedback for history sync, rebuilding, stale/reconnect age, command success, and command failure.

## Excluded From V1

- Real live order placement.
- Binance signed order submission.
- Live exchange position mutation.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace.
- Optimization engine.

## Runtime Scope

The runtime is intentionally narrow:

- Single local operator.
- Single active symbol.
- Single timeframe at a time.
- Single open paper position.
- Paper execution is the only execution mode.
- Live execution remains locked/blocked; post-v1 read-only LIVE ACCESS does not place orders.

Post-v1 note: the repository now includes a LIVE ACCESS panel for live foundations, live shadow sync, intent preview, testnet preflight, and constrained TESTNET-only manual execution. It does not change Paper Mode V1 paper-execution scope.

## Evidence

The release gate was verified with:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd web && npm test`
- `cd web && npm run build`
- `cargo build --workspace --release`
- local release-server smoke check for `/api/health`, `/api/bootstrap`, and static `/`

Coverage includes domain ASO/signal/paper-engine tests, app-layer bootstrap/recovery/trade tests, fixture-backed Binance REST adapter tests, real SQLite restart/rebuild tests, real Axum HTTP/WebSocket tests, and browser-style frontend operator-flow tests.

## Acceptable Paper-Mode Limitations

- Paper calculations currently use `f64`; this is acceptable for local simulation but not final live-trading truth.
- Paper wallet and position state are local simulation state, not exchange-authoritative account truth.
- Public Binance market-data availability is assumed for normal runtime.
- The app is local single-user software and has no authentication layer.
- The dashboard is operational, not a full broker-grade audit console.

## Definition Of Done For Paper Mode V1

- The app runs locally end-to-end from a clean checkout.
- Backend can serve the built frontend.
- Bootstrap returns a complete typed snapshot.
- ASO and signals are generated only from closed candles.
- Paper trades are persisted and recover across restart.
- Settings/symbol/timeframe rebuilds are deterministic or fail safely.
- Reconnect recovery cannot silently skip closed-candle transitions in covered scenarios.
- UI state is textually explicit and does not depend on color alone.
- Live trading remains disabled and unimplemented.

## Why This Phase Is Complete

The current repository satisfies the requested paper-trading product boundaries with production-minded defaults: typed models, layered architecture, bounded history planning, SQLite persistence, restart/rebuild tests, real HTTP/WebSocket tests, static frontend serving, and operator-facing failure states.

Further work should not expand Paper Mode V1 unless a bug is found. The next phase is design-controlled live-readiness work.

## Before Any Broader Live Trading Code Is Written

- Read `docs/LIVE_READINESS.md`.
- Freeze credential storage rules in implementation tasks.
- Choose and implement a stricter precision strategy.
- Preserve exchange symbol-rule validation and decimal intent handling.
- Preserve account snapshot and shadow reconciliation foundations.
- Preserve explicit operator arming and fail-closed risk gates.
- Prove kill-switch behavior, strategy-driven testnet auto-execution, and exchange-authoritative fill reconciliation before mainnet.
