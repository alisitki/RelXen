# Live Implementation Plan

## Phase 0 — Design Freeze

Goal: Close Paper Mode V1 and freeze post-v1 live-readiness design.

Non-goals: No source-code live implementation, no credential persistence, no order placement.

Likely files/modules to touch: Documentation only.

Tests required: Lightweight docs consistency checks.

Exit criteria: Live-readiness docs exist, README/project state/backlog are aligned, and one bounded next task is declared.

Key risks: Scope creep into live execution before safety boundaries exist.

Rollback/fail-closed behavior: Keep live mode disabled and paper runtime unchanged.

## Phase 1 — Credential And Validation Slice (Complete)

Goal: Add secure credential storage integration and masked credential validation without order placement.

Status: Implemented in Live Foundations v0 and extended with explicit env-backed local credential loading. Raw secure-store secrets are kept behind the secret-store abstraction, raw env secrets remain process-only, and SQLite stores masked metadata plus source only.

Non-goals: No order submission, no account trading actions, no autonomous live runtime.

Likely files/modules to touch:

- `crates/app`: credential ports, validation service, typed errors, metadata models.
- `crates/infra`: OS secure-storage adapter and exchange validation adapter.
- `crates/server`: masked credential metadata API and validation endpoint.
- `web`: small credential status/validation UI with no raw-secret display after submit.
- `docs`: runbook and security notes.

Tests required:

- Secure-storage adapter tests with fakes where OS integration is not deterministic.
- Credential validation service tests for valid, invalid, missing, permission-insufficient, and environment-mismatch states.
- HTTP tests proving raw secrets are never echoed.
- Frontend tests proving masked display and failure feedback.

Exit criteria:

- Credentials can be saved to OS secure storage; local env credentials can be surfaced as read-only masked summaries when explicitly enabled.
- Metadata persists without raw secrets.
- Validation endpoint returns typed masked status.
- Live order placement remains impossible.

Key risks: Secret leakage through logs, responses, frontend state, or tests.

Rollback/fail-closed behavior: If secure storage or validation fails, live state remains `credentials_missing` or `credentials_invalid`.

## Phase 2 — Exchange Rules And Account Snapshot Slice (Foundations Complete)

Goal: Add exchange symbol rules and account snapshot foundations needed for live readiness.

Status: Read-only Binance account snapshots, active-symbol rules, readiness checks, and read-only arming are implemented. Decimal intent/preflight math is implemented in Phase 3; exchange-authoritative placement/fill accounting remains deferred.

Non-goals: No autonomous live execution and no strategy-driven orders.

Likely files/modules to touch:

- `crates/domain`: decimal/fixed-point primitives and validation helpers.
- `crates/app`: symbol-rules and account-snapshot ports, fail-closed validation.
- `crates/infra`: Binance rules/account adapters.
- `crates/server`: read-only account/rules endpoints.
- `web`: textual account/rules status panels.

Tests required:

- Rules parsing and rounding tests.
- Account snapshot mapping tests.
- Fail-closed tests for stale/missing rules and ambiguous account state.
- HTTP tests for account/rules status.

Exit criteria:

- Active symbol rules load and expire deterministically.
- Account snapshot can be read with validated credentials.
- Precision/rules validation rejects unsafe quantities and notionals.
- No order placement exists.

Key risks: Treating read-only account snapshots as reconciled execution state too early.

Rollback/fail-closed behavior: Missing or stale rules/account snapshots keep live state out of `ready`.

## Phase 3 — Shadow And Testnet Preflight Slice (Complete)

Goal: Add read-only shadow reconciliation, narrow order-intent construction, and Binance testnet preflight validation behind explicit operator controls.

Status: Implemented as Live Shadow Mode v1. The system can open/maintain listenKey user-data streams, reconcile read-only shadow state from REST plus stream events, build decimal/rules-aware `MARKET` / `LIMIT` intents for `BTCUSDT` and `BTCUSDC`, and validate payloads through Binance testnet `order/test`. Actual placement and cancel execution remain absent.

Non-goals: No matching-engine order placement, no cancel execution, no mainnet autonomy, no advanced order types, no multi-symbol runtime.

