# Operator Handoff

## Current Project Status

RelXen Paper Mode V1 is release-candidate complete. Post-v1 live work has completed real TESTNET validation and two bounded manual MAINNET canaries:

- TESTNET soak evidence: `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`
- First MAINNET canary evidence: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`
- Second MAINNET canary evidence: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`

The first MAINNET canary submitted one `BTCUSDT SELL LIMIT 0.001 @ 77950`, canceled it, recorded `executed_qty=0.000`, and left no position. The second MAINNET canary submitted one `BTCUSDT BUY LIMIT 0.001 @ 77800`, canceled it, recorded `executed_qty=0.000`, and left no position. Restart repair passed after both canaries.

The second canary exposed a cancel payload ergonomics issue. That issue is fixed: `POST /api/live/orders/:order_ref/cancel` now uses the path `order_ref` as authoritative, does not require body `order_ref`, accepts a matching optional body `order_ref`, and rejects a mismatch.

The dashboard RC UI has also been cleaned for review: the top safety strip makes `MAINNET AUTO: BLOCKED`, `MAINNET CANARY: DISABLED`, kill-switch state, active symbol, current state, and position state visible without opening advanced details. MAINNET auto dry-run infrastructure now exists for status, decisions, evidence, and lessons, but live MAINNET auto remains disabled by default.

Mainnet Auto Live Support v1 is implemented for a future explicitly approved 15-minute `BTCUSDT` session, but no real live-auto session has been run. The future start path is server-config-gated, session-confirmed, watchdog-protected, and evidence-logged; it remains blocked unless `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true`, `RELXEN_MAINNET_AUTO_MODE=live`, and exact confirmation `START MAINNET AUTO LIVE BTCUSDT 15M` are supplied for that session.

Operator-start preparation is in place for the 15-minute live trial. `scripts/run_mainnet_auto_live_trial.sh` accepts the explicit batch flags (`--max-leverage 5`, `--max-notional 80`, `--max-session-loss-usdt 5`, `--order-type MARKET`, and `--confirm ...`), verifies the running server is already in live-auto mode, configures the bounded risk budget, and then calls the existing live-start endpoint. It does not alter strategy logic or widen symbol/order scope.

## What Is Safe To Run

Normal safe local operation:

```sh
RELXEN_CREDENTIAL_SOURCE=env \
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false \
RELXEN_AUTO_START=false \
cargo run -p relxen-server
```

Then open `http://localhost:3000/`.

Safe read-only/status surfaces:

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl http://localhost:3000/api/live/status
curl http://localhost:3000/api/live/credentials
curl http://localhost:3000/api/live/orders
curl http://localhost:3000/api/live/fills
curl http://localhost:3000/api/live/mainnet-auto/status
```

Headless mainnet-auto status helpers:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --precheck
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --summary
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --flat-check
```

TESTNET execution remains available only through explicit operator confirmation and normal fail-closed gates. MAINNET canary execution remains disabled unless the explicit server-side canary flag is enabled for a separate canary session.

## What Is Not Enabled

- MAINNET auto live execution must remain blocked unless a later explicit live-auto task enables it. Current safe use is dry-run only.
- The implemented live-auto start path is not an approval to run; use `docs/MAINNET_AUTO_LIVE_TRIAL_PLAN.md` for the future execution checklist.
- Broad MAINNET operation is not enabled.
- Conditional/algo orders are not supported.
- Symbol scope is still `BTCUSDT` / `BTCUSDC`.
- Liquidation heatmap/liquidation context is deferred and must not become a live decision layer.
- `.env` is local operator convenience, not production-grade secret storage.

## MAINNET Auto Dry-Run

