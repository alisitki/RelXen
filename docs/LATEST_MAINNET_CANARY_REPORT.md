# Latest Mainnet Canary Report

## Report Status

Status: GO for mainnet-canary closure / operator handoff; MAINNET auto dry-run infrastructure and one no-order live trial were added afterward with live mode still disabled by default.

Date: 2026-04-24

Primary first-canary evidence bundle: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`

Second-canary evidence bundle: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`

Credential source: env-backed `env-mainnet`, masked only.

MAINNET canary server gate was enabled only after reference-price hardening tests, leverage, balance, local-risk, and initial-preview gates passed. MAINNET auto-execution remained blocked during the canary phase. Exactly one manual MAINNET `LIMIT` canary was submitted, canceled, reconciled, and restart-repair checked. The server canary flag was disabled afterward. MAINNET auto now has dry-run/status/evidence infrastructure and a gated session-scoped live path; the first later live-auto trial submitted no order and recorded no fill, and live MAINNET auto remains default-off.

## Outcome

One MAINNET canary order was submitted and canceled.

The retry reached explicit MAINNET env credential selection and validation, fresh mainnet readiness, one-way/single-asset account checks, mainnet auto block verification, mainnet shadow refresh, existing smallest exchange-compliant canary profile, a fresh non-marketable `LIMIT` preview, and the kill-switch engage/release drill.

`BTCUSDT` leverage reported `5x`. Mainnet account/shadow reported available `USDT=25.0902305` and available `USDC=0`. Active symbol was `BTCUSDT`, min quantity was `0.001`, min notional was `50.0`, and the final non-marketable `SELL LIMIT` preview at `77950` produced quantity `0.001`, estimated notional `77.95`, required margin `15.59`, and local preview leverage `5`. After the required kill-switch engage/release drill, the final preview forced a fresh REST mark-price refresh: source `rest_mark_price`, reference price `77444.60000000`, age `128 ms`, rounded order price `77950`, and non-marketable result `false` for marketability. The canary order was submitted with ACK, canceled, reconciled to `canceled`, and had `executed_qty=0.000`.

## Gate Results

| Gate | Result |
| --- | --- |
| Env mainnet credential visible only as masked summary | Pass |
| Env mainnet credential explicitly selected | Pass |
| Env mainnet credential validated | Pass |
| Mainnet canary flag default-off | Pass |
| Mainnet auto-execution blocked | Pass |
| One-way position mode | Pass |
| Single-asset margin mode | Pass |
| Supported symbol `BTCUSDT` | Pass |
| Fresh mainnet rules/account/shadow | Pass |
| Shadow environment matches active environment | Pass |
| Conservative canary risk profile | Pass: local settings `fixed_notional=78`, `leverage=5`; risk max notional `78`, max leverage `5`, one order |
| Available quote balance | Pass: available `USDT=25.0902305`, preview required margin `15.59` before fee/buffer |
| Approved notional cap | Pass: smallest exchange-compliant `BTCUSDT` preview estimated notional `77.95` under max `78` |
| Exchange/account leverage observed | Pass: fresh account snapshot reported `BTCUSDT` leverage `5` |
| Initial non-marketable `LIMIT` preview | Pass: `SELL LIMIT BTCUSDT 0.001 @ 77950` |
| Kill switch engage/release proof | Pass: engage blocked execution; release completed |
| Final fresh reference price | Pass: REST mark price source, age `128 ms` |
| Order submission | Pass: one ACK for `SELL LIMIT BTCUSDT 0.001 @ 77950` |
| Cancel | Pass: order reconciled to `canceled` |
| Fill | None; executed quantity `0.000` |
| Flatten | Not needed; BTCUSDT position remained flat |
| Restart repair | Pass: same order remained `canceled`; no duplicate order submitted |

## Bugs Found And Fixed

Reference-price hardening was required and implemented before this retry:

