# Latest Mainnet Canary Report

## Report Status

Status: GO after reference-price freshness hardening and one guarded MAINNET canary.

Date: 2026-04-24

Evidence bundle: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`

Credential source: env-backed `env-mainnet`, masked only.

MAINNET canary server gate was enabled only after reference-price hardening tests, leverage, balance, local-risk, and initial-preview gates passed. MAINNET auto-execution remained blocked. Exactly one manual MAINNET `LIMIT` canary was submitted, canceled, reconciled, and restart-repair checked. The server canary flag was disabled afterward.

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

## Final Verdict

GO for this bounded manual MAINNET canary.

Exactly one MAINNET canary order was submitted, canceled, and reconciled. No fill occurred, no flatten was needed, restart repair passed, MAINNET auto remained blocked, and the canary flag was disabled afterward.

Exact next task: review the canary evidence bundle and decide whether to keep the recommendation at bounded manual-canary GO or defer broader mainnet enablement design.
