# Project State

## Current Phase

Mainnet Auto Live Support v1 implemented and verification gate complete.

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
- MAINNET auto live execution remains disabled by default. Mainnet Auto Live Support v1 now implements the gated future 15-minute `BTCUSDT` live session path, but no live auto session was run in this batch.
- The bounded TESTNET soak drill is documented in `docs/TESTNET_SOAK_RUNBOOK.md`.
- Evidence export scripts write secret-safe artifacts under `artifacts/testnet-soak/<timestamp>/`.
- The current real evidence bundle is `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`.
- Env-backed credential validation evidence is `artifacts/testnet-soak/20260424T061338Z-env-credential-validation/`.
- The latest MAINNET canary evidence is `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`; it records a guarded `BTCUSDT` MAINNET `SELL LIMIT 0.001 @ 77950` canary that submitted with ACK, canceled cleanly, reconciled with `executed_qty=0.000`, required no flatten, passed restart repair, and ended with the canary server flag disabled again.
- Post-canary audit reviewed that bundle, scoped the canary-specific `orders.json` / `fills.json` evidence to the single MAINNET canary order, preserved the original generic recent exports as `orders_all_recent.json` / `fills_all_recent.json`, verified no raw `.env` secrets in repository/evidence surfaces scanned, and confirmed a normal safe-default server run still reports MAINNET canary disabled and MAINNET execution blocked.
- Second MAINNET canary readiness dry-run evidence is `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`. No real order was submitted. The dry-run selected and validated `env-mainnet`, refreshed mainnet readiness/shadow, verified no open BTCUSDT mainnet order, verified the previous canary remained canceled with no fill, exercised kill-switch engage/release, and built a fresh non-marketable `BUY LIMIT BTCUSDT 0.001 @ 77800` preview with reference `78294.8`, source `internal_market_candle`, age `25046 ms`, required margin `15.56`, and available `USDT=25.0902305`.
- Second bounded MAINNET canary execution evidence is `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`. Exactly one real `BUY LIMIT BTCUSDT 0.001 @ 77800` order was submitted with ACK, canceled, reconciled with `executed_qty=0.000`, required no flatten, passed restart repair, and ended with the canary server flag disabled again. MAINNET auto remained blocked. The run exposed a cancel endpoint ergonomics issue: the first cancel request omitted `order_ref` in the JSON body and was rejected even though the path contained the target order. Retrying the same order with `order_ref` and the exact confirmation text canceled cleanly without any additional order. The endpoint is now fixed so the path `order_ref` is authoritative and body `order_ref` is optional.
- Post-fix safe-default smoke on 2026-04-24 confirmed `/api/health`, `/api/bootstrap`, `/api/live/status`, `/api/live/credentials`, and `/` respond with `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`; `env-mainnet` validates as masked metadata only; MAINNET auto remains stopped; both recorded MAINNET canary orders remain `canceled` with `executed_qty=0.000`; no MAINNET BTCUSDT fills were returned; the BTCUSDT account snapshot position amount was `0`; and raw env secrets were not found in smoke API payloads.
- Mainnet canary closure review inspected the real TESTNET soak, first MAINNET canary, and second MAINNET canary evidence bundles. The TESTNET bundle is a valid historical soak export but predates the newer canary evidence layout, so it uses `credentials.json` with masked fields and does not include the newer before/after snapshot filenames or `final_verdict.json`.
- Closure safe-default smoke on 2026-04-24 confirmed `env-mainnet` validates for read-only status, `mainnet_canary.enabled_by_server=false`, `mainnet_canary.manual_execution_enabled=false`, MAINNET auto `stopped`, both MAINNET BTCUSDT canary orders remain `canceled` with `executed_qty=0.000`, no MAINNET BTCUSDT fills are returned, BTCUSDT account snapshot position amount is `0`, `.env` is ignored/untracked, and raw env secrets are absent from captured smoke payloads.
- `docs/OPERATOR_HANDOFF.md` now captures the safe operator handoff: how to start safely, verify env credential mode, confirm mainnet canary is disabled, inspect status/orders/fills, locate evidence, and avoid accidental scope expansion.
- Release-candidate cleanup documented git/worktree hygiene and evidence policy in `docs/FINAL_RC_SNAPSHOT.md`: source/docs should be committed, `.env` stays ignored/untracked, raw operational evidence remains ignored local artifact data unless a future task explicitly curates and secret-scans it, and generated `web/dist`, `target`, `var`, and dependency outputs remain ignored.
- Shareable RC UI cleanup added a top safety status strip and clearer LIVE ACCESS sections for credential, readiness/shadow/account, preview/preflight, safety/canary controls, orders/fills, and advanced details. No trading behavior was added and no order was submitted in the cleanup batch.
- MAINNET auto infrastructure v1 now exists as default-off infrastructure only. It adds typed config gates, persisted risk budget/state/decision/watchdog/lesson metadata, dry-run start/stop/status endpoints, live-start fail-closed blocking, headless helper scripts, dry-run evidence export under `artifacts/mainnet-auto/<timestamp>/`, and lesson reports. Dry-run mode can inspect live status and record ASO decision outcomes without submitting orders. Live MAINNET auto remains disabled by default and requires a later explicit live-run task before any real order can be considered.
- Credential-selected MAINNET auto dry-run evidence is `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`. It ran against the operator DB with `env-mainnet` explicitly selected and validated, `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false`, `RELXEN_MAINNET_AUTO_MODE=dry_run`, and the dry-run risk profile `mainnet-auto-operator-dry-run-v1`. The run refreshed mainnet readiness/shadow, confirmed `BTCUSDT`, one-way mode, single-asset mode, leverage `5`, no open BTCUSDT mainnet order, no BTCUSDT position, mainnet canary disabled, kill switch released, and live auto disabled. One dry-run decision was recorded as `dry_run_would_submit` using REST mark price with age `421 ms`; `orders.json` and `fills.json` are empty, `final_verdict.json` says `no_live_order_submitted`, and live-start verification returned `config_blocked` with live auto config disabled.
- Mainnet Auto Live Support v1 adds typed live start support for a future session only: `BTCUSDT`, `MARKET`, 15 minutes, exact session confirmation `START MAINNET AUTO LIVE BTCUSDT 15M`, server config gates, fresh mainnet credential/readiness/shadow/rules/reference gates, flat start, one-way/single-asset mode, leverage `<=5`, risk budget, watchdog, evidence logging, and lesson reporting. Closed-candle ASO signals can submit only through an internal `live_running` auto session policy; the public manual execute endpoint remains TESTNET/canary confirmed. Mocked-adapter tests prove the path without using a real exchange order call.
- The Mainnet Auto Live Support verification gate was rerun without starting a real MAINNET auto session, without submitting any TESTNET or MAINNET order, and without calling real cancel/flatten against a new order. The gate now explicitly tests mocked shadow open-position and open-order blockers for live ASO signals and records separate `open_position` / `open_order` decision blockers before any submit attempt.
- Operator-start preparation for the 15-minute live trial is complete. `scripts/run_mainnet_auto_live_trial.sh` now accepts the explicit batch command flags, verifies its shell and the running server are both in session-scoped live mode, configures the bounded risk budget, and then calls the existing typed start endpoint. No live session was started and no order was submitted during this preparation.
- Startup compatibility for existing operator databases is fixed: legacy `mainnet_auto_state` singleton JSON records that predate the `watchdog` field now decode with default watchdog/config/risk values instead of blocking server initialization. A session-scoped live-auto server smoke returned `/api/health` without starting the auto session or submitting an order.
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
- Audited the successful MAINNET canary evidence bundle without submitting another order. The audit found no exposure, no canary fill, and no enabled mainnet canary flag after restart. It also tightened the evidence bundle so `orders.json` and `fills.json` prove the single canary outcome rather than mixing in previous TESTNET recent records.
- Dry-ran readiness for a possible second MAINNET canary without enabling the server canary flag and without submitting an order. Current price/signal conditions required a `BUY` non-marketable preview rather than reusing the previous `SELL @ 77950`; the final dry-run profile used the current smallest exchange-compliant preview notional `77.8` at `5x`.
- Executed the second bounded manual MAINNET canary as a separate session. The server canary flag was enabled only for the canary session, the exact generated confirmation text was used, one order was submitted, cancel/reconcile/restart-repair completed, and the server was restarted with `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`.

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

