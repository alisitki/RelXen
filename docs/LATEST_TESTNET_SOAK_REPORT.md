# Latest Testnet Soak Report

## Report Status

Status: Real TESTNET soak completed with operator-provided credentials, secret-safe evidence export, and targeted drill-blocking fixes.

Date: 2026-04-23

Evidence bundle: `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`

Env credential validation addendum: `artifacts/testnet-soak/20260424T061338Z-env-credential-validation/`

Active credential summary during the run:

- alias: `codex-testnet-20260423`
- environment: `testnet`
- api key hint: `jl9F…drYl`

MAINNET remained disabled throughout the run: `/api/live/status.mainnet_canary.enabled_by_server=false`.

On 2026-04-24, env-backed TESTNET credentials were loaded from local `.env`, selected as `env-testnet` without secure-store prompts, validated successfully, refreshed through readiness/shadow, and exported with masked credential metadata only.

## What Was Exercised For Real

- TESTNET credential creation, selection, and validation through the existing secure-store flow.
- Live readiness refresh and shadow sync startup.
- Precision-aware preview build and Binance testnet `order/test` preflight.
- Real TESTNET manual `MARKET` execution with ACK-first handling and later authoritative fill state.
- Real TESTNET manual `LIMIT` execution kept working long enough to exercise real cancel.
- Real TESTNET flatten from a deterministic open position.
- Kill switch engage and release.
- Restart against the same SQLite database with bounded recent-window execution repair.
- Shadow stop/start and manual shadow refresh as reconnect/recovery evidence.
- TESTNET auto-executor proof and duplicate-signal suppression proof through a TESTNET-only, default-off drill helper because no natural closed-candle crossover appeared during the bounded window.

## What Was Not Exercised As A Natural Market Event

- A natural fresh closed-candle crossover did not arrive during the bounded drill window.
- The auto path was therefore exercised through the explicit drill helper at `/api/live/drill/auto/replay-latest-signal` with `RELXEN_ENABLE_TESTNET_DRILL_HELPERS=true`.
- That helper is TESTNET-only, off by default, requires explicit confirmation, and replays the latest persisted closed signal through the existing auto-execution path instead of creating a synthetic order path.

## Scenario Results

| Scenario | Result | Evidence |
| --- | --- | --- |
| Credential / readiness / shadow bootstrap | Pass | Credential created/validated; shadow sync running; readiness coherent |
| Manual preview + preflight sanity | Pass | Real preview built; `PREFLIGHT PASSED. No order was placed.` |
| Real TESTNET manual execution | Pass | Real `MARKET` buy submitted; ACK captured separately from eventual fill |
| Cancel flow | Pass | Real `LIMIT` buy at `72000` entered `working` then `canceled` |
| Flatten flow | Pass | Real flatten used reduce-only `MARKET` close; final position returned flat |
| Kill switch | Pass | Engage blocked new submissions; release required normal readiness recovery |
| Restart / recent-window repair | Pass after fix | Restart preserved recent orders/fills; bounded repair reconciled recent execution without duplicate submission |
| Reconnect / repair | Pass after fix | Shadow stop/start and manual refresh recovered truthfully; repair remained bounded recent-window only |
| Auto-executor proof | Pass via drill helper | First replay submitted exactly one real TESTNET auto order |
| Duplicate-signal suppression | Pass | Second replay of the same persisted closed signal was suppressed with `duplicate_signal_suppressed` |
| Recent-window repair honesty | Pass | Repair remained bounded; no infinite recovery claims were made |

## Real Execution Timeline Summary

1. Credential created and validated through the live credential API.
2. Live mode switched to read-only, risk profile configured conservatively, live mode armed, and shadow sync started.
3. Real preview built for `BTCUSDT`; preflight passed with no order placement.
4. Real TESTNET manual `MARKET` buy submitted with ACK handling, then later reconciled to `filled`.
5. Real TESTNET manual flatten closed the resulting position.
6. Real TESTNET `LIMIT` buy at `72000` stayed working long enough to cancel, and final state reconciled to `canceled`.
7. Kill switch engaged and released successfully.
8. Restart preserved recent state, then repair and shadow restart recovered coherent status.
9. TESTNET auto mode ran; because no natural bounded-window signal appeared, the explicit drill helper replayed the latest persisted closed signal once, creating one real TESTNET auto order.
10. A second replay of the same signal was suppressed as a duplicate.
11. Restart plus manual shadow refresh repaired the auto order and repaired fills.
12. Final flatten returned the exchange shadow position to flat, auto mode was stopped, and the final evidence export was captured.

