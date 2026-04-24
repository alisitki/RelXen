# Decisions

## 2026-04-24

### Env credential source is local-only operator convenience

RelXen now supports `RELXEN_CREDENTIAL_SOURCE=env` for local operators who need to avoid repeated OS secure-storage prompts during validation. The setting is explicit and authoritative; `RELXEN_ENABLE_ENV_CREDENTIALS=true` is only a compatibility alias when the source setting is unset. Raw env values remain process-only, SQLite stores only masked metadata plus `source=env`, authoritative env mode selects the TESTNET env credential at startup ahead of any persisted secure-store TESTNET active selection, and MAINNET env credentials never auto-select.

### First MAINNET canary must be non-marketable LIMIT

The manual MAINNET canary path remains default-off and now blocks `MARKET` for the first canary. A MAINNET canary `LIMIT` preview must remain non-marketable after tick-size rounding and requires a fresh reference price before exact confirmation can enable submission.

### Shadow environment mismatch is execution-blocking

Live shadow and user-data stream state must match the active live environment before readiness, preview, or execution gates can pass. Switching from TESTNET to MAINNET with stale TESTNET shadow metadata is treated as ambiguous state, even when credential validation and account reads succeed.

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

### Constrained testnet execution is allowed, mainnet remains canary-gated

RelXen now permits actual Binance USDⓈ-M Futures TESTNET `MARKET` / `LIMIT` placement, cancel, cancel-all-active-symbol, flatten, and closed-candle auto-execution only after explicit operator controls and fail-closed gates. MAINNET manual canary execution uses the same execution pipeline but is disabled by default through `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`, requires an operator-configured risk profile, exact confirmation text, fresh shadow/rules/account state, and all normal gates. MAINNET auto-execution and conditional/algo orders remain out of scope.

### Exchange reconciliation is authoritative for testnet execution

Local live order records are request/audit state, not fill truth. User-data stream order/fill updates and bounded REST repair define order lifecycle, fills, and shadow account/position state. The UI must not show a placement acknowledgement as a fill, and ambiguous submission status must block new submissions until repaired or marked degraded.

### Real submissions use ACK plus authoritative reconciliation

RelXen requests `ACK` for real Binance order submission. ACK confirms request acceptance only; user-data stream events and bounded REST repair are authoritative for working, partial-fill, fill, cancel, reject, and expiry states. Unknown submission outcomes are repaired before any retry to avoid duplicate orders.

### Execution repair is recent-window only

RelXen repair logic intentionally queries a bounded recent order/trade window because Binance order and trade query surfaces have retention limits. If an older or ambiguous execution state cannot be proven from the recent window, the system stays degraded and blocks new submissions instead of manufacturing certainty.

### Account mode checks use dedicated Binance endpoints

Live execution gates use Binance USDⓈ-M dedicated position-mode and multi-assets-mode endpoints rather than inferring mode from account snapshots alone. Hedge mode and multi-assets mode remain unsupported and fail closed.

### Soak evidence uses existing read-only APIs

TESTNET soak evidence is exported by scripts that call existing REST endpoints instead of adding a privileged evidence endpoint or hidden drill trigger. This keeps the server execution surface unchanged, avoids accidental order placement from tooling, and makes real exchange evidence explicit rather than inferred from smoke exports.

### TESTNET auto drill helper is explicit, gated, and not part of normal runtime

When a bounded soak window produces no natural fresh closed-candle auto signal, RelXen may expose a TESTNET-only drill helper that replays the latest persisted closed signal through the existing auto-execution path. The helper is off by default behind `RELXEN_ENABLE_TESTNET_DRILL_HELPERS=false`, requires explicit confirmation, and must never be used in MAINNET sessions. This keeps soak validation bounded without inventing a parallel execution path.

### Manual shadow refresh is also the bounded execution-repair path

Manual `Refresh Shadow` does not only refresh read-only account state; it also runs the existing recent-window execution repair logic. This keeps restart/reconnect/operator recovery on one bounded path instead of forcing operators to trust stale ACK-only order records after a stream gap or restart.

### Conservative mainnet canary profile must fit exchange minimums

The manual MAINNET canary path does not bypass exchange min quantity, min notional, available-balance, or configured risk-profile caps to force a first order through. The 2026-04-24 leverage-gated retry kept the server canary flag disabled and stopped before order submission because the approved `BTCUSDT` 50 USDT / 5x profile could not produce an exchange-compliant quantity at the current BTC price and still exceeded available USDT at 5x before fees/buffer. RelXen did not add or use an exchange leverage-adjustment endpoint in that retry.

### Exchange leverage must satisfy canary limits before mainnet submission

The 2026-04-24 balance-funded retry proved the smallest non-marketable `BTCUSDT` canary preview could pass local balance, reference-price, min-quantity, min-notional, and 5x risk gates after funding. RelXen still stopped before enabling the server canary flag because the account snapshot reported `BTCUSDT` exchange leverage `20x`, above the batch maximum `5x`, and the repository does not expose a reviewed safe exchange leverage-adjustment endpoint. The system must resolve that operator/exchange setting before a real MAINNET canary order.

### Reference price freshness remains a final mainnet canary hard gate

The 2026-04-24 leverage-fixed retry proved the same account/API context could report `BTCUSDT` leverage `5x`, sufficient available USDT, one-way mode, single-asset mode, and a valid non-marketable `LIMIT` preview. RelXen enabled the server canary flag only for that session and tested the kill switch, but stopped before order submission when the refreshed preview returned `reference_price_unavailable`. The system must not reuse an older preview or bypass the fresh-reference gate to force a MAINNET order.

### Mainnet reference price resolver may use REST mark price as deterministic fallback

The 2026-04-24 reference-price-fixed retry kept the fresh-reference hard gate but made the source explicit. RelXen now prefers fresh internal market state and falls back to Binance USD-M REST mark price for the active environment/symbol when internal state is missing, stale, or invalid after kill-switch release. Final MAINNET submit forces this refresh and still blocks on stale or failed reference-price resolution. Preview/evidence must record source, age, rounded order price, and marketability so the operator can audit why the final canary gate passed.