The project has completed the first credential-selected MAINNET auto dry-run on the operator DB and now has Mainnet Auto Live Support v1 implemented, verified, and unrun. It has env-backed credential validation evidence, real TESTNET soak evidence, two bounded manual MAINNET canary execution bundles, the cancel endpoint body ergonomics fix, an operator handoff doc, a final RC snapshot doc, a cleaner operator-facing dashboard, mainnet-auto dry-run infrastructure, an operator-DB dry-run evidence bundle with no live order submitted, a gated future 15-minute live-auto support path, mocked tests proving live ASO signals block on existing MAINNET shadow position/order state, an operator-ready live-trial helper matching the requested terminal flags, and a startup compatibility fix for legacy persisted mainnet-auto status JSON. Mainnet auto live execution remains default-off until the operator starts the explicit session.

## declared_next_task

Operator may start the 15-minute live MAINNET auto trial from the terminal using the documented session-scoped server and helper commands. After that run, review/export evidence, verify flat stop, and update docs with the actual run result. Do not enable or run live MAINNET auto in normal startup.

## done_when

- A post-canary safe-default server run confirms `/api/health`, `/api/bootstrap`, `/api/live/status`, `/api/live/credentials`, and `/` are reachable without enabling the MAINNET canary flag.
- The latest canary evidence remains secret-safe and canary-specific for order/fill outcome review.
- A second-canary dry-run evidence bundle exists and records no order submission, no open mainnet order, no fill, fresh reference/marketability diagnostics, and mainnet canary disabled.
- A second-canary execution evidence bundle records exactly one order, clean final cancel/reconciliation, no fill, flat final position, restart repair, and mainnet canary disabled afterward.
- `POST /api/live/orders/:order_ref/cancel` accepts the path order reference without requiring a duplicate `order_ref` in the JSON body, rejects mismatched optional body `order_ref`, and preserves TESTNET / MAINNET confirmation gates.
- The post-fix safe-default smoke confirms canary disabled, auto stopped, previous MAINNET orders canceled, no MAINNET fills, flat BTCUSDT account snapshot position, and no raw secret exposure in smoke payloads.
- `docs/OPERATOR_HANDOFF.md` exists and reflects the final safe operating posture.
- `docs/FINAL_RC_SNAPSHOT.md` exists and documents repo hygiene, evidence policy, safe startup, test/build gate status, known risks, and the exact next bounded task.
- The dashboard shows safety-critical state in plain text by default, including MAINNET auto blocked, MAINNET canary disabled/enabled, kill switch state, active symbol, current mode, blockers, and latest order/fill truth.
- `/api/live/mainnet-auto/status`, dry-run start/stop, decisions, lessons, risk-budget, and evidence-export endpoints exist and show live auto blocked by default.
- The operator-DB MAINNET auto dry-run evidence bundle records `dry_run_would_submit`, no live order submission, empty `orders.json` / `fills.json`, a blocked live-start check, and `lessons.md` / `lessons.json`.
- `POST /api/live/mainnet-auto/start` accepts only the gated future live session shape and remains blocked unless server live config, exact session confirmation, risk, watchdog, and live-state gates pass.
- Closed-candle MAINNET auto ASO signal tests use mocked adapters only and prove existing shadow open-position and open-order state blocks before submit.
- `scripts/run_mainnet_auto_live_trial.sh` and enhanced `scripts/show_mainnet_auto_status.sh` exist for an explicit operator execution batch. The live-trial helper accepts the requested flags and refuses to start unless the running server is already live-auto enabled for the session.
- Existing operator DBs with pre-watchdog `mainnet_auto_state` JSON decode successfully and do not block startup.
- `docs/LATEST_MAINNET_CANARY_REPORT.md` records the pass outcome, audit result, and post-canary recommendation.
- No auto execution, no conditional/algo orders, no heatmap/liquidation decision layer, and no hidden bypass path are used.

## Not Now

- MAINNET auto-execution.
- Broad mainnet enablement policy beyond manual canary gates.
- Conditional/algo orders.
- Liquidation heatmap/liquidation-context module or any new live decision layer.
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
