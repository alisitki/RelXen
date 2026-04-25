# Mainnet Auto Live Trial Plan

## Status

Mainnet Auto Live Support v1 is implemented as a gated code path and the first explicit live session was started on 2026-04-25.

Live MAINNET auto remains disabled by default. The operator-start command surface is prepared for explicit session-scoped batches only after rechecking every gate. The helper accepts the explicit v1 risk flags, verifies the running server is already in session-scoped live mode, configures the risk budget, and then submits the existing typed start request. Use the exact session-level confirmation for the chosen runtime:

```text
START MAINNET AUTO LIVE BTCUSDT 15M
```

For operator-stop runtime, use:

```text
START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP
```

First run result:

- Evidence: `artifacts/mainnet-auto/1777099647957-mnauto_live_39b61e12f8084f669b334420a3f105ac/`
- Session id: `mnauto_live_39b61e12f8084f669b334420a3f105ac`
- Stop reason: `max_runtime_reached`
- Signals/decisions/orders/fills: `0` / `0` / `0` / `0`
- Final state: BTCUSDT flat, no open MAINNET BTCUSDT order
- Lessons: generated; recommendation `safe_to_repeat_dry_run`

The second `always_in_market` run on 2026-04-25 submitted and filled one real LONG, then ended degraded because reversal and auto-owned flat-stop were not implemented at that time. A follow-up implementation batch added mocked-adapter support for coherent `always_in_market` reverse and auto-owned flat-stop. No live order was submitted in that implementation batch.

## Session Design

- Runtime: fixed `15` / `60` minute hard stop, or explicit `0` operator-stop runtime. In operator-stop runtime, `expires_at` is unset and only the fixed max-runtime stop is removed; all other watchdog/risk stops remain active.
- Symbol: `BTCUSDT` only.
- Order type: `MARKET` only for v1 live auto, matching the existing TESTNET auto execution path.
- Strategy: ASO closed-candle signals only.
- Margin type policy: default `isolated`; `cross` must be explicitly allowed with both env and script flag; `unknown` blocks live auto.
- Position policy: default `crossover_only`; optional `always_in_market` uses latest closed-candle ASO state and is more active/riskier; optional `flat_allowed` filters weak ASO states with delta/zone thresholds.
- Confirmation: one explicit operator confirmation to start the session; no per-order confirmation after start.
- Scope: one active symbol, one open live position maximum, one in-flight order maximum.
- Caps: max notional per order/open/session `80`, max realized session loss `5 USDT`, max leverage no higher than the explicit risk budget and hard-capped at `100x`.
- Emergency circuit breakers: max orders `20`, max fills `20`; these are not the primary throttle.
- Primary stops: runtime, loss, notional, stale data, reconciliation ambiguity, kill switch, watchdog, and operator stop.

## Live Execution Behavior

- `BUY` while flat may enter LONG if all gates pass.
- `SELL` while flat may enter SHORT if all gates pass.
- `always_in_market` opposite-side decisions close the current position first with a reduce-only `MARKET` order, require flat reconciliation, and only then submit the opposite entry. If an unresolved order exists, close reconciliation fails, or state is ambiguous, the entry is blocked. `crossover_only` preserves its conservative open-position blocker.
- Duplicate signal suppression persists by environment, symbol, timeframe, candle open time, and signal side.
- ACK is not fill. Order/fill/account truth remains user-data stream plus REST repair.

## Required Pre-Live Checks

1. `.env` is ignored and untracked.
2. `env-mainnet` or another MAINNET credential is explicitly selected and validated.
3. `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=false` before the session.
4. Future session sets `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true` and `RELXEN_MAINNET_AUTO_MODE=live` only for that session.
5. Active symbol is `BTCUSDT`.
6. Account mode is one-way.
7. Margin mode is single-asset.
8. BTCUSDT margin type is known and allowed by `RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE`.
9. BTCUSDT leverage is no higher than the configured session budget and never above `100`.
10. Available USDT supports the configured notional and fees.
11. No open MAINNET BTCUSDT order exists.
12. BTCUSDT position is flat at start.
13. Kill switch is released.
14. Shadow/account/rules/reference price/user-data stream are fresh.
15. Evidence and lesson-report paths are writable.
16. Latest dry-run/live evidence is reviewed.
17. Exact session confirmation is supplied.

## Headless Commands

Terminal 1, start a session-scoped live-auto server. This does not start the auto session by itself:

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

For an operator-stop session, set `RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES=0` in the server environment and use `--duration-minutes operator-stop` plus confirmation `START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP` in the helper. The server config, helper argument, and risk budget must match.

Terminal 2, run the safe precheck against that server:

```sh
RELXEN_BASE_URL=http://localhost:3000 ./scripts/show_mainnet_auto_status.sh --precheck
```

Terminal 2, start the live trial only if the precheck is clean and the operator intends to start real MAINNET auto execution:

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
RELXEN_MAINNET_AUTO_REQUIRE_FLAT_START=true \
RELXEN_MAINNET_AUTO_REQUIRE_FLAT_STOP=true \
RELXEN_MAINNET_AUTO_EVIDENCE_REQUIRED=true \
RELXEN_MAINNET_AUTO_LESSON_REPORT_REQUIRED=true \
RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE=isolated \
RELXEN_MAINNET_AUTO_POSITION_POLICY=crossover_only \
RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD=5 \
RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD=55 \
RELXEN_MAINNET_AUTO_START_CONFIRMATION="START MAINNET AUTO LIVE BTCUSDT 15M" \
./scripts/run_mainnet_auto_live_trial.sh \
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

Terminal 3, heartbeat while the session is running:

```sh
RELXEN_BASE_URL=http://localhost:3000 ./scripts/show_mainnet_auto_status.sh --heartbeat
```

Graceful stop:

```sh
curl -X POST http://localhost:3000/api/live/mainnet-auto/stop
```

Kill switch:

```sh
curl -X POST http://localhost:3000/api/live/kill-switch/engage
```

Release only after review:

```sh
curl -X POST http://localhost:3000/api/live/kill-switch/release
```

Evidence export:

```sh
RELXEN_BASE_URL=http://localhost:3000 ./scripts/export_mainnet_auto_evidence.sh
```

Summary and flat check:

```sh
RELXEN_BASE_URL=http://localhost:3000 ./scripts/show_mainnet_auto_status.sh --summary
RELXEN_BASE_URL=http://localhost:3000 ./scripts/show_mainnet_auto_status.sh --flat-check
```

## Evidence And Lessons

Evidence exports include status snapshots, risk budget, margin policy, position policy, ASO policy decisions, signal context, intent previews, reference prices, watchdog events, orders, fills, final verdict, `lessons.md`, and `lessons.json`.

Lessons are analysis only. They must not change settings, risk, symbols, strategy, leverage, or live enablement automatically.

## Implementation Gaps / Boundaries

- `always_in_market` reverse and flat-stop are implemented only through reduce-only close plus reconciliation. Any ambiguous close/order/position state still blocks entry or degrades stop.
- `always_in_market` and `flat_allowed` are policy modes, not new indicators. They still use closed-candle ASO only and remain bounded by open-order/open-position/reversal safety gates.
- Conditional/algo orders remain unsupported.
- Liquidation heatmap/liquidation context remains deferred and has no live decision impact.
- Supported symbols remain `BTCUSDT` / `BTCUSDC`, with this first live-auto trial limited to `BTCUSDT`.

## Recommendation

`ready_for_execution_batch` only after the full implementation gate passes and a separate operator execution batch rechecks live state. The first no-order run does not approve another run by itself.
