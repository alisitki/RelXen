# Runbook

## First Run

1. Install Rust and Node.js.
2. Copy `.env.example` to `.env`.
3. Install and build the dashboard:

```sh
cd web
npm install
npm run build
cd ..
```

4. Start the integrated backend/frontend server:

```sh
cargo run -p relxen-server
```

5. Open `http://localhost:3000/`.

## Environment Variables

- `RELXEN_BIND`: backend bind address. Default is `[::]:3000`.
- `RELXEN_DATABASE_URL`: SQLite URL. Default is `sqlite://var/relxen.sqlite3`.
- `RELXEN_FRONTEND_DIST`: built frontend directory. Default is `web/dist`.
- `RELXEN_LOG_LEVEL`: tracing filter. Default is `info,relxen=debug`.
- `RELXEN_AUTO_START`: whether bootstrap should start the WebSocket runtime. Default is `true`.
- `RELXEN_CREDENTIAL_SOURCE`: set to `env` to load local operator credentials from `.env`. This setting is authoritative.
- `RELXEN_ENABLE_ENV_CREDENTIALS`: compatibility alias; `true` enables env credentials only when `RELXEN_CREDENTIAL_SOURCE` is unset.
- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION`: enables the manual MAINNET canary path when every other gate passes. Default is `false`; leave it false for normal paper/testnet operation.
- `RELXEN_ENABLE_TESTNET_DRILL_HELPERS`: enables explicit TESTNET-only drill helpers for a bounded soak drill. Default is `false`; leave it off outside intentional soak validation.
- `BINANCE_TESTNET_API_KEY`, `BINANCE_TESTNET_API_SECRET_KEY`, `BINANCE_MAINNET_API_KEY`, `BINANCE_MAINNET_API_SECRET_KEY`: local-only env credential values when env source is enabled. `.env.example` must contain placeholders only and `.env` must never be committed.

## Database And Migrations

The backend creates the SQLite parent directory if needed and runs SQLx migrations on connect. Runtime persistence uses real SQLite tables for settings, klines, signals, trades, paper wallets, paper positions, logs, live credential metadata, shadow snapshots, preflight results, TESTNET live orders, TESTNET live fills, and execution state cache.

Live shadow snapshots and preflight results are cached for operator visibility. Preflight never means an order was placed. TESTNET live order/fill records are separate execution records and must be interpreted through exchange reconciliation status.

Raw live API secrets are not stored in SQLite. Normal production-minded runtime stores secret material through the OS secure-storage adapter; tests use in-memory secret stores. `RELXEN_CREDENTIAL_SOURCE=env` is a local operator convenience that reads raw values from process environment only and persists masked metadata plus source in SQLite. In this authoritative env-source mode, the TESTNET env credential is selected at startup ahead of any persisted secure-store TESTNET selection so local validation does not trigger OS secure-storage prompts. MAINNET env credentials are never auto-selected.

SQLite is configured for WAL mode, `synchronous = normal`, and a busy timeout. To reset local paper state safely, use the UI `Reset Paper` action or:

```sh
curl -X POST http://localhost:3000/api/paper/reset
```

Deleting the SQLite file is only recommended when you intentionally want a clean local instance.

## Health Checks

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl -I http://localhost:3000/
```

`/api/bootstrap` returns the complete typed snapshot used by the dashboard: metadata, runtime status, settings, candles, ASO points, recent signals/trades/logs, current position, wallets, performance, connection state, live status, and system metrics.

Live-foundation checks:

```sh
curl http://localhost:3000/api/live/status
curl http://localhost:3000/api/live/credentials
curl -X POST http://localhost:3000/api/live/readiness/refresh
curl -X POST http://localhost:3000/api/live/start-check
curl -X POST http://localhost:3000/api/live/shadow/refresh
curl http://localhost:3000/api/live/intent/preview
curl http://localhost:3000/api/live/preflights
curl http://localhost:3000/api/live/orders
curl http://localhost:3000/api/live/fills
```

## Testnet Soak Evidence Capture

The bounded TESTNET soak drill is documented in [TESTNET_SOAK_RUNBOOK.md](./TESTNET_SOAK_RUNBOOK.md). It should be run before any MAINNET canary review.