Likely files/modules to touch:

- `crates/domain`: order-intent and live-risk validation types.
- `crates/app`: live runtime state machine, arming, intent builder, shadow reconciliation service, preflight service.
- `crates/infra`: Binance listenKey/user-data adapter pieces and `order/test` preflight adapter.
- `crates/server`: shadow, preview, preflight endpoints and live status events.
- `web`: unmistakable shadow/preflight controls and no-order-placed messaging.

Tests required:

- Intent builder tests from signal/settings/rules/shadow state to validated preview.
- User-data parser and listenKey lifecycle tests.
- Testnet/fake exchange adapter tests for preflight accept/reject/error paths.
- Shadow reconciliation tests for stale/degraded/ambiguous states.
- HTTP/WebSocket/frontend tests for shadow status, intent preview, and preflight results.

Exit criteria:

- Operator can arm read-only live mode explicitly.
- Shadow sync and preflight work only for the narrow supported symbol/order surface.
- Reconciliation failures fail closed.
- Preflight success never mutates live/paper positions and never reports "order placed".
- Evidence is green before any testnet placement or mainnet work.

Key risks: Accidental mainnet enablement, treating preflight as execution, insufficient reconciliation, unsafe retry behavior.

Rollback/fail-closed behavior: Disable arming, revoke credentials, and keep paper mode operational.

## Phase 3B — Testnet Placement, Cancel, Fill Reconciliation, And Flatten Slice (Complete)

Goal: Add testnet-only `MARKET` / `LIMIT` placement and cancel flow using the existing intent/preflight/shadow foundations.

Status: Implemented as Constrained Testnet Executor v1. RelXen can submit explicit operator-confirmed TESTNET `MARKET` / `LIMIT` orders, cancel RelXen-created TESTNET orders, cancel all active-symbol TESTNET open orders with backend confirmation, flatten a deterministic active-symbol TESTNET position, persist live order/fill state, and reconcile order/fill lifecycle through user-data events plus REST fallback. MAINNET execution is blocked.

Non-goals: No mainnet placement, no autonomous strategy-driven execution, no conditional/algo orders, no multi-symbol runtime.

Likely files/modules to touch:

- `crates/domain`: execution request/result models and stricter duplicate/order-state guards.
- `crates/app`: operator-gated testnet executor service, cancel service, kill-switch blockers, and reconciliation handoff.
- `crates/infra`: Binance testnet new-order and cancel-order adapter methods only.
- `crates/server`: placement/cancel endpoints with typed blocked/error responses.
- `web`: explicit testnet-only placement/cancel controls and confirmations.

Tests required:

- Local block tests for stale shadow state, unsupported mode, mainnet, duplicate intent, and unarmed runtime.
- Mocked Binance tests for accepted, rejected, timeout, and cancel paths.
- Reconciliation tests proving submitted orders are reflected through REST/user-data state before the UI treats them as live truth.
- Server/frontend tests proving no false success state and clear kill-switch/blocking UX.

Exit criteria:

- Only operator-confirmed testnet placement is possible.
- Mainnet placement is impossible.
- Cancel flow exists for RelXen-created testnet orders.
- Exchange/reconciliation ambiguity fails closed.
- Paper mode remains unchanged.

Key risks: Creating exchange state without reliable reconciliation, duplicate submission, or operator confusion between preflight and placement.

Rollback/fail-closed behavior: Disable placement endpoints, disarm live mode, leave shadow/preflight available, and keep paper mode operational.

## Phase 3C — Strategy-Driven Testnet Auto-Executor And Kill Switch Slice (Complete)

Goal: Add strategy-driven closed-candle TESTNET auto-execution behind explicit arming, duplicate-signal suppression, kill switch, and exchange-authoritative reconciliation.

Status: Implemented in Mainnet Readiness Hardening v1. TESTNET auto-execution is opt-in, closed-candle-only, backed by persisted signal/intent locks, and stopped by the kill switch and normal execution gates.

Non-goals: No mainnet execution, no conditional/algo orders, no unattended production claims, no multi-symbol runtime.

