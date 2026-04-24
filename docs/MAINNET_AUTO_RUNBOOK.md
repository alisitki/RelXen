# Mainnet Auto Runbook

## Current Boundary

MAINNET auto infrastructure exists for dry-run validation, evidence, lesson reporting, and a gated future live-session path. Live MAINNET auto remains disabled by default and must not be started without a separate explicit live-run task.

Dry-run may read status, credentials metadata, market/account/rules/shadow/reference context, and closed-candle ASO decisions. It must not submit, cancel, or flatten orders.

## Safe Defaults

Start with live auto disabled:

```sh
RELXEN_CREDENTIAL_SOURCE=env \
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false \
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false \
RELXEN_MAINNET_AUTO_MODE=dry_run \
cargo run -p relxen-server
```

Check status:

```sh
curl http://localhost:3000/api/live/mainnet-auto/status
curl http://localhost:3000/api/live/mainnet-auto/risk-budget
curl http://localhost:3000/api/live/mainnet-auto/decisions?limit=10
curl http://localhost:3000/api/live/mainnet-auto/lessons/latest
```

Expected safe posture:

- live auto config is disabled unless explicitly changed
- mode is `dry_run`
- live order count is `0`
- blockers explain why live mode is not allowed
- MAINNET canary remains a separate default-off manual path

## Headless Dry-Run

Use the helper scripts from a running server:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_mainnet_auto_dry_run.sh
```

The dry-run script refuses to run if its shell environment contains `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true`. It starts dry-run mode, records one decision cycle, exports evidence, and prints the evidence path.

Export evidence again if needed:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_mainnet_auto_evidence.sh
```

Evidence is written under `artifacts/mainnet-auto/<timestamp>/` and is ignored local operational data by default.

Latest operator-DB dry-run evidence:

- `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`
- Mode: `dry_run`
- Credential: masked `env-mainnet`, explicitly selected and validated
- Decision: `dry_run_would_submit`
- Live order submitted: no
- Orders/fills: empty
- Live-start check: `config_blocked` with live-auto config disabled

This evidence is not live-auto approval. It only supports preparing a separate explicit live-auto plan if the operator wants to continue.

## Evidence Files

A dry-run evidence bundle should include:

- `manifest.json`
- `session_summary.md`
- `timeline.ndjson`
- `live_status_before.json`
- `live_status_after.json`
- `auto_status_before.json`
- `auto_status_after.json`
- `risk_budget.json`
- `auto_decisions.json`
- `signal_events.json`
- `aso_context.json`
- `intent_previews.json`
- `reference_prices.json`
- `watchdog_events.json`
- `blocking_reasons.json`
- `orders.json`
- `fills.json`
- `final_verdict.json`
- `lessons.md`
- `lessons.json`

In dry-run, `orders.json` and `fills.json` must be empty or explicitly show that no live order was submitted.

## Live Mode Requirements For A Future Batch

Do not start live MAINNET auto in the current boundary. Mainnet Auto Live Support v1 is implemented but unrun. A future live batch must require all of the following:

- `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true`
- `RELXEN_MAINNET_AUTO_MODE=live`
- explicit operator arm/start command and strong confirmation `START MAINNET AUTO LIVE BTCUSDT 15M`
- validated MAINNET credential
- fresh account, rules, shadow, user-data, and reference price
- flat start if required by risk budget
- supported first live-auto symbol: `BTCUSDT`
- order type: `MARKET` for v1 live auto
- no open order or unresolved position
- configured risk budget
- max leverage not above budget
- evidence logging initialized
- lesson-report output initialized
- kill switch released
- watchdog running

If any input is missing, stale, ambiguous, or invalid, live start must fail closed and no order may be submitted.

Future live trial command, for an approved execution batch only:

```sh
RELXEN_BASE_URL=http://localhost:3000 \
RELXEN_CREDENTIAL_SOURCE=env \
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true \
RELXEN_MAINNET_AUTO_MODE=live \
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false \
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=15 \
RELXEN_MAINNET_AUTO_MAX_ORDERS=20 \
RELXEN_MAINNET_AUTO_MAX_FILLS=20 \
RELXEN_MAINNET_AUTO_MAX_NOTIONAL=80 \
RELXEN_MAINNET_AUTO_MAX_DAILY_LOSS=5 \
RELXEN_MAINNET_AUTO_START_CONFIRMATION="START MAINNET AUTO LIVE BTCUSDT 15M" \
scripts/run_mainnet_auto_live_trial.sh --symbol BTCUSDT --duration-minutes 15
```

Monitor and recovery helpers:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --heartbeat
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --summary
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --flat-check
```

## Stop And Recovery

Stop dry-run:

```sh
curl -X POST http://localhost:3000/api/live/mainnet-auto/dry-run/stop
```

Stop any mainnet-auto session:

```sh
curl -X POST http://localhost:3000/api/live/mainnet-auto/stop
```

If status is ambiguous, stop the server, keep `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false`, inspect `/api/live/status`, `/api/live/orders`, `/api/live/fills`, and export evidence before taking any further action.
