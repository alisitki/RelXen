# Project State

## Current Phase

Reference-price freshness hardening completed and one guarded manual MAINNET canary submitted, canceled, reconciled, and restart-repair checked.

## Current Status

Paper Mode V1 remains release-candidate complete and runnable end-to-end as a local single-user Binance Futures paper-trading dashboard.

Post-v1 live execution is now mainnet-ready in bounded engineering terms and has a repeatable TESTNET soak evidence workflow plus an env-backed credential path:

- Local `.env` credential loading is implemented behind `RELXEN_CREDENTIAL_SOURCE=env`; raw env secrets remain process-only, `.env` remains gitignored, and SQLite stores masked metadata plus source only.
- Authoritative env-source mode selects `env-testnet` at startup ahead of persisted secure-store TESTNET active selections so local validation does not trigger OS secure-storage prompts. MAINNET env credentials never auto-select and require explicit selection.
- TESTNET `MARKET` / `LIMIT` manual execution, cancel, cancel-all-active-symbol, flatten, and closed-candle auto-execution are implemented.
- A kill switch blocks new live submissions immediately.
- Duplicate closed-candle live auto intents are suppressed with persisted signal/intent locks.
- Real submissions use Binance `ACK` request handling and rely on user-data stream plus bounded recent-window REST repair for authoritative order/fill/account truth.
- Dedicated Binance position-mode and multi-assets-mode checks are used before live execution gates can pass.
- User-data streams force a reconnect/REST repair before the 24-hour WebSocket lifecycle limit.
- MAINNET manual canary execution is implemented behind `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` by default, a configured risk profile, arming, fresh shadow/rules/account state, one-way/single-asset mode, exact operator confirmation, and all normal execution gates.
- MAINNET auto-execution remains unavailable.
- The bounded TESTNET soak drill is documented in `docs/TESTNET_SOAK_RUNBOOK.md`.
- Evidence export scripts write secret-safe artifacts under `artifacts/testnet-soak/<timestamp>/`.
- The current real evidence bundle is `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`.
- Env-backed credential validation evidence is `artifacts/testnet-soak/20260424T061338Z-env-credential-validation/`.
- The latest MAINNET canary evidence is `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`; it records a guarded `BTCUSDT` MAINNET `SELL LIMIT 0.001 @ 77950` canary that submitted with ACK, canceled cleanly, reconciled with `executed_qty=0.000`, required no flatten, passed restart repair, and ended with the canary server flag disabled again.
- The current report is `docs/LATEST_TESTNET_SOAK_REPORT.md`; it records real TESTNET credential validation, shadow sync, preview, preflight, manual execution, cancel, flatten, kill switch, restart/recent-window repair, reconnect repair, and TESTNET auto proof with duplicate suppression.
- The latest canary report is `docs/LATEST_MAINNET_CANARY_REPORT.md`.

Conditional/algo orders, hedge mode, multi-assets mode, multi-symbol concurrent runtime, Tauri packaging, auth, multi-user support, strategy marketplace, and optimization tooling remain out of scope.

## Completed In This Phase