Likely files/modules to touch:

- `crates/domain`: duplicate intent/order key helpers if needed.
- `crates/app`: auto-execution orchestration, kill-switch state, duplicate suppression, and reconciliation gates.
- `crates/infra`: no broad adapter expansion unless repair gaps are found.
- `crates/server`: kill-switch and auto-execution control endpoints.
- `web`: explicit auto-execution/kill-switch controls and blocked-state UX.

Tests required:

- Closed-candle signal to TESTNET intent/order path.
- Duplicate signal suppression across reconnect/restart.
- Kill-switch blocks new submissions immediately.
- Stale shadow, unsupported account mode, mainnet, preview mismatch, and paper/live ambiguity remain blocked.
- User-data/REST repair remains authoritative for fills and open orders.

Exit criteria:

- Auto-execution is opt-in, armed, and testnet-only.
- Every auto-submitted order has a persisted signal/intent/order key.
- Kill-switch drills pass and mainnet remains blocked.

Key risks: Duplicate live submissions, operator confusion between manual and auto mode, and incomplete repair after partial fills.

Rollback/fail-closed behavior: Disable auto-execution, engage kill switch, keep manual testnet controls and paper mode operational.

## Phase 4 — Manual Mainnet Canary Slice (Canary-Ready)

Goal: Enable a tightly constrained manual mainnet canary path while keeping broad mainnet execution default-off and fail-closed.

Status: Implemented as a default-off manual canary path. `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` blocks it by default. When enabled, manual MAINNET canary submission uses the same ACK-plus-authoritative-reconciliation pipeline as TESTNET, requires a configured risk profile, fresh matching mainnet shadow/rules/account state, dedicated one-way and single-asset-mode checks, sufficient available balance, arming, a fresh non-marketable `LIMIT` preview after tick-size rounding, and exact operator confirmation. MAINNET auto-execution remains blocked. On 2026-04-24, reference-price freshness was hardened and one guarded MAINNET `BTCUSDT` `LIMIT` canary submitted, canceled, reconciled flat, and restart-repair checked under `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`.

Non-goals: No broad exchange feature set, no MAINNET auto-execution, no multi-symbol runtime, no unattended operation claims.

Likely files/modules to touch:

- Existing live credential, rules, account, execution, reconciliation, risk, server, and UI modules from Phases 1-3.

Tests required:

- Full fake-exchange regression suite.
- Testnet evidence runbook and soak drill.
- Mainnet canary blocked/default-off tests.
- Explicit operator confirmation tests.
- Failure and kill-switch drills.

Exit criteria:

- Mainnet canary requires server enablement, non-marketable `LIMIT` preview, and exact operator confirmation.
- Operator-configured risk profile is required and enforced.
- Reconciliation is exchange-authoritative.
- Rollback to paper-only is documented and tested.

Key risks: Real financial loss, operator confusion, stale account state, exchange-side rule changes.

Rollback/fail-closed behavior: Kill switch, disarm, disable `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION`, credential revocation, and paper-only fallback.

## Phase 4A — Testnet Soak Evidence Slice (Complete)

Goal: Produce operational evidence for TESTNET execution, kill switch, cancel, flatten, restart repair, reconnect repair, auto-execution, and recent-window repair honesty before any MAINNET canary recommendation.

Status: Completed with a real TESTNET soak on 2026-04-23. The run captured credential validation, readiness, shadow sync, preview, preflight, manual execution, cancel, flatten, kill switch, restart repair, reconnect repair, and duplicate-safe auto proof. The current TESTNET execution evidence bundle is `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`. The latest MAINNET canary evidence is `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`.

Non-goals: No new order types, no hidden drill trigger, no mainnet enablement, no broad incident-management subsystem.

Likely files/modules to touch:

- `scripts`: evidence export and guided drill capture.
- `docs`: soak runbook, latest report, mainnet checklist, project state, backlog, runbook, and README.
- Application code only if a real drill exposes a bug in execution truthfulness, duplicate suppression, restart repair, reconnect repair, kill switch, cancel, flatten, or UI status.

Tests required:

- Shell syntax checks for drill scripts.
- Full existing automated gate remains green.

## Phase 4B — Post-Mainnet-Canary Audit Slice (Complete)

Goal: Review the successful bounded MAINNET canary evidence, confirm the safe-default post-canary state, and prepare only the next bounded mainnet-readiness step.

Status: Completed for `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`. The audit scoped canary order/fill evidence to the single MAINNET order, preserved all-recent exports separately, confirmed no canary fill or open BTCUSDT position, confirmed the server canary flag is disabled by default, and kept MAINNET auto blocked.

Non-goals: No second MAINNET order without an explicit operator request, no broad mainnet enablement, no MAINNET auto-execution, no conditional/algo orders, no symbol-scope widening, and no liquidation heatmap/liquidation-context module.

Likely files/modules to touch:

- `docs`: latest canary report, checklist, project state, backlog, runbook, and safety docs.
- `artifacts`: canary evidence annotation or scoping only when it improves audit clarity without rerunning dangerous actions.
- Application code only if the audit exposes a direct canary safety defect.

Tests required:

- Script syntax checks for evidence tooling.
- Safe-default smoke checks for health, bootstrap, live status, masked credentials, and static frontend.
- Secret-scan evidence/repository surfaces for raw credential values, excluding the local `.env` source file itself.

## Phase 4C — Second Canary Readiness Dry-Run Slice (Complete)

Goal: Prepare for a possible second manual MAINNET canary by validating current state, rebuilding a fresh preview, and exporting evidence without submitting any order.

Status: Completed for `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`. The dry-run selected and validated `env-mainnet`, refreshed mainnet readiness/shadow, confirmed no open BTCUSDT mainnet order and no previous canary fill, exercised kill-switch engage/release, kept the server canary flag disabled, and built a fresh non-marketable `BUY LIMIT BTCUSDT 0.001 @ 77800` preview.

Non-goals: No real MAINNET order submission, no canary flag enablement, no MAINNET auto-execution, no conditional/algo orders, no symbol-scope widening, and no liquidation heatmap/liquidation-context module.

Tests required:

- Script syntax checks for evidence tooling.
- Safe-default smoke checks for health, bootstrap, live status, masked credentials, and static frontend.
- Evidence/repository secret scan for raw credential values, excluding the local `.env` source file itself.

## Phase 4D — Second Manual Mainnet Canary Execution Slice (Complete)

Goal: Execute one additional bounded manual MAINNET canary after the dry-run gates pass, then cancel, reconcile, restart-repair, and disable the server canary flag.

Status: Completed for `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`. Exactly one `BUY LIMIT BTCUSDT 0.001 @ 77800` MAINNET order submitted with ACK, canceled, reconciled with `executed_qty=0.000`, left the account flat, passed restart repair, and ended with `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`. MAINNET auto remained blocked.

Non-goals: No broader mainnet enablement, no MAINNET auto-execution, no conditional/algo orders, no symbol-scope widening, and no liquidation heatmap/liquidation-context module.

Follow-up completed: The cancel endpoint body ergonomics issue is fixed. The first cancel request in the second canary had the order reference in the route path and exact confirmation text in the body, but the body omitted `order_ref`; the route treated it as absent and returned `mainnet_confirmation_missing`. Retrying with `order_ref` duplicated in the body canceled successfully. `POST /api/live/orders/:order_ref/cancel` now uses the path as authoritative, accepts omitted or matching optional body `order_ref`, rejects mismatches, and keeps confirmation gates intact.

## Phase 4E — Mainnet Canary Closure And Operator Handoff Slice (Complete)

Goal: Close the current mainnet-canary phase without submitting any new order, align final status docs, audit the major evidence bundles, and provide an operator handoff.

Status: Completed as documentation and smoke validation. `docs/OPERATOR_HANDOFF.md` now defines safe startup, env credential verification, status/order/fill inspection, evidence bundle locations, canary re-run prerequisites, and rollback/stop notes. The real TESTNET soak bundle, first MAINNET canary bundle, and second MAINNET canary bundle were reviewed. The older TESTNET soak bundle predates the newer canary evidence shape but contains masked credential metadata, order/fill exports, timeline, live-status before/after, repair events, and session summary.

