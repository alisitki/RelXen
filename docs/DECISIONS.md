# Decisions

## 2026-04-25

### MAINNET auto margin type and ASO policy are explicit session policy

MAINNET auto now treats cross/isolated margin type as an explicit policy gate, separate from one-way position mode and single-asset/multi-assets margin mode. The default allowed margin type is `isolated`; actual `cross` requires explicit `RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=cross` or `any`; actual `unknown` blocks live MAINNET auto. ASO position policy is also explicit: `crossover_only` remains the conservative default, `always_in_market` may enter from latest closed ASO state and is therefore more active/riskier, and `flat_allowed` filters weak ASO states with delta/zone thresholds. In `flat_allowed`, weak state while already positioned defaults to hold rather than adding an unstated stop-loss/take-profit or flatten policy.

### MAINNET auto live evidence is session-scoped

The 15-minute MAINNET auto live trial exposed that generic recent order/fill exports can mix historical TESTNET and prior MAINNET canary records into a live-auto evidence bundle. Live-auto evidence now scopes decisions, orders, and fills to the active mainnet-auto session window and matching session order identifiers. `final_verdict.json` records stop reason, signal/decision counts, order/fill counts, realized PnL, fees, final position, final open orders, and flat-stop outcome. Historical evidence is not rewritten; fresh exports use the scoped format.

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

### Liquidation heatmap is deferred until after mainnet safety hardening

Liquidation heatmap or liquidation-context work is intentionally deferred after the first successful bounded MAINNET canary. ASO remains the active strategy signal, and no new heatmap models, endpoints, frontend panels, or live decision layer should be added until source semantics, data quality, and execution-safety implications are designed in a separate batch.

### Cancel route path order reference is authoritative

`POST /api/live/orders/:order_ref/cancel` now treats the route path `order_ref` as the cancel target. The JSON body carries confirmation fields only; an optional body `order_ref` is accepted for compatibility when it matches the path and rejected when it differs. This avoids duplicating target identity in normal clients while preserving fail-closed mismatch handling.

### Operational evidence remains ignored local artifact data by default

TESTNET soak and MAINNET canary bundles under `artifacts/testnet-soak/` and `artifacts/mainnet-canary/` are local operational evidence and remain ignored by default. Release-candidate commits should carry durable docs and summaries, not raw artifact bundles, unless a future task explicitly curates and secret-scans a bundle for publication. Historical evidence should not be rewritten to hide real failures; reports must preserve truthful outcomes such as the second canary's failed first cancel attempt followed by a successful retry.

### Mainnet auto infrastructure is dry-run first and lesson reports are analysis only

RelXen may expose MAINNET auto state, risk-budget, dry-run, decision-journal, watchdog, evidence, and lesson-report infrastructure before live MAINNET auto execution is authorized. The default config keeps `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false` and `RELXEN_MAINNET_AUTO_MODE=dry_run`; dry-run records would-submit/blocker decisions but must not call the exchange order endpoint. Lesson reports are operator review artifacts only: they must not automatically change strategy settings, risk budget, symbol scope, or live enablement. Any real MAINNET auto trial requires a later explicit live-run batch with fresh gates.

### First MAINNET auto live support uses session confirmation and MARKET-only BTCUSDT

Mainnet Auto Live Support v1 implements the future live path without running it. The supported live-auto trial shape is exactly `BTCUSDT`, 15 minutes, `MARKET`, session-level confirmation `START MAINNET AUTO LIVE BTCUSDT 15M`, notional cap `80`, session loss cap `5 USDT`, max leverage no higher than the configured budget and hard-capped at `100`, one in-flight order, one open position maximum, watchdog runtime stop, and evidence/lesson logging. The public manual execute endpoint remains canary-confirmed; mainnet auto orders can only use the internal auto execution policy from a `live_running` session. Reversal is not improvised: if an unresolved order or position exists and the current policy cannot prove a coherent reduce-only close and flat reconciliation, the entry blocks or the watchdog stops/degrades.

### Mainnet auto margin type and ASO position policy are explicit session gates

MAINNET auto must not infer isolated margin from single-asset mode. Cross, isolated, and unknown margin type are modeled separately from one-way/single-asset account gates; the default allowed margin type remains `isolated`, unknown blocks live start, and cross margin passes only when the operator explicitly allows `cross` or `any` for that bounded session. ASO position policy also stays explicit: `crossover_only` is the conservative default and preserves existing behavior, `always_in_market` may enter from the latest closed-candle ASO state and is riskier, and `flat_allowed` filters weak ASO states with documented delta/zone thresholds instead of adding stop-loss, take-profit, heatmap, liquidation, or conditional-order behavior.

### Always-in-market reverse and flat-stop require reduce-only close reconciliation

The degraded 2026-04-25 `always_in_market` live run proved that blocking reversal and only reporting a flat-stop failure can leave exposure after a bounded session. MAINNET auto now owns a private reduce-only close path for coherent `always_in_market` reverse and `require_flat_stop`: close current position first, repair/reconcile until BTCUSDT is flat and no open order remains, and only then allow an opposite entry. If close submission, order repair, or position reconciliation is ambiguous, the system blocks the entry or marks stop degraded. This does not add conditional/algo orders, heatmap/liquidation logic, broader symbols, or a public mainnet bypass.