- Created, selected, and validated the operator-provided TESTNET credential through the existing secure-store flow.
- Proved real TESTNET readiness and shadow bootstrap with mainnet canary forced off.
- Captured real TESTNET preview, preflight, manual execution, cancel, flatten, kill switch, restart/recent-window repair, reconnect/recovery, and auto duplicate-suppression evidence.
- Exported the real evidence bundle under `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`.
- Fixed stale visible account snapshots after shadow refresh by deriving the visible account snapshot from the latest shadow state.
- Added a TESTNET-only, default-off drill helper for replaying the latest persisted closed signal through the existing auto executor when no natural signal appears during a bounded soak window.
- Fixed manual shadow refresh so it also performs bounded recent-window execution repair.
- Fixed recent-window repaired fills so they backfill local `order_id` and `client_order_id` when an authoritative exchange trade can be matched to a repaired order.
- Updated the soak runbook, mainnet checklist, latest soak report, runbook, README, and live-readiness docs with the real run and current recommendation.
- Implemented env-backed credential loading from local `.env`, masked env credential summaries, credential source metadata, compatibility alias precedence, and env-safe evidence export.
- Validated env-backed TESTNET and MAINNET credentials without printing or persisting raw secrets.
- Fixed authoritative env startup selection so `env-testnet` is selected before persisted secure-store TESTNET active credentials.
- Fixed shadow/reconciliation environment mismatch gating so stale TESTNET shadow state cannot satisfy MAINNET canary gates.
- Implemented an explicit environment/symbol-aware reference-price resolver. Fresh internal market state is preferred when valid, Binance USD-M REST mark price is used as the deterministic fallback, final MAINNET submit forces a fresh refresh after the kill-switch drill, and preview/evidence include reference source, age, rounded order price, and marketability.
- Retried the manual MAINNET canary with the existing smallest exchange-compliant `BTCUSDT` 5x profile. Available `USDT=25.0902305`, exchange leverage `5x`, and a final REST mark-price-backed non-marketable `SELL LIMIT BTCUSDT 0.001 @ 77950` preview passed. Exactly one MAINNET order was submitted, canceled, reconciled, and restart-repair checked.

## Previously Completed Execution Hardening

- Added persisted kill-switch, risk-profile, auto-executor, and intent-lock state.
- Added canary-specific server gating via `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION`; no generic mainnet bypass flag exists.
- Added conservative operator-configured risk profile requirement before MAINNET canary readiness.
- Changed real order submission semantics to `newOrderRespType=ACK` and local `accepted` state until authoritative reconciliation updates lifecycle state.
- Added dedicated Binance account-mode checks through `GET /fapi/v1/positionSide/dual` and `GET /fapi/v1/multiAssetsMargin`.
- Added forced user-data stream reconnect with REST repair before the 24-hour stream limit.
- Defined execution repair as bounded recent-window repair because Binance order/trade query retention is finite.
- Added closed-candle TESTNET auto-executor with persisted duplicate signal suppression.
- Added kill switch, risk profile, auto-executor, mainnet canary, ACK, account-mode, forced reconnect, and recent-window repair status into API/bootstrap/websocket/frontend state.
- Added app, infra, server, and frontend tests for the new gates and operator states.

## Current Focus

The project now has env-backed credential validation evidence, real TESTNET soak evidence, and a successful bounded manual MAINNET canary evidence bundle. Mainnet remains default-off and fail-closed; any broader mainnet capability still requires a separate design decision and implementation batch.

## declared_next_task

Review the successful MAINNET canary evidence bundle and decide the next bounded mainnet-readiness task; do not enable MAINNET auto or broader mainnet operation without a separate design batch.

## done_when

- Mainnet shadow shows enough available `USDT` balance for the smallest exchange-compliant non-marketable `LIMIT` preview at the approved notional/leverage cap, including fee/buffer.
- Account/exchange leverage for `BTCUSDT` is confirmed no greater than `5x`.
- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` is enabled only for the canary session and turned back off immediately afterward.
- Exactly one manual MAINNET canary order was submitted through the existing confirmation-gated path.
- ACK, authoritative reconciliation, kill switch, cancel, restart repair, and no-flatten-needed behavior are captured in a fresh evidence bundle.
- `docs/LATEST_MAINNET_CANARY_REPORT.md` records the pass outcome and post-canary recommendation.
- No auto execution, no conditional/algo orders, and no hidden bypass path are used.

## Not Now

- MAINNET auto-execution.
- Broad mainnet enablement policy beyond manual canary gates.
- Conditional/algo orders.
- Hedge mode or multi-assets mode support.
- Plaintext secret persistence.
- Using paper engine state as live account truth.
- Tauri packaging.
- Auth or multi-user support.
- Multi-symbol concurrent runtime.

## Known Blockers

- No active canary blocker remains for the bounded manual canary that was exercised. Broader mainnet enablement remains intentionally out of scope.

## Key References

- [V1 Release Status](./V1_RELEASE_STATUS.md)
- [Live Readiness](./LIVE_READINESS.md)
- [Live Execution Boundary](./LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](./SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](./PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](./LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](./LIVE_IMPLEMENTATION_PLAN.md)