1. MAINNET preview and final submit now resolve reference price through an explicit environment/symbol-aware resolver.
2. Fresh internal market state is preferred when valid; stale or missing state falls back to Binance USD-M REST mark price.
3. Final MAINNET submit forces a fresh reference-price refresh and blocks with typed reference-price blockers if refresh fails or is stale.
4. Preview/evidence now include reference price, source, age, rounded order price, and marketability result.
5. MAINNET cancel response text was corrected to avoid saying TESTNET for a MAINNET canary cancel.

Previously fixed issues remain relevant:

1. Env source startup could still use a persisted secure-store TESTNET active credential before env selection.
   Fix: authoritative `RELXEN_CREDENTIAL_SOURCE=env` now selects `env-testnet` at startup ahead of persisted secure-store TESTNET selection, while MAINNET env credentials still require explicit selection.

2. MAINNET readiness could inherit stale TESTNET shadow/stream metadata after switching environments.
   Fix: shadow stream/shadow environment mismatch now blocks readiness, previews, and execution; shadow refresh/reconnect/event paths write the active environment.

## Post-Canary Audit

The post-canary audit reviewed `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/` without submitting another order.

Evidence audit result:

- Required evidence files are present, including manifest, session summary, timeline, before/after status snapshots, masked credentials, account/position/open-order snapshots, risk profile, reference price, kill-switch events, repair events, and final verdict.
- `orders.json` was scoped during audit to the single MAINNET canary order: `rx_exec_405cc0ab8c914df29369f008`, status `canceled`, executed quantity `0.000`.
- `fills.json` was scoped during audit to canary-linked fills and is empty.
- The original generic recent export is preserved as `orders_all_recent.json` and `fills_all_recent.json`; those files include previous TESTNET records and are not used as canary fill proof.
- `cancel_result.json` contains a raw message captured before the MAINNET cancel wording fix and says TESTNET. Authoritative order records, exchange order id `994615724156`, environment fields, and reconciliation records show the canceled order was MAINNET. The code wording was corrected after that capture.
- A secret scan of the evidence bundle and repository surfaces, excluding the local `.env` source file itself, found no raw env API keys or secrets.
- `.env` remains gitignored and untracked.

Post-canary live-state result from a safe-default server run:

- `/api/health`, `/api/bootstrap`, `/api/live/credentials`, `/api/live/status`, and `/` responded.
- `env-mainnet` was visible as masked metadata only and validated.
- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` kept `mainnet_canary.enabled_by_server=false`.
- Execution state was `mainnet_execution_blocked` with blockers `intent_unavailable` and `mainnet_canary_disabled`.
- Account/shadow refresh showed `BTCUSDT` position amount `0`, available `USDT=25.0902305`, one-way mode, single-asset mode, and `BTCUSDT` leverage `5`.
- Local recent order state showed the one MAINNET canary order as `canceled`; no canary-linked fills were present.

## Second Canary Readiness Dry-Run

Evidence bundle: `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`

This was a readiness dry-run only. No real MAINNET order was submitted, no new order was canceled, and no flatten was needed. The server was started with `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`; MAINNET auto stayed stopped/blocked.

Dry-run result:

- `env-mainnet` appeared only as masked metadata, was explicitly selected, and validated successfully.
- Mainnet readiness and shadow were refreshed with active symbol `BTCUSDT`.
- Account mode remained one-way and single-asset.
- BTCUSDT leverage remained `5x`.
- Available `USDT=25.0902305`.
- Previous canary order `rx_exec_405cc0ab8c914df29369f008` remained `canceled` with `executed_qty=0.000`.
- No active BTCUSDT mainnet order was present in local recent order state.
- Kill switch engage/release was exercised before the final dry-run preview.
- Current closed signal side was `BUY`, so the final dry-run preview was `BUY LIMIT BTCUSDT 0.001 @ 77800`, not the prior canary's `SELL @ 77950`.
- Final reference/marketability diagnostics: reference `78294.8`, source `internal_market_candle`, age `25046 ms`, rounded order price `77800`, marketable after rounding `false`.
- Sizing: requested notional `77.8`, estimated notional `77.8`, required margin `15.56`, leverage `5`, available balance `25.0902305`.
- Mainnet preflight was locally blocked as unsupported on mainnet; no exchange preflight or order request was sent.

Dry-run recommendation: CONDITIONAL GO for a separately requested second manual canary execution batch. The dry-run does not authorize automatic execution and does not change the default-off mainnet posture.

## Second Canary Execution

Evidence bundle: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`