### Live-auto evidence includes bounded stop settlement

The operator-stopped 2026-04-25 `flat_allowed` live run showed that an auto-owned flat-stop order can be submitted milliseconds after the previously recorded `stopped_at` timestamp, causing the first export to omit the closing order/fill even though exchange state was flat. MAINNET auto now records stop time after flat-stop reconciliation, runs bounded live repair before evidence export, and includes session-owned stop/reverse settlement records for a 30-second post-stop evidence window. The window is intentionally narrow and tied to auto-owned order identity so evidence captures real close settlement without turning exports into broad recent-account dumps.

### Market-data stream opening must time out visibly

The same run showed the public kline runtime can remain in `opening Binance kline stream` without producing a first candle event. Runtime subscribe and first-event waits now time out after 15 seconds, record a reconnect/error state, and retry instead of staying silently stuck. This is an observability/safety hardening change only; it does not add a new signal, indicator, strategy rule, symbol, or order type.

### MAINNET auto requires fresh market data before and during exposure

The disabled-live-auto kline smoke showed that Binance REST history can be fresh while the WebSocket kline stream needs multiple reconnect attempts before its first usable event. MAINNET auto no longer treats bootstrap history alone as enough for policy entry: policy-driven entry waits for fresh runtime market data, the headless live helper starts the public runtime and waits up to 120 seconds for fresh BTCUSDT stream/closed-candle evidence before calling the live-start endpoint, and the session watchdog stops with `market_data_stale` and uses the existing reduce-only flat-stop path if fresh market data is lost during a live session. Closed-candle freshness is timeframe-aware: the latest closed candle may be up to one active timeframe plus the stale-data grace old, while stream message freshness still uses the tighter stale-data grace. This is a safety gate around data freshness, not a change to ASO strategy logic.

### Mainnet auto leverage budget hard cap is 100x

The explicit MAINNET auto leverage budget may now be configured up to `100x`; values above `100x` are rejected. The start gate still requires active-symbol exchange leverage to be no higher than the configured session budget, and RelXen still does not add an exchange leverage-adjustment endpoint. This is a gate change, not a recommendation to use high leverage.

### Operator-stop MAINNET auto runtime is explicit and default-off

The operator requested a live run shape that does not stop by a fixed 15-minute or 60-minute max-runtime watchdog. RelXen now represents that as the explicit runtime value `0`, meaning `operator_stop`: `expires_at` is left unset and the max-runtime watchdog stop is disabled for that session only. This mode requires the exact confirmation `START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP`, the running server config `RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=0`, and a matching risk budget/start request. It does not change default-off live-auto behavior and does not relax kill switch, max loss, order/fill caps, flat-start/flat-stop, margin-type, fresh market-data, shadow, reconciliation, evidence, or lesson-report gates.

### Do not repeat operator-stop live run after stale-data stop without repair hardening

The 2026-04-25 `5m` `always_in_market` operator-stop live attempt showed that a just-ACKed `MARKET` order can remain locally `accepted` until REST repair even though the exchange has filled it, and that the current `5m` closed-candle freshness watchdog can stop the session shortly after entry when no newer closed candle has reconciled yet. The system must not simply restart the same live run shape after this condition. The next implementation step is to harden pre-stop repair/classification for just-ACKed market orders and tune `5m` market-data freshness/reconnect behavior before another operator-stop live attempt.

### Stale closed-candle stop may repair only a small proved gap

MAINNET auto now tries a bounded REST kline repair before stopping for `market_data_closed_candle_stale`, but only when the stream itself is connected/resynced and the missing closed-candle gap is three candles or fewer. The recovered candles must exactly match the active symbol/timeframe, be closed, and be contiguous. Incomplete, non-contiguous, too-large, missing-anchor, disconnected-stream, or stale-stream cases still fail closed and stop/flat according to the existing watchdog policy. This keeps `5m` sessions from stopping only because a small closed-candle gap is recoverable, without turning stale market data into an unsafe allow condition.

### Auto flat-stop repairs ACK-only MARKET orders before classifying open-order blockers

Before MAINNET auto submits an auto-owned reduce-only close for flat-stop or reverse, it refreshes the live shadow and runs the bounded recent-window order repair path. This lets an ACK-only `MARKET` order that has already filled on the exchange reconcile to a terminal local state before the flat-stop logic checks for unexpected open orders. If refresh and repair both fail, the existing repository snapshot is still used and the close remains fail-closed on ambiguity. This changes stop/reverse reconciliation only; it does not add a public mainnet bypass, new strategy logic, conditional orders, or broader symbol scope.

### Shadow freshness uses REST and user-data freshness, not user-data events alone

The 2026-04-25 operator-stop run showed a quiet Binance user-data stream can produce no new account event while REST shadow repair is still fresh and exchange state remains coherent. MAINNET auto now treats shadow freshness as the newest of user-data event time, REST shadow sync time, and shadow update time. A running-but-stale shadow is repaired through read-only REST before live start, signal submit, or watchdog stop; only failed/incomplete repair, down/degraded stream state, or ambiguous shadow state remains fail-closed. Transient market-data `reconnecting` is also allowed inside the normal stream-message freshness window so a single reconnect tick does not flat-stop an otherwise fresh feed. This is a false-positive stop hardening decision, not a relaxation for genuinely stale market data.
