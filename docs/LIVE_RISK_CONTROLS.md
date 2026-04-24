# Live Risk Controls

## Principle

Live execution must be operator-gated and fail closed. In the current repository, actual execution includes TESTNET `MARKET` / `LIMIT` placement/cancel/flatten, TESTNET closed-candle auto-execution, and manual MAINNET canary execution behind a default-off server gate. Risk controls must run before arming, before runtime start, before auto-execution, and before every order intent becomes an exchange order request.

## Account-Level Controls

- Max notional per order.
- Max total live position notional.
- Max leverage.
- Max position quantity.
- Max daily realized loss.
- Optional max exposure by quote asset.
- Quote-asset-specific balance checks for `USDT` and `USDC`.
- Minimum free balance after estimated fee and margin.
- Mainnet/testnet environment separation. MAINNET auto-execution is blocked; manual MAINNET canary requires `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true`, an explicitly selected mainnet credential, matching mainnet shadow/reconciliation environment, sufficient available balance, a fresh reference price from internal market state or Binance USD-M REST mark price, a non-marketable `LIMIT` after tick-size rounding, and all canary gates.
- Explicit operator-configured risk profile before MAINNET canary readiness.
- Real TESTNET soak evidence should be captured and reviewed before any MAINNET canary session. The current successful MAINNET canary evidence bundle is `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`.
- A second-canary readiness dry-run exists under `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`; it did not submit an order and kept the mainnet canary server flag disabled.
- A second bounded MAINNET canary execution exists under `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`; it submitted one order, canceled it, left no fill/position, and disabled the canary flag afterward. Its cancel payload ergonomics issue has been fixed and regression-tested without submitting another order.
- Mainnet-canary closure is an operator handoff, not broader enablement. MAINNET canary remains session-only, MAINNET auto remains blocked, and any further canary must repeat fresh dry-run gates first.
- MAINNET auto infrastructure has its own persisted risk budget and remains live-blocked by default. Dry-run may evaluate a would-submit decision, but it must not call the exchange order endpoint. Future live auto requires explicit server config, operator arm/start, fresh account/rules/shadow/reference price, flat-start checks, evidence logging, lesson reporting, and watchdog readiness.
- The operator-DB dry-run used risk budget `mainnet-auto-operator-dry-run-v1`: `BTCUSDT`, `LIMIT` only, max leverage `5`, one order/fill max, `80` max notional/order/session/open, flat start/stop, fresh shadow/reference, evidence logging, and lesson report required. This is a dry-run profile only and is not approval for live auto.

## Runtime Guards

- Kill switch blocks all new live orders immediately.
- WebSocket stale, lagged, or `resync_required` state blocks new orders.
- Exchange connectivity degradation blocks new orders.
- Symbol-rules lookup failure blocks arming and order submission.
- Account reconciliation failure blocks new orders and moves runtime to a degraded or failed state.
- Duplicate signal/order suppression prevents repeated execution for the same closed candle.
- Clock drift or timestamp ambiguity blocks signed requests.
- Repeated exchange rejection engages a safe stop.
- Real submission handling uses `ACK` and authoritative reconciliation; an ACK is never treated as a fill.
- User-data streams force reconnect and REST repair before the 24-hour lifecycle limit.
- Execution repair is recent-window only because Binance order/trade query retention is finite.
- MAINNET auto watchdog stop reasons include kill switch, stale shadow/account/rules/reference price, user-data stream down, unexpected open order/position, max runtime/orders/fills/loss/rejections, duplicate signal, unsupported account/margin mode, evidence logging failure, lesson report failure, config disabled, and operator stop.

## Start-Gating Conditions

Live runtime may start only when all are true:

- Credentials are present and validated.
- Operator explicitly selected live mode.
- Runtime is armed.
- Active symbol is supported.
- Timeframe is supported.
- Exchange symbol-rules snapshot is present and fresh.
- Account snapshot is present and clean.
- No ambiguous open live position exists.
- Risk limits are configured and valid.
- Market data is connected and not stale.
- Operator has confirmed the current environment, especially mainnet.
- Dedicated position-mode and multi-assets-mode checks report one-way and single-asset mode.
- Shadow stream/account environment matches active mainnet.
- Available mainnet balance is sufficient for required margin plus fee/buffer, and exchange min quantity does not force notional above the approved canary cap.
- Active-symbol account/exchange leverage is no greater than the approved canary maximum.
- MAINNET canary has the server canary flag enabled and exact confirmation text for the current preview.
- MAINNET canary preview is `LIMIT`, non-marketable after rounding, and based on a fresh reference price.
- MAINNET canary review has the current TESTNET soak evidence bundle and updated checklist.
- MAINNET auto live mode has `RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true`, `RELXEN_MAINNET_AUTO_MODE=live`, an operator arm/start command, a valid risk budget, and evidence/lesson output initialized. These gates are for a future explicit batch; current default is dry-run/live-blocked.

## Stop Conditions

Live runtime must stop or degrade when:

- Operator manually stops.
- Kill switch is engaged.
- Reconciliation becomes ambiguous.
- Exchange rejects orders repeatedly.
- Connectivity is stale too long.
- Symbol rules become stale and cannot refresh.
- Credentials become invalid or revoked.
- Daily loss or exposure limit is breached.

## Operator Controls

- Arm/disarm live mode.
- Kill switch.
- TESTNET auto start/stop.
- Manual flatten intent.
- Manual TESTNET or canary-gated MAINNET flatten execution when shadow state is coherent.
- Manual refresh of credentials, rules, and account snapshot.
- Clear recovery workflow after failure.
- Exact confirmation before MAINNET canary execution.
- No TESTNET drill helper is enabled or used in a MAINNET session.

Manual flatten should be an intent routed through the same credential, rules, precision, reduce-only, and reconciliation gates as strategy orders.
The implemented TESTNET flatten path cancels active-symbol open orders first, then submits a reduce-only MARKET close only when account mode and shadow position state are deterministic.

## Recovery Rules After Failure

- Do not auto-rearm after kill switch.
- Do not auto-rearm after reconciliation failure.
- Do not auto-restart TESTNET auto-execution after kill switch or reconciliation failure.
- Do not auto-restart MAINNET auto after watchdog stop, kill switch, evidence failure, lesson failure, or reconciliation ambiguity.
- Require fresh account snapshot after any exchange rejection burst.
- Require operator acknowledgement before moving from degraded/error back to ready.
- Preserve audit events for the failure and recovery path.

## Minimal Future UI Requirements

- Textual live state, not color-only meaning.
- Unmistakable `PAPER` versus `LIVE` distinction.
- Visible armed/disarmed state.
- Visible environment marker: testnet or mainnet.
- Disabled controls when prerequisites are missing.
- Confirmation wording that includes symbol, environment, max notional, and max loss.
- Clear kill-switch button and post-kill status.
- Reconciliation status and last account snapshot age.
- Explicit `MAINNET CANARY READY` versus `MAINNET EXECUTION BLOCKED` text.
- Evidence/export status should distinguish smoke-only exports from real TESTNET drill evidence.
- New signal-context modules, including liquidation heatmap/liquidation context, must remain non-execution work until separately designed and reviewed. They must not silently widen live decision inputs or canary gates.

## Fail-Closed Defaults

Any missing, stale, invalid, or ambiguous risk input must block live order placement. The system should require operator action to resume rather than guessing.