Status: GO after follow-up cancel endpoint ergonomics fix. The evidence run itself remains truthful: the first cancel request failed due to payload shape, then a retry canceled the same order.

Exactly one second MAINNET canary order was submitted. MAINNET auto stayed stopped/blocked, and the server canary flag was disabled again after restart-repair.

Execution result:

- Credential: env-backed `env-mainnet`, masked only.
- Active symbol: `BTCUSDT`.
- Order: `BUY LIMIT BTCUSDT 0.001 @ 77800`.
- Reference price: `78201.50864493`.
- Reference source: `rest_mark_price`.
- Reference age at preview: `82 ms`.
- Marketable after rounding: `false`.
- Requested/estimated notional: `77.8`.
- Required margin: `15.56`.
- Available `USDT`: `25.0902305`.
- Exact submit confirmation: `SUBMIT MAINNET BUY LIMIT BTCUSDT 0.001 @ 77800`.
- Submit result: ACK accepted, exchange order id `994751734783`.
- Cancel result: final retry accepted and order reconciled to `canceled`.
- Executed quantity: `0.000`.
- Fill: none.
- Flatten: not needed.
- Final BTCUSDT position: flat.
- Final open BTCUSDT mainnet orders: none.
- Restart repair: passed; the second canary order remained `canceled` and no duplicate order was submitted.
- Canary flag after restart: disabled.

Minor non-safety issue:

- The first cancel request included the exact confirmation text but omitted `order_ref` inside the JSON body because the order reference was already present in the route path. The then-current server route treated that body as absent and returned `mainnet_confirmation_missing`. Retrying the same order with `order_ref` duplicated in the JSON body canceled successfully. No additional order was submitted and no fill occurred.

Follow-up fix: `POST /api/live/orders/:order_ref/cancel` now uses the path `order_ref` as authoritative. Body `order_ref` is optional for compatibility, matching body `order_ref` is accepted, and mismatched body `order_ref` is rejected as a validation error. Mainnet and testnet confirmation requirements remain intact.

Post-fix smoke:

- Server started with `RELXEN_CREDENTIAL_SOURCE=env`, `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`, and `RELXEN_AUTO_START=false`.
- `/api/health`, `/api/bootstrap`, `/api/live/status`, `/api/live/credentials`, and `/` responded.
- `.env` remained gitignored and untracked.
- `env-mainnet` selected and validated as masked metadata only for read-only smoke checks.
- MAINNET canary remained disabled by server policy and MAINNET auto remained stopped.
- Previous MAINNET `BTCUSDT` orders `rx_exec_405cc0ab8c914df29369f008` and `rx_exec_876038f71d1e479c9fc68831` remained `canceled` with `executed_qty=0.000`.
- No MAINNET `BTCUSDT` fills were returned by `/api/live/fills`.
- Account snapshot showed BTCUSDT position amount `0`.
- Raw env secrets were not found in captured smoke API payloads.

Recommendation: GO for the cancel API ergonomics fix. Do not submit another canary automatically; broader mainnet operation remains out of scope.

## Closure Audit

Closure review inspected:

- TESTNET soak bundle: `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`
- First MAINNET canary bundle: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`
- Second MAINNET canary bundle: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`

Evidence result:

- TESTNET soak bundle includes manifest, session summary, timeline, live status before/after, masked credential metadata in `credentials.json`, orders, fills, repair events, logs, and preflights. It predates the newer canary evidence layout, so it does not have `credentials_masked.json`, per-snapshot before/after files, or `final_verdict.json`.
- First MAINNET canary bundle includes the required canary evidence files, one scoped MAINNET order, empty scoped fills, account/position/open-order snapshots, reference price, kill-switch events, repair events, and final verdict.
- Second MAINNET canary bundle includes the required canary evidence files, one scoped MAINNET order, empty fills, account/position/open-order snapshots, reference price, kill-switch events, repair events, final verdict, and the truthful failed-first-cancel / successful-retry cancel records.
- Raw env secret scan over docs, code, scripts, and the three reviewed evidence bundles passed.
- `.env` remains gitignored and untracked.