Evidence export uses existing read-only API endpoints and writes ignored local artifacts under `artifacts/testnet-soak/<timestamp>/`:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_live_evidence.sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_testnet_soak.sh
```

The guided script does not create credentials, arm execution, or place orders. It pauses while the operator performs each drill step through the UI/API and captures status, masked credential summaries, orders, fills, preflights, blocking reasons, and repair-related logs. If valid TESTNET credentials are unavailable, mark the real exchange scenarios as not exercised and keep the generated smoke/export artifacts separate from real-drill evidence.

The latest go/no-go report is [LATEST_TESTNET_SOAK_REPORT.md](./LATEST_TESTNET_SOAK_REPORT.md). The current real evidence bundle is `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`. MAINNET canary checklist and no-go criteria are in [MAINNET_CANARY_CHECKLIST.md](./MAINNET_CANARY_CHECKLIST.md).

## Runtime States

- `CONNECTED`: WebSocket deltas are flowing normally.
- `RECONNECTING <age>`: the runtime is reopening the Binance stream.
- `STALE <age>`: data is stale or deterministic recovery is not yet complete.
- `RESYNCED`: bounded REST recovery completed and market-data deltas are resuming.
- `DISCONNECTED`: runtime is stopped or disconnected.
- `HISTORY SYNC`: bootstrap/history loading is active.
- `REBUILDING`: settings apply triggered a deterministic series rebuild.

If `resync_required` is emitted, the frontend reloads `/api/bootstrap` and rebuilds chart data with `setData()`. Normal steady-state candle/ASO updates remain incremental.

## LIVE ACCESS Flow

The LIVE ACCESS panel supports paper-mode operation, read-only shadow/preflight work, TESTNET manual execution, TESTNET closed-candle auto-execution, kill switch controls, and a manual MAINNET canary path that is disabled by default.

The shareable RC UI shows a top safety strip before the control panels. It should plainly show `MAINNET AUTO: BLOCKED`, `MAINNET CANARY: DISABLED` unless a separate canary session is intentionally enabled, kill-switch state, active symbol, current state, and position state. The LIVE ACCESS panel groups its controls into credential, readiness/shadow/account, preview/preflight, safety/canary controls, orders/fills, and advanced details.

1. Create a credential with alias, environment (`testnet` or `mainnet`), API key, and API secret, or enable local env credentials with `RELXEN_CREDENTIAL_SOURCE=env`.
2. RelXen stores secure-store raw material in OS secure storage; env raw material remains process-only. SQLite stores only masked metadata and source.
3. Select the active credential. TESTNET env credentials may auto-select when no valid active TESTNET credential exists. When `RELXEN_CREDENTIAL_SOURCE=env` is set, the TESTNET env credential takes precedence over a persisted secure-store TESTNET selection. MAINNET env credentials never auto-select and require explicit selection.
4. Run `Validate` to perform a signed read-only Binance USDⓈ-M Futures account check.
5. Run `Refresh Readiness` to fetch symbol rules and a read-only account snapshot for the active symbol.
6. If all gates pass, the state becomes `ready_read_only` and arming is enabled.
7. Use `Start Shadow Sync` to open a Binance USDⓈ-M user-data listenKey stream and maintain a read-only shadow view.
8. Use `Build Preview` to compute a precision-aware `MARKET` or `LIMIT` order intent from current settings, rules, shadow account state, and the latest closed signal when available.
9. Use `Run Preflight` to submit the serialized payload to Binance testnet `order/test`. This validates the signed payload but does not place an order.
10. Use `Configure Conservative Risk Profile` before any MAINNET canary review. MAINNET canary readiness cannot pass without an explicit operator-configured risk profile.
11. Use `Execute TESTNET Preview` only when the UI shows `TESTNET EXECUTION READY`. Confirm the browser prompt. This sends a real TESTNET matching-engine order, not a mainnet order.
12. Use `Start TESTNET Auto` only after shadow sync is fresh and you intentionally want closed-candle ASO signals to submit TESTNET orders. Auto mode is TESTNET-only and suppresses duplicate signal/candle intents.
13. If a bounded soak window produces no natural fresh closed-candle auto signal, you may use the drill-only helper endpoint only when `RELXEN_ENABLE_TESTNET_DRILL_HELPERS=true`:

```sh
curl -X POST http://localhost:3000/api/live/drill/auto/replay-latest-signal \
  -H 'content-type: application/json' \
  -d '{"confirm_testnet_drill":true}'
