# Project State

## Current Phase

Real TESTNET Soak Validation v1 completed with operator-provided TESTNET credentials and a captured evidence bundle.

## Current Status

Paper Mode V1 remains release-candidate complete and runnable end-to-end as a local single-user Binance Futures paper-trading dashboard.

Post-v1 live execution is now mainnet-ready in bounded engineering terms and has a repeatable TESTNET soak evidence workflow:

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
- The current report is `docs/LATEST_TESTNET_SOAK_REPORT.md`; it records real TESTNET credential validation, shadow sync, preview, preflight, manual execution, cancel, flatten, kill switch, restart/recent-window repair, reconnect repair, and TESTNET auto proof with duplicate suppression.

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

The project now has a real TESTNET evidence bundle and a bounded CONDITIONAL GO recommendation for one manual MAINNET canary session. Mainnet remains default-off and fail-closed until an operator intentionally enables the canary server gate and follows the updated checklist/runbook exactly.

## declared_next_task

Run one single-order manual MAINNET canary session with the existing gates and capture a matching evidence bundle plus report update.

## done_when

- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` is enabled only for the canary session and turned back off immediately afterward.
- Exactly one manual MAINNET canary order is submitted through the existing confirmation-gated path.
- ACK, authoritative reconciliation, kill switch, and rollback/flatten behavior are captured in a fresh evidence bundle.
- `docs/LATEST_TESTNET_SOAK_REPORT.md` or a follow-up canary report records pass/fail outcomes and the post-canary recommendation.
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

- MAINNET canary still requires intentional operator enablement, a valid mainnet credential, and a deliberate decision to move beyond the default-off state.

## Key References

- [V1 Release Status](./V1_RELEASE_STATUS.md)
- [Live Readiness](./LIVE_READINESS.md)
- [Live Execution Boundary](./LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](./SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](./PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](./LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](./LIVE_IMPLEMENTATION_PLAN.md)
