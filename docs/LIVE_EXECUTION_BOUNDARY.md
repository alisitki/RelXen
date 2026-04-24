# Live Execution Boundary

## Scope

This document defines live-mode boundaries in terms of the current RelXen architecture. Credential validation, account snapshots, symbol rules, readiness, arming, user-data shadow reconciliation, precision-aware intent preview, testnet `order/test` preflight, constrained TESTNET `MARKET` / `LIMIT` execution/cancel/flatten/auto-execution, default-off manual MAINNET canary execution, and MAINNET auto dry-run infrastructure exist.

## Reusable Current Components

- Candle ingestion: Binance REST ranged-history loading and WebSocket kline streaming can feed live strategy decisions.
- ASO calculation: Domain ASO logic can remain pure and reusable.
- Signal generation: Closed-candle crossover rules can produce strategy signals.
- Settings/runtime orchestration: App service patterns can coordinate lifecycle and snapshots.
- WebSocket snapshot/delta model: Existing outbound event discipline can show live readiness and reconciliation state.
- Persistence foundations: SQLite can store non-secret settings, metadata, audit events, order-intent records, and reconciliation summaries.
- Operator UI shell: Current panels and text-first status language can be extended carefully.

## New Boundaries Required For Live Mode

- Credential provider: implemented as the secret-store abstraction; normal production-minded runtime uses OS secure storage, and explicit local env loading is available with `RELXEN_CREDENTIAL_SOURCE=env`.
- Credential validation service: implemented for signed read-only Binance USDⓈ-M Futures account checks.
- Exchange symbol-rules provider: implemented for the active supported symbols and the core filters needed by future execution.
- Account snapshot provider: implemented for read-only balances and positions.
- Order-intent builder: implemented for supported symbols and `MARKET` / `LIMIT` previews using settings, shadow account state, precision, and exchange-rule validation.
- Preflight adapter: implemented for Binance testnet `order/test`; this validates signed payloads but does not place orders.
- Execution adapter: implemented for TESTNET and canary-gated MAINNET `MARKET` / `LIMIT` new-order and cancel requests after all gates pass.
- Fill/order-status reconciliation: implemented for order lifecycle updates through user-data events plus bounded recent-window REST repair.
- Live position truth: current exchange-authoritative shadow/account/order/fill state remains distinct from paper state and is used to gate execution.
- Kill-switch boundary: Blocks new orders immediately and permits only safe recovery actions when deterministic.
- Canary boundary: MAINNET manual canary requires server enablement, configured risk profile, arming, exact confirmation, and all normal gates.
- MAINNET auto boundary: dry-run/status/evidence/lesson infrastructure exists; live MAINNET auto is blocked by default and requires explicit server config, operator arm/start, valid risk budget, watchdog, evidence logging, lesson reporting, and all normal gates in a future live-run task.

## Future Live Runtime State Machine

