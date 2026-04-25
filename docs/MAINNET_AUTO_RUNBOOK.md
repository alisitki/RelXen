# Mainnet Auto Runbook

## Current Boundary

MAINNET auto infrastructure exists for dry-run validation, evidence, lesson reporting, and gated session-scoped live runs. Live MAINNET auto remains disabled by default and must not be started without a separate explicit live-run task.

Dry-run may read status, credentials metadata, market/account/rules/shadow/reference context, and closed-candle ASO decisions. It must not submit, cancel, or flatten orders.

The first explicit live run, `15-Minute MAINNET Auto Live Run v1`, completed on 2026-04-25 under `artifacts/mainnet-auto/1777099647957-mnauto_live_39b61e12f8084f669b334420a3f105ac/`. It ran `BTCUSDT` in `live` mode for the bounded 15-minute session, stopped by watchdog at `max_runtime_reached`, observed zero closed-candle ASO signals, submitted zero orders, recorded zero fills, ended flat with zero open MAINNET BTCUSDT orders, and generated `lessons.md` / `lessons.json`. This is not approval for always-on or broader MAINNET auto.

The second explicit live run used `--allowed-margin-type isolated --position-policy always_in_market` and completed degraded under `artifacts/mainnet-auto/1777104375086-mnauto_live_0518464591cd473fbdac1e34675c1cae/`. It submitted one real `BUY MARKET BTCUSDT 0.001` order, reconciled it as filled at average price `77493.50000`, held LONG while ASO desired LONG, then blocked later desired SHORT reversals as `reversal_unsupported`. The watchdog stopped at `max_runtime_reached`, but flat stop failed because the final exchange position remained LONG `0.001` with zero open BTCUSDT orders. Manual cleanup then used the existing canary-gated flatten path to submit one reduce-only `SELL MARKET BTCUSDT 0.001`, filled at average price `77513.60000`, and final BTCUSDT position amount became `0`; evidence is `artifacts/mainnet-canary/20260425T081553Z-mainnet-auto-manual-flatten/`.

The follow-up policy hardening batch added a MAINNET-auto-owned reduce-only close path for coherent flat-stop and `always_in_market` reversal. Mocked-adapter tests prove LONG-to-SHORT reversal closes flat first and then enters the opposite side, and watchdog/operator stop can flatten a coherent open position without the manual canary endpoint. No live order was submitted in that hardening batch.

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
- allowed margin type defaults to `isolated`
- ASO position policy defaults to `crossover_only`

## Margin Type Policy

MAINNET auto treats margin type as separate from single-asset/multi-assets mode:

- `isolated`: allowed by default for MAINNET auto.
- `cross`: blocked unless the operator explicitly sets `RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=cross` or `any`.
- `unknown`: blocks live MAINNET auto. Do not infer isolated from single-asset mode.

Use `cross` only when the operator intentionally accepts cross-margin risk. Use `any` only for diagnostics because it allows both cross and isolated but still blocks unknown.

## ASO Position Policy

`RELXEN_MAINNET_AUTO_POSITION_POLICY` supports:

- `crossover_only`: default conservative behavior. Only a new closed-candle ASO bull/bear cross can create an entry decision.
- `always_in_market`: latest closed ASO state maps bulls > bears to desired LONG and bears > bulls to desired SHORT. This is more active and riskier because it can enter from current state instead of waiting for a rare crossover.
- `flat_allowed`: uses ASO state plus conservative `RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD` and `RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD` filters. Weak/equal states stay flat when flat; if already in position, the default is hold rather than inventing a stop-loss/take-profit.

Reversal is not improvised. `always_in_market` uses a reduce-only MARKET close first, waits for flat reconciliation, and only then submits the opposite entry. If the close is not acknowledged/reconciled, if an open order exists, or if state is ambiguous, the entry is blocked. `crossover_only` preserves the conservative open-position blocker.

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
- `position_policy.json`
- `margin_policy.json`
- `auto_decisions.json`
- `aso_policy_decisions.json`
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

## Live Mode Requirements For A Repeat Batch

Do not start live MAINNET auto in normal operation. Mainnet Auto Live Support v1 has been exercised once with no orders/fills and remains session-scoped. For any repeat operator-started live batch, the terminal helper accepts the explicit v1 risk flags, checks the running server is in session-scoped live mode, configures the bounded risk budget, and then uses the existing typed start endpoint. A live batch must require all of the following:

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
- max leverage not above budget and no higher than the hard live-auto cap of `100`
- evidence logging initialized
- lesson-report output initialized
- kill switch released
- watchdog running

If any input is missing, stale, ambiguous, or invalid, live start must fail closed and no order may be submitted.

Start the server in a dedicated terminal before the helper. This server config is session-scoped and does not start auto by itself:

```sh
RELXEN_CREDENTIAL_SOURCE=env \
RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true \
RELXEN_MAINNET_AUTO_MODE=live \
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false \
RELXEN_AUTO_START=false \
RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=15 \
RELXEN_MAINNET_AUTO_MAX_ORDERS=20 \
RELXEN_MAINNET_AUTO_MAX_FILLS=20 \
RELXEN_MAINNET_AUTO_MAX_NOTIONAL=80 \
RELXEN_MAINNET_AUTO_MAX_DAILY_LOSS=5 \
RELXEN_MAINNET_AUTO_REQUIRE_FLAT_START=true \
RELXEN_MAINNET_AUTO_REQUIRE_FLAT_STOP=true \
RELXEN_MAINNET_AUTO_EVIDENCE_REQUIRED=true \
RELXEN_MAINNET_AUTO_LESSON_REPORT_REQUIRED=true \
RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=isolated \
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only \
RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD=5 \
RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD=55 \
cargo run -p relxen-server
```

Operator live trial command, only after `--precheck` is clean:

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
RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=isolated \
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only \
RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD=5 \
RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD=55 \
RELXEN_MAINNET_AUTO_START_CONFIRMATION="START MAINNET AUTO LIVE BTCUSDT 15M" \
scripts/run_mainnet_auto_live_trial.sh \
  --symbol BTCUSDT \
  --duration-minutes 15 \
  --max-leverage 100 \
  --max-notional 80 \
  --max-session-loss-usdt 5 \
  --order-type MARKET \
  --allowed-margin-type isolated \
  --position-policy crossover_only \
  --aso-delta-threshold 5 \
  --aso-zone-threshold 55 \
  --confirm "START MAINNET AUTO LIVE BTCUSDT 15M"
```

Monitor and recovery helpers:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --heartbeat
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --summary
RELXEN_BASE_URL=http://localhost:3000 scripts/show_mainnet_auto_status.sh --flat-check
```

Latest live evidence summary:

- Session id: `mnauto_live_39b61e12f8084f669b334420a3f105ac`
- Evidence: `artifacts/mainnet-auto/1777099647957-mnauto_live_39b61e12f8084f669b334420a3f105ac/`
- Stop reason: `max_runtime_reached`
- Signals/decisions/orders/fills: `0` / `0` / `0` / `0`
- Realized PnL / fees: `0` / `0`
- Final BTCUSDT position: flat
- Final MAINNET BTCUSDT open orders: `0`
- Lesson recommendation: `safe_to_repeat_dry_run`; treat this as "repeat dry-run/review first", not live approval.

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
