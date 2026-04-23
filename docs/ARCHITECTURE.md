# Architecture

## Goals

RelXen is structured as a layered Rust workspace with a statically served web dashboard. The first vertical slice prioritizes runtime discipline, deterministic paper trading, and a clean path toward future live execution without mixing transport code into the strategy layer.

## Layers

### `crates/domain`

Pure domain logic only:

- candle/timeframe/symbol models
- ASO computation
- signal crossover rules
- paper trading engine
- risk and sizing checks
- performance aggregation

This crate has no networking, persistence, or framework concerns.

### `crates/app`

Application orchestration and contracts:

- repository and adapter ports
- bootstrap use case
- runtime state transitions
- typed snapshot assembly
- command handling for start/stop/settings/paper actions
- live credential metadata, readiness, read-only validation, shadow reconciliation, intent preview, preflight, testnet/mainnet-canary execution gating, kill switch, auto-execution, cancel/flatten orchestration, and start-gating services

### `crates/infra`

Operational adapters:

- Binance REST ranged-history adapter
- Binance WebSocket kline stream adapter
- SQLite repositories and migrations
- OS secure-storage adapter for live secrets
- Binance live adapter for signed validation, account snapshots, symbol rules, dedicated position/multi-assets mode checks, listenKey lifecycle, user-data events, testnet `order/test` preflight, and constrained `MARKET` / `LIMIT` order/cancel requests
- internal event bus for outbound server events
- system metrics sampler

### `crates/server`

Delivery layer:

- Axum HTTP API
- Axum WebSocket endpoint
- env/config loading
- tracing setup
- static frontend serving

### `web`

React single-page dashboard:

- bootstrap load through REST
- WebSocket delta subscription
- selector-based Zustand store
- charting via `lightweight-charts`
- integrated LIVE ACCESS panel for credential/readiness, shadow sync, intent preview, preflight, kill switch, TESTNET auto, explicit TESTNET execution/cancel/flatten, and default-off MAINNET canary controls

## Startup Flow

1. Load settings.
2. Load persisted paper state.
3. Load recent logs.
4. Validate or repair the default symbol.
5. Load recent klines from SQLite.
6. Fetch an explicit Binance REST kline range if deterministic local history is insufficient.
7. Compute ASO history.
8. Derive signal history on closed candles only.
9. Compute position and performance snapshot.
10. Publish bootstrap payload.
11. Start the Binance kline WebSocket stream.

## Runtime Flow

On stream updates:

1. Update the current candle in memory.
2. Persist closed candles.
3. Incrementally update ASO.
4. Emit a signal only when a closed-candle crossover occurs.
5. Route the signal into the paper engine.
6. Update mark-to-market for the current position.
7. Persist meaningful state transitions.
8. Publish throttled WebSocket deltas.

## History And Recovery

History loading is planned in `crates/app` before any exchange request is made. The planner computes the required closed-candle window for chart seed coverage, ASO warmup, crossover correctness, and bounded recompute tails. It validates contiguous closed candles after local/remote merge.

The market-data port uses explicit ranged requests only. Bootstrap, runtime start, settings rebuilds, symbol/timeframe changes, and reconnect recovery all request aligned `startTime` / `endTime` windows through the same contract. Ambiguous history during bootstrap or rebuild returns a typed history error; ambiguous reconnect recovery emits `resync_required` so the frontend reloads a fresh snapshot.

## Key Constraints

- allowed symbols are fixed to `BTCUSDT` and `BTCUSDC`
- only one active symbol can run at a time
- only one open position can exist at a time
- opposite signals close or reverse the existing position
- live access supports credentials, validation, readiness, shadow sync, testnet preflight, constrained TESTNET matching-engine `MARKET` / `LIMIT` execution/cancel/flatten/auto-execution, and manual MAINNET canary execution behind default-off server gates
- resync uses `resync_required` plus a fresh snapshot, not fatal client desync