## Bugs Found And Fixed In This Batch

1. Visible account snapshots could stay stale after shadow refresh, hiding a real non-zero exchange position after a filled TESTNET order.
   Fix: visible live status now derives the account snapshot from the freshest shadow state when available.

2. Manual shadow refresh did not also perform bounded recent-window execution repair.
   Impact: a real TESTNET auto-submitted order could stay stuck at `accepted` after restart/reconnect even though the exchange position was already open.
   Fix: `refresh_live_shadow()` now runs the existing bounded recent-window repair path after refreshing shadow state.

3. Recent-window repaired fills did not backfill local `order_id` / `client_order_id` when the authoritative exchange trade could be matched to a repaired order.
   Impact: repaired fills were harder to audit locally after restart/repair.
   Fix: recent-window repair now enriches repaired fills with matched local order references.

4. No natural bounded-window closed-candle crossover appeared for auto proof.
   Fix: added a TESTNET-only, default-off drill helper that replays the latest persisted closed signal through the existing auto-execution path when explicitly enabled for a soak session.

## Bugs Deferred

- None considered drill-blocking after the fixes above.
- A future production-facing auto soak should still prefer a natural crossover over the drill helper whenever the market gives one within the bounded window.

## Current Mainnet Canary Recommendation

GO for the bounded manual MAINNET canary path that was exercised on 2026-04-24. Broader mainnet operation remains out of scope. MAINNET auto now has dry-run/status/evidence infrastructure, but live MAINNET auto remains disabled by default and requires a separate explicit live-run task.

Rationale:

- Real TESTNET evidence now exists for credential validation, readiness, shadow sync, preview, preflight, manual execution, cancel, flatten, kill switch, restart repair, reconnect repair, and duplicate-safe auto submission.
- Env-backed TESTNET credential validation now exists without OS secure-storage prompts.
- A guarded MAINNET canary retry on 2026-04-24 hardened reference-price freshness, forced a fresh REST mark-price-backed final preview after the kill-switch drill, submitted one non-marketable `BTCUSDT` `LIMIT` canary, canceled it, reconciled flat with no fill, passed restart repair, and disabled the canary flag afterward. Evidence: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`.
- Mainnet remains default-off and no hidden bypass was used.
- MAINNET auto live execution remains blocked; use dry-run evidence only until a separate explicit live task.

## Exact Preconditions For Safe Manual MAINNET Canary

- Keep `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` until the canary session starts.
- Use a validated mainnet credential from OS secure storage or the env-backed `env-mainnet` summary.
- Confirm mainnet available quote balance is sufficient for the smallest exchange-compliant non-marketable `LIMIT` preview, exchange min quantity does not force notional above the approved canary cap, active-symbol exchange leverage is no greater than the approved maximum, and the preview remains fresh after kill-switch release.
- Configure and review a conservative risk profile before arming.
- Start fresh shadow sync and verify one-way mode plus single-asset mode.
- Confirm the active symbol is `BTCUSDT` or `BTCUSDC`.
- Confirm no active paper position or ambiguous live order state exists.
- Build one small preview only.
- Enter the exact MAINNET confirmation text for that preview only.
- Submit one manual canary order only.
- Capture a new evidence bundle immediately after the canary action.
- Do not use the TESTNET drill helper in any MAINNET session.

## Commands / Surfaces Used During The Real Run

- `POST /api/live/credentials`
- `POST /api/live/credentials/:credential_id/select`
- `POST /api/live/credentials/:credential_id/validate`
- `POST /api/live/mode`
- `PUT /api/live/risk-profile`
- `POST /api/live/readiness/refresh`
- `POST /api/live/arm`
- `POST /api/live/shadow/start`
- `GET /api/live/intent/preview`
- `POST /api/live/preflight`
- `POST /api/live/execute`
- `POST /api/live/orders/:order_ref/cancel`
- `POST /api/live/flatten`
- `POST /api/live/kill-switch/engage`
- `POST /api/live/kill-switch/release`
- `POST /api/live/auto/start`
- `POST /api/live/auto/stop`
- `POST /api/live/drill/auto/replay-latest-signal`
- `scripts/export_live_evidence.sh`

No raw secrets were exported in the evidence bundle, docs, or report.
