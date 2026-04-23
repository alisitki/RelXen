# Decisions

## 2026-04-23

### Clean-room workspace

The repository is built from scratch around the requested domain/app/infra/server split instead of mirroring an existing product layout.

### Paper-only execution in batch one

The initial runtime kept live-trading affordances behind disabled placeholders. Post-v1 live foundations now support credentials and read-only readiness, but no real order placement is implemented.

### Live credential storage

Live credential raw secrets are stored only behind the secret-store abstraction. Normal runtime uses OS secure storage; SQLite stores masked metadata only. Plaintext secret persistence, raw-secret API echo, frontend storage, and secret logging remain forbidden.

### Single-symbol runtime discipline

The runtime enforces one active symbol and one open position. Symbol changes are blocked while a position is open.

### Numeric model for v1 paper trading

The v1 paper engine uses `f64` for pricing, fee, and PnL calculations instead of decimal fixed-point types. This keeps the clean-room vertical slice smaller and easier to verify in the requested batch. Before any real execution support is added, precision strategy should be reviewed and likely tightened.

### Trade history delivery

Trade history now bootstraps from the typed snapshot/REST path and stays current through dedicated `trade_appended` and `trade_history_reset` websocket events. This keeps the operator loop event-driven in paper mode without introducing a fuller order-management stream model.

### Single async state lock in v1 runtime

The app service uses a single async mutex around runtime state. This is acceptable for the current single-user local dashboard and keeps orchestration predictable. If runtime contention grows, state can be split into narrower locks later.

### Deterministic reconnect policy

After a websocket interruption, the runtime does not trust live deltas immediately. It compares the first post-reconnect stream candle against the last persisted/open-time anchor, fetches a bounded recent REST window, and replays only a contiguous closed-candle tail through the normal ASO, signal, and paper-trading path. If continuity cannot be proven, the runtime emits `resync_required` and rebuilds from a fresh snapshot instead of guessing.

### Ranged-only history contract

The public market-data port exposes explicit kline range requests only. Bootstrap, settings rebuilds, symbol/timeframe changes, runtime start, and reconnect recovery all use aligned start/end open-time windows instead of a recent-limit compatibility path. This keeps history behavior deterministic and makes contiguity failures visible as typed application errors.

### Live execution must be operator-gated and fail-closed

Future live mode must require explicit operator arming and must block order placement whenever credentials, market data, exchange rules, account snapshots, precision validation, risk limits, or reconciliation state are missing, stale, invalid, or ambiguous.

### Live foundations are read-only until execution is separately implemented

Credential validation, account snapshots, symbol rules, readiness checks, and read-only arming were implemented as live foundations before the executor slice. That foundation slice intentionally did not include order placement, cancellation, or exchange position mutation.

### Paper state is not live truth

The paper engine remains a simulator. Future live execution must treat exchange account snapshots, order statuses, and fills as authoritative and must not use paper wallet or position state as the source of live reconciliation truth.

### Live precision must be stricter than paper precision

The `f64` model is acceptable for Paper Mode V1 only. Future live execution must introduce decimal or fixed-point execution math and exchange-rule validation before any real order can be submitted.

### Live shadow/preflight is not execution

RelXen can maintain Binance user-data shadow state, build decimal/rules-aware order-intent previews, and validate payloads through Binance testnet `order/test`. Preflight is intentionally not execution: preflight success must never create a local live position or report "order placed".

### Mainnet preflight remains blocked

The preflight path is testnet-only in this repository state. Even if an exchange offers non-matching-engine validation on mainnet, RelXen keeps mainnet preflight blocked until the next execution slice has explicit operator confirmations, kill-switch behavior, and reconciliation evidence.

### Constrained testnet execution is allowed, mainnet execution remains blocked

RelXen now permits actual Binance USDⓈ-M Futures TESTNET `MARKET` / `LIMIT` placement, cancel, cancel-all-active-symbol, and flatten only after explicit operator confirmation and fail-closed gates. MAINNET execution has no bypass in this build. Conditional/algo orders remain out of scope.

### Exchange reconciliation is authoritative for testnet execution

Local live order records are request/audit state, not fill truth. User-data stream order/fill updates and bounded REST repair define order lifecycle, fills, and shadow account/position state. The UI must not show a placement acknowledgement as a fill, and ambiguous submission status must block new submissions until repaired or marked degraded.