MAINNET auto dry-run is the only supported mainnet-auto mode in the current handoff. It can record decision and risk/evidence context without submitting orders:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_mainnet_auto_dry_run.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_mainnet_auto_evidence.sh
```

Expected dry-run truth:

- `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false`
- mode is `dry_run`
- no call to the Binance new-order endpoint
- exported `orders.json` and `fills.json` are empty
- `lessons.md` / `lessons.json` are analysis only and do not change settings

Latest operator-DB dry-run: `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`. It selected/validated `env-mainnet`, refreshed mainnet readiness/shadow, recorded `dry_run_would_submit`, verified live start remained config-blocked, and submitted no order. Treat this as readiness evidence for a future plan only, not live-auto approval.

## Safe Startup Checklist

1. Keep `.env` local only:

```sh
git check-ignore -v .env
git ls-files --error-unmatch .env
```

The first command should show `.env` is ignored. The second command should fail if `.env` is untracked.

2. Start with safe defaults:

```sh
RELXEN_CREDENTIAL_SOURCE=env \
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false \
RELXEN_AUTO_START=false \
cargo run -p relxen-server
```

3. Verify health and static frontend:

```sh
curl http://localhost:3000/api/health
curl -I http://localhost:3000/
```

4. Verify masked credentials only:

```sh
curl http://localhost:3000/api/live/credentials
```

The response must show masked metadata only. It must not contain raw `api_key` or `api_secret` fields.

5. Verify MAINNET canary is disabled:

```sh
curl http://localhost:3000/api/live/status
```

Check:

- `mainnet_canary.enabled_by_server=false`
- `mainnet_canary.manual_execution_enabled=false`
- `auto_executor.state=stopped`

## Inspecting Orders, Fills, And Exposure

Recent orders:

```sh
curl http://localhost:3000/api/live/orders
```

Recent fills:

```sh
curl http://localhost:3000/api/live/fills
```

Live status, including account/shadow state when available:

```sh
curl http://localhost:3000/api/live/status
```

Expected closure truth for the two MAINNET canaries:

- `rx_exec_405cc0ab8c914df29369f008`: canceled, `executed_qty=0.000`
- `rx_exec_876038f71d1e479c9fc68831`: canceled, `executed_qty=0.000`
- No MAINNET `BTCUSDT` fills linked to either canary.
- BTCUSDT position amount should be `0` after a fresh read-only account/shadow refresh.

## Evidence Bundle Map

TESTNET soak:

- Path: `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`
- Notes: historical export shape; includes masked `credentials.json`, orders, fills, timeline, live status before/after, repair events, and session summary. It predates the newer canary-specific before/after snapshot filenames and `final_verdict.json`.

First MAINNET canary:

- Path: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`
- Order: `BTCUSDT SELL LIMIT 0.001 @ 77950`
- Final: canceled, `executed_qty=0.000`, no fill, no position, restart repair passed.

Second MAINNET canary:

- Path: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`
- Order: `BTCUSDT BUY LIMIT 0.001 @ 77800`
- Final: canceled, `executed_qty=0.000`, no fill, no position, restart repair passed.
- Historical note: first cancel attempt failed because the old route body expected duplicated `order_ref`; retry succeeded. Code was later fixed.

## Running Another Canary

Do not run another canary by default. A previous successful canary does not authorize a new order.

Another canary requires a separate explicit operator request and a fresh dry-run first:

1. Keep `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`.
2. Select and validate `env-mainnet` or a secure-store mainnet credential.
3. Refresh readiness and shadow.
4. Confirm one-way and single-asset account mode.
5. Confirm active symbol is `BTCUSDT` or `BTCUSDC`.
6. Confirm no open MAINNET order and no position.
7. Confirm available quote balance and leverage are sufficient for the smallest non-marketable `LIMIT` preview.
8. Run kill-switch engage/release.
9. Build a fresh non-marketable `LIMIT` preview with fresh reference price.
10. Export dry-run evidence.
11. Only after a separate explicit execution task, restart with `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` for that session.
12. Submit at most one MAINNET canary `LIMIT` order with exact confirmation, cancel immediately, reconcile, repair, and restart with the flag disabled again.

## Must Never Be Done Accidentally

- Do not enable MAINNET auto-execution.
- Do not bypass server-side canary gates.
- Do not use `MARKET` for a MAINNET canary.
- Do not submit conditional/algo orders.
- Do not widen symbol scope.
- Do not treat ACK as fill.
- Do not treat paper state as live account truth.
- Do not add liquidation heatmap/liquidation context as a live decision layer.
- Do not commit `.env`.
- Do not copy raw secrets into docs, logs, screenshots, SQLite, frontend payloads, or evidence.

## Rollback And Stop Notes

If anything looks unsafe:

1. Stop the server.
2. Restart with `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`.
3. Inspect `/api/live/status`, `/api/live/orders`, and `/api/live/fills`.
4. If a live session is running and the state is coherent, engage the kill switch.
5. Verify open orders from the exchange-authoritative live status.
6. Verify BTCUSDT position is flat.
7. Cancel only RelXen-created active-symbol open orders when shadow state is coherent.
8. Flatten only if an unexpected position exists and state is coherent.
9. Preserve evidence and do not submit additional orders while state is ambiguous.
