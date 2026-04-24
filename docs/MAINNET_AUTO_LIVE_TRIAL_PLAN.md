# Mainnet Auto Live Trial Plan

## Status

Mainnet Auto Live Support v1 is implemented as a gated code path, but no real MAINNET auto session was started in this batch and no TESTNET or MAINNET order was submitted.

Live MAINNET auto remains disabled by default. A future execution batch must start the server with explicit live config, recheck every gate, and use the exact session-level confirmation:

```text
START MAINNET AUTO LIVE BTCUSDT 15M
```

## Session Design

- Runtime: 15 minutes hard stop.
- Symbol: `BTCUSDT` only.
- Order type: `MARKET` only for v1 live auto, matching the existing TESTNET auto execution path.
- Strategy: ASO closed-candle signals only.
- Confirmation: one explicit operator confirmation to start the session; no per-order confirmation after start.
- Scope: one active symbol, one open live position maximum, one in-flight order maximum.
- Caps: max notional per order/open/session `80`, max realized session loss `5 USDT`, max leverage `5x`.
- Emergency circuit breakers: max orders `20`, max fills `20`; these are not the primary throttle.
- Primary stops: runtime, loss, notional, stale data, reconciliation ambiguity, kill switch, watchdog, and operator stop.

## Live Execution Behavior

- `BUY` while flat may enter LONG if all gates pass.
- `SELL` while flat may enter SHORT if all gates pass.
- Opposite-signal close/reverse is not improvised in v1. If a position or unresolved order exists and the current live policy cannot prove a coherent close/reverse, the decision is blocked or the watchdog stops instead of submitting another order.
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
8. BTCUSDT leverage is `<= 5`.
9. Available USDT supports the configured notional and fees.
10. No open MAINNET BTCUSDT order exists.
11. BTCUSDT position is flat at start.
12. Kill switch is released.
13. Shadow/account/rules/reference price/user-data stream are fresh.
14. Evidence and lesson-report paths are writable.
15. Latest dry-run evidence is reviewed.
16. Exact session confirmation is supplied.

## Headless Commands

Safe precheck:

```sh
RELXEN_BASE_URL=http://localhost:3000 ./scripts/show_mainnet_auto_status.sh --precheck
```

Future live trial start command, to run only in a separate approved execution batch:

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
RELXEN_MAINNET_AUTO_START_CONFIRMATION="START MAINNET AUTO LIVE BTCUSDT 15M" \
./scripts/run_mainnet_auto_live_trial.sh --symbol BTCUSDT --duration-minutes 15
```

Heartbeat:

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

Evidence exports include status snapshots, risk budget, decisions, signal context, intent previews, reference prices, watchdog events, orders, fills, final verdict, `lessons.md`, and `lessons.json`.

Lessons are analysis only. They must not change settings, risk, symbols, strategy, leverage, or live enablement automatically.

## Implementation Gaps / Boundaries

- V1 live auto blocks instead of reversing when an existing position or unresolved order is present.
- Conditional/algo orders remain unsupported.
- Liquidation heatmap/liquidation context remains deferred and has no live decision impact.
- Supported symbols remain `BTCUSDT` / `BTCUSDC`, with this first live-auto trial limited to `BTCUSDT`.

## Recommendation

`ready_for_execution_batch` only after the full implementation gate passes and a separate operator execution batch rechecks live state. This document is not approval to run live MAINNET auto by itself.
