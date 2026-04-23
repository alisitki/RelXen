# Latest Testnet Soak Report

## Report Status

Status: Real TESTNET validation attempted; real exchange soak blocked before execution because no TESTNET credential metadata exists in the active local API state.

Date: 2026-04-23

Reason real drill was not executed: `/api/live/credentials` returned an empty list, `/api/live/status` reported `credentials_missing`, `active_credential=null`, and `mainnet_canary.enabled_by_server=false`. Without an operator-provided TESTNET credential in the secure-store/metadata flow, RelXen cannot validate credentials, start shadow sync, build executable readiness, or place TESTNET orders.

## Evidence Produced In This Batch

- Evidence export tooling: `scripts/export_live_evidence.sh`
- Guided operator capture wrapper: `scripts/run_testnet_soak.sh`
- Testnet drill procedure: `docs/TESTNET_SOAK_RUNBOOK.md`
- Mainnet canary checklist: `docs/MAINNET_CANARY_CHECKLIST.md`
- Real-validation blocked export generated ignored artifact path: `artifacts/testnet-soak/real-validation-blocked-20260423T1424Z/`
- The evidence bundle includes `credentials.json` with masked credential summaries only; this run exported zero credentials.

## Smoke Verification

- `/api/health` returned `ok`.
- `/api/bootstrap` returned 500 candles for `BTCUSDT`.
- `/api/live/credentials` returned `[]`.
- `/api/live/status` returned `credentials_missing`, `execution_blocked`, `active_credential=null`, and `mainnet_canary.enabled_by_server=false`.
- Static frontend serving at `/` returned HTTP 200.
- Evidence export produced manifest, status, masked credential summaries, bootstrap, orders, fills, preflights, blocking reasons, repair events, logs, timeline, and session summary files.

This export is not a real exchange drill because no TESTNET credential was available.

## Scenario Results

| Scenario | Result | Evidence |
| --- | --- | --- |
| Credential / readiness / shadow bootstrap | Blocked before exchange call | `/api/live/credentials=[]`; status `credentials_missing` |
| Manual preview + preflight sanity | Not exercised for real | Requires valid TESTNET credential and readiness |
| Real TESTNET manual execution | Not exercised for real | Requires valid TESTNET credential |
| Cancel flow | Not exercised for real | Requires a working TESTNET order |
| Flatten flow | Not exercised for real | Requires deterministic TESTNET position |
| Kill switch | Covered by existing automated tests; not real-drill exercised | API/app/frontend tests from executor hardening |
| Restart / recent-window repair | Covered by existing automated tests; not real-drill exercised | App/server persistence and repair tests from executor hardening |
| Reconnect / repair | Covered by existing automated tests; not real-drill exercised | Shadow/recovery tests from executor hardening |
| Auto-executor proof | Covered by existing automated tests; not real-drill exercised | Closed-candle auto-executor tests from mainnet-readiness hardening |
| Recent-window repair honesty | Documented | Runbook and live-risk docs |

## Bugs Found And Fixed

- No executor bug was found because the drill could not progress past credential discovery.
- Improved evidence export to include masked credential summaries so missing-credential blockers are auditable without exposing secrets.

## Bugs Deferred

- None identified in this batch.

## Current Mainnet Canary Recommendation

NO-GO for a real MAINNET manual canary until a real TESTNET soak evidence bundle is captured and reviewed.

The codebase has default-off canary gates, risk-profile requirements, kill switch controls, ACK-plus-authoritative-reconciliation, dedicated account-mode checks, forced user-data reconnect, and recent-window repair policy. Operational evidence is still required before recommending a real mainnet canary session.

## Preconditions For A Future Go

- Run `docs/TESTNET_SOAK_RUNBOOK.md` with valid TESTNET credentials.
- Capture an evidence bundle with `scripts/run_testnet_soak.sh` or `scripts/export_live_evidence.sh`.
- Demonstrate at least one real TESTNET execution lifecycle.
- Demonstrate kill switch blocks new submissions.
- Demonstrate restart repair does not duplicate orders.
- Demonstrate reconnect repair recovers or truthfully degrades.
- Demonstrate cancel and flatten when market conditions make them applicable, or document why an immediate fill prevented cancel.
- Review `docs/MAINNET_CANARY_CHECKLIST.md` and satisfy every hard precondition.
