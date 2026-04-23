# Project State

## Current Phase

Real TESTNET Soak Validation v1 attempted; blocked before exchange execution by missing operator-provided TESTNET credential metadata.

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
- The current report is `docs/LATEST_TESTNET_SOAK_REPORT.md`; it records `/api/live/credentials=[]`, `credentials_missing`, and `active_credential=null` for this validation attempt.

Conditional/algo orders, hedge mode, multi-assets mode, multi-symbol concurrent runtime, Tauri packaging, auth, multi-user support, strategy marketplace, and optimization tooling remain out of scope.

## Completed In This Phase

- Attempted the real TESTNET validation path through the running server with mainnet canary forced off.
- Proved the real-drill blocker through API state: no live credential summaries exist and live status is `credentials_missing`.
- Exported a blocked-run evidence bundle under `artifacts/testnet-soak/real-validation-blocked-20260423T1424Z/`.
- Added masked credential summaries to evidence exports so missing-credential blockers are auditable without exposing secrets.
- Added testnet soak runbook and mainnet canary checklist.
- Added latest soak report with explicit real-vs-mocked evidence status.
- Added read-only live evidence export script using existing API endpoints.
- Added guided operator soak wrapper that captures checkpoints without placing orders itself.
- Documented that MAINNET canary remains NO-GO until real TESTNET evidence is captured and reviewed.

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

The project is ready for a controlled real TESTNET soak drill once an operator creates/selects a TESTNET credential through the secure-store flow. Mainnet remains default-off and fail-closed; current recommendation is NO-GO for manual MAINNET canary until the real TESTNET evidence bundle exists.

## declared_next_task

Create/select a valid TESTNET credential through the secure-store flow, then run the real TESTNET soak drill and attach the generated evidence bundle to `docs/LATEST_TESTNET_SOAK_REPORT.md`.

## done_when

- A valid TESTNET credential is created or selected and validated through the secure-store flow.
- `scripts/run_testnet_soak.sh` captures a real evidence bundle.
- Manual TESTNET execution, kill switch, restart repair, reconnect repair, and applicable cancel/flatten behavior are recorded.
- TESTNET auto mode is exercised with a natural closed-candle signal or documented as a bounded no-signal timeout.
- `docs/LATEST_TESTNET_SOAK_REPORT.md` is updated with pass/fail/not-exercised results and a revised mainnet canary recommendation.
- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION` remains false during the TESTNET drill.

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

- Real exchange smoke for TESTNET auto/canary paths requires valid operator-provided Binance testnet credentials and OS secure-storage availability.

## Key References

- [V1 Release Status](./V1_RELEASE_STATUS.md)
- [Live Readiness](./LIVE_READINESS.md)
- [Live Execution Boundary](./LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](./SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](./PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](./LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](./LIVE_IMPLEMENTATION_PLAN.md)