```

14. Use `Engage Kill Switch` to block all new live submissions immediately. Release requires explicit operator action.
15. Use MAINNET canary controls only when `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true`, the active credential is explicitly selected mainnet, a risk profile is configured, all gates pass, a non-marketable `LIMIT` preview remains non-marketable after tick-size rounding, and the displayed exact confirmation text is entered.
16. Use cancel/cancel-all or flatten only when shadow state is coherent. RelXen cancels active-symbol open orders first for flatten, then submits a reduce-only MARKET close intent when safe.

If OS secure storage is unavailable, the UI/API report `secure_store_unavailable` and paper mode remains usable. If env source is enabled with missing or partial variables, the UI/API report env credential blockers and paper mode remains usable. `.env` is local-only operator convenience, not production-grade secret storage; never commit it or put raw secrets in SQLite, frontend storage, logs, screenshots, docs, reports, or evidence bundles.

## Live Readiness States

- `credentials_missing`: no active live credential is selected.
- `secure_store_unavailable`: the OS secure-storage backend could not be used.
- `validation_failed`: credential validation is missing, stale, invalid, or failed.
- `rules_unavailable`: active symbol rules are missing.
- `account_snapshot_unavailable`: read-only account snapshot is missing.
- `ready_read_only`: credentials, rules, account snapshot, and local gates are sufficient for read-only readiness.
- `armed_read_only`: the operator explicitly armed live mode; testnet execution still requires the execution gates below.
- `shadow_starting`: listenKey creation, REST shadow bootstrap, or stream attachment is in progress.
- `shadow_running`: user-data shadow sync is running and the shadow account snapshot is coherent enough for preflight work.
- `shadow_degraded`: the user-data stream or REST fallback became ambiguous or stale; live readiness fails closed.
- `preflight_ready`: an order-intent preview passed local precision/rules checks and may be validated through testnet `order/test`.
- `preflight_blocked`: a preview or preflight is blocked locally, for example due to stale shadow state, unsupported mode, mainnet preflight, or precision/rule validation failure.
- `testnet_execution_ready`: a confirmed TESTNET order may be submitted for the displayed preview.
- `testnet_auto_running`: RelXen is consuming closed-candle signals and may submit TESTNET orders when every execution gate passes.
- `kill_switch_engaged`: new live submissions are blocked immediately. Cancel/flatten may remain available only when deterministic and safe.
- `testnet_submit_pending`: RelXen submitted a TESTNET order and is waiting for authoritative exchange reconciliation.
- `testnet_order_open`: the exchange reports a working TESTNET order.
- `testnet_partially_filled`: the exchange reports partial fills.
- `testnet_filled`: the exchange reports the TESTNET order as filled.
- `testnet_cancel_pending`: RelXen submitted a cancel and is waiting for authoritative exchange reconciliation.
- `execution_degraded`: submission, stream, or repair state is ambiguous; new submissions fail closed.
- `mainnet_execution_blocked`: MAINNET execution is disabled by server canary policy or another fail-closed gate.
- `mainnet_canary_ready`: manual MAINNET canary gates can pass if exact operator confirmation is entered.
- `mainnet_manual_execution_enabled`: manual MAINNET canary submission is available for the current displayed preview only. MAINNET auto-execution is not available.
- `start_blocked`: a live start was requested or checked, but current gates block the operation.

To switch back safely, click `Disarm` or set the execution preference to `PAPER MODE`. This does not affect paper wallets or paper positions.

## Live Shadow And Preflight Controls

- `Start Shadow Sync`: creates a listenKey, bootstraps shadow account state through REST, and attaches a user-data WebSocket stream.
- `Stop Shadow Sync`: closes the listenKey when possible and marks the shadow stream stopped.
- `Refresh Shadow`: refreshes shadow account state from read-only REST and clears stale shadow blockers only when the snapshot is coherent.
- `Build Preview`: builds an inspectable intent. Quantity and LIMIT price are rounded with live decimal logic and exchange symbol rules.
- `Run Preflight`: sends the preview payload to Binance testnet `POST /fapi/v1/order/test`. Mainnet preflight is blocked in this repository state.
- `Execute TESTNET Preview`: submits the displayed preview to Binance testnet `POST /fapi/v1/order` after explicit confirmation and fail-closed gates.
- `Execute MAINNET Canary Preview`: submits the displayed preview to Binance mainnet only when the server canary flag is enabled, a risk profile is configured, the exact confirmation text matches, and every normal gate passes.
- `Start TESTNET Auto`: enables closed-candle strategy-driven TESTNET order submission. Duplicate signal/open-time intents are persisted and suppressed.
- `Stop TESTNET Auto`: stops strategy-driven TESTNET submissions without stopping paper mode.
- `Engage Kill Switch`: blocks every new live submission.
- `Release Kill Switch`: clears the kill switch; other gates still decide readiness.
- `Cancel Open ... Order`: sends `DELETE /fapi/v1/order` for a RelXen-created order.
- `Cancel All Active-Symbol Orders`: cancels RelXen-created open orders for the active symbol only.
- `Flatten ... Position`: cancels open active-symbol orders and submits a reduce-only MARKET close when shadow position state is deterministic.

If the stream expires, disconnects, or cannot be reconciled, the UI should show `SHADOW DEGRADED`, `execution_degraded`, `shadow_stream_down`, or `shadow_state_ambiguous`. Stop and restart shadow sync after checking credentials/connectivity. Do not interpret preflight success as exchange position truth.

RelXen forces user-data stream reconnect plus REST repair before the Binance 24-hour user-data WebSocket lifecycle limit. After an ambiguous submission or reconnect, do not retry manually until `/api/live/status` shows a coherent order state or a degraded/blocked state with a clear repair outcome. Repair is intentionally recent-window only because Binance order/trade query retention is finite; older ambiguity must remain degraded and operator-reviewed. Manual `Refresh Shadow` now also triggers bounded recent-window execution repair so restart/reconnect recovery uses the same operator-facing repair path captured in the real TESTNET soak.

Real order submissions request `ACK`. `ACK` means Binance accepted the request, not that the order is filled. User-data stream events and recent-window REST repair define final order, fill, account, and position truth.

## MAINNET Canary Procedure

For normal operator handoff after the completed canary phase, start with [OPERATOR_HANDOFF.md](./OPERATOR_HANDOFF.md). The procedure below is only for an explicitly requested canary session.

1. Review [LATEST_TESTNET_SOAK_REPORT.md](./LATEST_TESTNET_SOAK_REPORT.md) and [LATEST_MAINNET_CANARY_REPORT.md](./LATEST_MAINNET_CANARY_REPORT.md). If the current recommendation is NO-GO, do not proceed.
2. Review [MAINNET_CANARY_CHECKLIST.md](./MAINNET_CANARY_CHECKLIST.md) and satisfy every hard precondition.
3. Leave `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` unless you are intentionally performing a canary drill. A previous successful canary does not authorize a second order; the operator must explicitly request the follow-up run and repeat every gate with fresh state.
4. Stop TESTNET auto mode and engage the kill switch before changing credentials or environment.
5. Set `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` only for the canary run, restart the backend, and verify `/api/live/status` reports canary server enablement.
6. Explicitly select and validate a mainnet credential from OS secure storage or the env-backed `env-mainnet` summary. MAINNET env credentials are never auto-selected.
7. Configure a conservative risk profile and verify the active symbol is `BTCUSDT` or `BTCUSDC`.
8. Start shadow sync and verify dedicated position-mode and multi-assets-mode checks report one-way and single-asset mode, the shadow environment is `mainnet`, available balance is sufficient for required margin plus fee/buffer, the exchange min quantity does not force notional above the approved canary cap, and active-symbol exchange leverage is no greater than the approved canary maximum.
9. Build a non-marketable `LIMIT` preview, read the exact required confirmation text, and enter it only if you intend to submit that exact MAINNET order. `MARKET` and rounded marketable `LIMIT` canary previews are blocked. The final MAINNET preview must include a fresh reference price from internal market state or the Binance USD-M REST mark-price resolver.
10. Submit one manual canary action only. Wait for user-data/REST reconciliation before any follow-up.
11. Export evidence immediately after the action.
12. Disable the server canary flag after the drill and restart back into the default blocked state.

For a second canary readiness dry-run, keep `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false`, repeat credential validation/readiness/shadow refresh, exercise kill-switch engage/release, rebuild a fresh non-marketable `LIMIT` preview, export evidence, and stop before `POST /api/live/execute`. The latest dry-run evidence is `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`.

The latest second canary execution evidence is `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`. That run exposed a cancel payload ergonomics issue, later fixed: cancel requests now use the route path order reference as authoritative. The JSON body should carry only the required confirmation fields; if an optional body `order_ref` is supplied, it must match the path.

## Common Failure Notes

- History/rebuild failures mean contiguous closed-candle coverage could not be proven. The backend surfaces a typed history error and keeps the last valid visible state rather than silently running on ambiguous data.
- Settings apply can be temporarily disabled while `HISTORY SYNC` or `REBUILDING` is active.
- Paper commands can be temporarily disabled during history work. Command failures are normalized into operator-facing toasts.
- `RELXEN_AUTO_START=false` is useful for a backend smoke run that should bootstrap data but not open the market-data WebSocket stream.

## Paper-Mode Boundaries

RelXen v1 paper mode remains independent. Post-v1 RelXen can place/cancel/flatten TESTNET orders only through explicit operator actions and fail-closed gates. MAINNET remains default-off except for the manual canary gate, MAINNET auto remains blocked, conditional/algo orders are unsupported, and symbol scope is unchanged. Live credentials use OS secure storage by default or explicit local env loading; SQLite stores masked metadata only.

Liquidation heatmap/liquidation-context work is deferred until after mainnet safety hardening. It should not be added as a model, API, frontend panel, strategy input, or live decision layer in the current canary-readiness flow.