- `disabled`: Live mode is unavailable.
- `credentials_missing`: Operator has not configured credentials.
- `secure_store_unavailable`: Credential material cannot be read from OS secure storage.
- `validation_pending`: Credential, exchange rule, account, or settings validation is running.
- `validation_failed`: Credentials failed or validation is missing/stale.
- `rules_unavailable`: Symbol rules are missing.
- `account_snapshot_unavailable`: Read-only account snapshot is missing.
- `not_ready`: One or more readiness gates are blocked.
- `ready_read_only`: Preconditions are valid for read-only readiness, but execution is unavailable.
- `armed_read_only`: Operator explicitly armed read-only live mode; order execution is still unavailable.
- `shadow_starting`: listenKey creation, REST shadow bootstrap, or stream attachment is in progress.
- `shadow_syncing`: shadow state is being rebuilt from REST and user-data stream state.
- `shadow_running`: read-only shadow stream is coherent enough for preflight work.
- `shadow_degraded`: shadow state is stale, disconnected, or ambiguous; fail closed.
- `preflight_ready`: local intent validation passed and testnet `order/test` may be run.
- `preflight_blocked`: local intent/preflight validation is blocked.
- `testnet_execution_ready`: all gates pass for explicit operator-confirmed TESTNET execution of the displayed preview.
- `testnet_auto_running`: closed-candle TESTNET auto-execution is running.
- `testnet_submit_pending`: a TESTNET order was submitted and awaits exchange reconciliation.
- `testnet_order_open`: a TESTNET order is working.
- `testnet_partially_filled`: a TESTNET order has partial exchange-reported fills.
- `testnet_filled`: a TESTNET order is filled.
- `testnet_cancel_pending`: cancel was requested and awaits exchange reconciliation.
- `testnet_flatten_pending`: flatten was requested and awaits exchange reconciliation.
- `execution_degraded`: order, stream, or repair state is ambiguous; fail closed.
- `mainnet_execution_blocked`: MAINNET is disabled by server canary policy or another fail-closed gate.
- `mainnet_canary_ready`: manual MAINNET canary gates can pass for the displayed preview.
- `mainnet_manual_execution_enabled`: exact confirmation for the current MAINNET preview is available. MAINNET auto live mode remains unavailable by default.
- `dry_run_running`: MAINNET auto dry-run is recording decisions/evidence without submitting orders.
- `watchdog_stopped`: MAINNET auto stopped with a persisted watchdog/operator reason.
- `execution_not_implemented`: retained for routes/features that still do not have an execution implementation, including broader exchange features and unsupported order types.
- Liquidation heatmap/liquidation-context features are outside this boundary for now. They must not be added as live decision inputs or execution gates without a separate design batch.
- `start_blocked`: A live start/check command is blocked by the current gate result.
- `ready`: Future execution-ready state after order-intent/executor work exists.
- `armed`: Future execution-armed state after order-intent/executor work exists.
- `starting`: Live runtime is starting and refreshing exchange state.
- `live_running`: Live strategy loop may build intents and submit orders through gates.
- `degraded`: Connectivity or data quality is impaired; new orders should be blocked unless explicitly safe.
- `reconciliation_failed`: Exchange/account/order truth is ambiguous; fail closed.
- `kill_switch_engaged`: Operator or system blocked all new live execution.
- `stopping`: Runtime is stopping and reconciling final state.
- `stopped`: Runtime is inactive.
- `error`: Runtime hit an unrecoverable error and must be inspected.

## Fail-Closed Behavior

No live order may be placed when:

- Credentials are missing, invalid, expired, revoked, or unvalidated.
- Credential environment does not match configured environment.
- Live runtime is not armed.
- Symbol or timeframe is unsupported.
- Exchange symbol rules are missing, stale, or inconsistent.
- Account snapshot is missing or ambiguous.
- Shadow stream/account environment does not match the active live environment.
- An open live position exists that cannot be reconciled to the current strategy state.
- Market data is stale or `resync_required` is active.
- Environment is MAINNET and the canary server flag is disabled.
- Environment is MAINNET and the operator risk profile is missing.
- Environment is MAINNET and exact canary confirmation text is missing or mismatched.
- Environment is MAINNET and the preview is not a non-marketable `LIMIT` after tick-size rounding, or the reference price is missing/stale.
- The displayed preview is missing, stale, or mismatched from the submitted intent id/hash.
- Clock drift or request timestamp confidence is outside the configured tolerance.
- The order intent fails precision, sizing, notional, leverage, reduce-only, available-balance, or risk checks.
- The kill switch is engaged.
- Reconciliation failed after a prior order.
- Account mode checks report hedge mode or multi-assets mode.

## Terminology Separation

- Strategy signal: Domain-level BUY/SELL result from closed-candle ASO crossover.
- Execution intent: App-level proposed live action after settings and risk context are considered.
- Exchange order request: Adapter-level signed request with exchange-specific fields, rounded values, and flags.
- Accepted order: Exchange `ACK` that the request was accepted, not necessarily working or filled.
- Fill: Exchange-reported execution quantity/price/fee.
- Reconciled live position/account state: Local model updated from exchange-authoritative account, position, order, and fill data.

These objects must have separate types and audit events. A signal must never be treated as proof that a live order exists.

## Why Paper Engine State Is Not Live Truth

The paper engine is a deterministic simulator. It assumes fills at selected prices, local wallet state, local fees, and local position transitions. A live exchange can partially fill, reject, cancel, expire, liquidate, apply bracket changes, charge different fees, or report state asynchronously.

Future live execution must therefore treat exchange snapshots, order updates, and fills as authoritative. Paper outcomes can remain useful for comparison or dry-run display, but they cannot drive live reconciliation truth.