Non-goals: No order submission, no canary flag enablement, no MAINNET auto-execution, no conditional/algo orders, no symbol-scope widening, and no liquidation heatmap/liquidation-context module.

Exit criteria:

- Final live-state smoke confirms safe defaults and no unexpected MAINNET exposure.
- Evidence bundles remain truthful and secret-safe.
- Cancel endpoint fix remains documented as a post-canary follow-up, not a rewritten historical result.
- Operator handoff doc exists.

Rollback/fail-closed behavior: Keep `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`, stop the server if uncertain, engage kill switch if a live session is running, verify open orders and flat position from exchange-authoritative status, and preserve evidence.

## Smallest Safe Next Implementation Batch

## Phase 5 — Mainnet Auto Infrastructure And Dry-Run Slice (Current)

Goal: Prepare MAINNET auto for a future controlled live trial without starting live auto or submitting any order.

Status: Implemented as infrastructure and exercised in dry-run on the operator DB. RelXen has typed mainnet-auto config gates, persisted risk budget/state/decision/watchdog/lesson metadata, headless status and dry-run APIs, live-start fail-closed blocking, evidence export, and lesson report generation. Dry-run mode records ASO closed-candle decision outcomes, blockers, risk budget, reference price context, watchdog state, and lessons under `artifacts/mainnet-auto/<timestamp>/`. The operator-DB dry-run evidence is `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`; it recorded `dry_run_would_submit`, empty `orders.json` / `fills.json`, and blocked live start.

Non-goals: No live MAINNET auto run, no TESTNET or MAINNET order submission, no conditional/algo orders, no heatmap/liquidation module, no new strategy indicator, no symbol-scope widening.

Likely files/modules touched:

- `crates/domain`: mainnet-auto state/config/risk/decision/watchdog/lesson models.
- `crates/app`: dry-run supervisor, fail-closed live-start gate, evidence and lesson generation.
- `crates/infra`: SQLite persistence for non-secret mainnet-auto metadata.
- `crates/server`: mainnet-auto status/dry-run/live-block/risk/decision/lesson/evidence APIs and config parsing.
- `web`: compact mainnet-auto status surface and dry-run controls.
- `scripts`: headless dry-run/status/evidence helpers.
- `docs`: runbook, lessons guide, project state, and safety docs.

Tests required:

- MAINNET auto disabled by default.
- Live start blocked without explicit config and gates.
- Dry-run starts/stops and records decisions without invoking order submission.
- Duplicate closed-candle signals are suppressed from persisted decision history.
- Risk-budget/status/decision/lesson/evidence APIs return typed state.
- Frontend shows MAINNET auto blocked/dry-run state without implying live execution.
- Script syntax and full existing gate remain green.

Exit criteria:

- Mainnet auto live mode remains default-off.
- Dry-run evidence and lessons can be exported.
- No live order is submitted.
- Existing TESTNET/manual MAINNET canary behavior remains intact.

Rollback/fail-closed behavior: Set `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false`, stop dry-run, engage kill switch if a live session is otherwise active, and inspect status/evidence before any future live attempt.

## Smallest Safe Next Implementation Batch

Review `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/` and prepare a separate explicit live-auto plan only if the operator wants to continue. If a future live trial is ever requested, it must be a separate explicit batch with fresh gates and evidence review.

## What Not To Do Next

- Do not enable broad mainnet operation beyond the existing manual canary gate.
- Do not store secrets in SQLite or frontend storage.
- Do not turn the paper engine into live reconciliation truth.
- Do not run MAINNET canary without the current real TESTNET evidence bundle and updated checklist.
- Do not use the TESTNET drill helper in a MAINNET session.
- Do not expand to multiple symbols while live boundaries are still immature.

## Avoiding An Endless Rewrite

Keep paper mode stable. Add live readiness behind new ports and typed state instead of rewriting ASO, history loading, charting, or the existing paper engine. Each phase must have a small exit criterion and must preserve the ability to run the current paper dashboard unchanged.