Final live-state smoke:

- Server started with `RELXEN_CREDENTIAL_SOURCE=env`, `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`, and `RELXEN_AUTO_START=false`.
- `/api/health`, `/api/bootstrap`, `/api/live/status`, `/api/live/credentials`, and `/` responded.
- `env-mainnet` validated successfully for read-only status checks.
- `mainnet_canary.enabled_by_server=false` and `mainnet_canary.manual_execution_enabled=false`.
- MAINNET auto state remained `stopped`.
- MAINNET BTCUSDT order `rx_exec_405cc0ab8c914df29369f008` remained `canceled` with `executed_qty=0.000`.
- MAINNET BTCUSDT order `rx_exec_876038f71d1e479c9fc68831` remained `canceled` with `executed_qty=0.000`.
- No MAINNET BTCUSDT fills were returned.
- BTCUSDT account snapshot position amount was `0`.
- Raw env secrets were absent from captured smoke API payloads.

Operator handoff:

- `docs/OPERATOR_HANDOFF.md` now records safe startup, env credential verification, mainnet disabled checks, order/fill inspection, evidence paths, canary re-run prerequisites, never-do items, and rollback/stop notes.
- `docs/FINAL_RC_SNAPSHOT.md` records the release-candidate cleanup result, evidence artifact policy, git/output hygiene, safe startup posture, test/build gate status, known risks, and the exact next bounded task.

## Mainnet Auto Infrastructure Follow-Up

MAINNET auto infrastructure v1 was added after canary closure without submitting any new order. The new surface is dry-run first:

- `/api/live/mainnet-auto/status`
- `/api/live/mainnet-auto/dry-run/start`
- `/api/live/mainnet-auto/dry-run/stop`
- `/api/live/mainnet-auto/start`
- `/api/live/mainnet-auto/stop`
- `/api/live/mainnet-auto/decisions`
- `/api/live/mainnet-auto/lessons/latest`
- `/api/live/mainnet-auto/risk-budget`
- `/api/live/mainnet-auto/export-evidence`

Live start remains fail-closed by default because `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false` and `RELXEN_MAINNET_AUTO_MODE=dry_run`. Dry-run decisions may record would-submit/blocker outcomes and lesson reports, but they do not call the exchange order endpoint and do not authorize live trading.

The first credential-selected operator-DB dry-run is `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`. It selected and validated `env-mainnet`, refreshed mainnet readiness/shadow, used dry-run budget `mainnet-auto-operator-dry-run-v1`, recorded `dry_run_would_submit`, generated lessons, verified live start remained `config_blocked`, and submitted no order.

Mainnet Auto Live Support v1 was implemented afterward for explicit `BTCUSDT` 15-minute `MARKET` sessions with exact session confirmation and watchdog/risk gates. It was tested with mocked adapters, then exercised once on 2026-04-25. Session `mnauto_live_39b61e12f8084f669b334420a3f105ac` stopped at `max_runtime_reached`, observed zero signals, submitted no order, recorded no fill, and ended flat. Evidence: `artifacts/mainnet-auto/1777099647957-mnauto_live_39b61e12f8084f669b334420a3f105ac/`.

Mainnet Auto Policy Support v1 was added after that no-order live trial without submitting any new order. It makes cross/isolated margin type an explicit gate (`isolated` default, `cross` only when explicitly allowed, `unknown` blocked) and exposes ASO position policy modes `crossover_only`, `always_in_market`, and `flat_allowed` for a future explicit live-auto batch.

## Final Verdict

GO for mainnet-canary closure / operator handoff.

Two bounded manual MAINNET canary orders were submitted across separate sessions, canceled, and reconciled. No fill occurred, no flatten was needed, restart repair passed, MAINNET auto remained blocked, and the canary flag was disabled afterward.

Exact next task: prepare a separate explicit live-auto plan only if the operator wants to continue. The dry-run result is not live-auto approval.
