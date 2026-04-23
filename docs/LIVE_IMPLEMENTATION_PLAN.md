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

Status: Implemented in Live Foundations v0. Raw secrets are kept behind the secret-store abstraction; SQLite stores masked metadata only.

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

- Credentials can be saved to OS secure storage.
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

## Phase 3C — Strategy-Driven Testnet Auto-Executor And Kill Switch Slice (Next)

Goal: Add strategy-driven closed-candle TESTNET auto-execution behind explicit arming, duplicate-signal suppression, kill switch, and exchange-authoritative reconciliation.

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

## Phase 4 — Constrained Mainnet Slice

Goal: Enable a tightly constrained mainnet path only after prior evidence is green.

Non-goals: No broad exchange feature set, no multi-symbol runtime, no unattended operation claims.

Likely files/modules to touch:

- Existing live credential, rules, account, execution, reconciliation, risk, server, and UI modules from Phases 1-3.

Tests required:

- Full fake-exchange regression suite.
- Testnet evidence runbook.
- Mainnet dry-run preflight tests.
- Explicit operator confirmation tests.
- Failure and kill-switch drills.

Exit criteria:

- Mainnet arming requires explicit operator confirmation.
- Conservative limits are enforced.
- Reconciliation is exchange-authoritative.
- Rollback to paper-only is documented and tested.

Key risks: Real financial loss, operator confusion, stale account state, exchange-side rule changes.

Rollback/fail-closed behavior: Kill switch, disarm, credential revocation, and paper-only fallback.

## Smallest Safe Next Implementation Batch

Add an explicit kill switch and strategy-driven closed-candle TESTNET auto-executor behind arming, duplicate-signal suppression, and exchange-authoritative fill reconciliation. Do not add mainnet placement.

## What Not To Do Next

- Do not implement mainnet signed order endpoints.
- Do not store secrets in SQLite or frontend storage.
- Do not turn the paper engine into live reconciliation truth.
- Do not add mainnet UI controls before testnet auto-execution, kill-switch drills, and reconciliation evidence are proven.
- Do not expand to multiple symbols while live boundaries are still immature.

## Avoiding An Endless Rewrite

Keep paper mode stable. Add live readiness behind new ports and typed state instead of rewriting ASO, history loading, charting, or the existing paper engine. Each phase must have a small exit criterion and must preserve the ability to run the current paper dashboard unchanged.
