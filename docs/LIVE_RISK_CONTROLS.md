# Live Risk Controls

## Principle

Live execution must be operator-gated and fail closed. In the current repository, actual execution is constrained to TESTNET-only `MARKET` / `LIMIT` placement/cancel/flatten. Risk controls must run before arming, before runtime start, and before every order intent becomes an exchange order request.

## Account-Level Controls

- Max notional per order.
- Max total live position notional.
- Max leverage.
- Max position quantity.
- Max daily realized loss.
- Optional max exposure by quote asset.
- Quote-asset-specific balance checks for `USDT` and `USDC`.
- Minimum free balance after estimated fee and margin.
- Mainnet/testnet environment separation. MAINNET execution is blocked in this build.

## Runtime Guards

- Kill switch blocks all new live orders immediately.
- WebSocket stale, lagged, or `resync_required` state blocks new orders.
- Exchange connectivity degradation blocks new orders.
- Symbol-rules lookup failure blocks arming and order submission.
- Account reconciliation failure blocks new orders and moves runtime to a degraded or failed state.
- Duplicate signal/order suppression prevents repeated execution for the same closed candle.
- Clock drift or timestamp ambiguity blocks signed requests.
- Repeated exchange rejection engages a safe stop.

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
- Manual flatten intent.
- Manual TESTNET flatten execution when shadow state is coherent.
- Manual refresh of credentials, rules, and account snapshot.
- Clear recovery workflow after failure.
- Explicit confirmation before mainnet arming.

Manual flatten should be an intent routed through the same credential, rules, precision, reduce-only, and reconciliation gates as strategy orders.
The implemented TESTNET flatten path cancels active-symbol open orders first, then submits a reduce-only MARKET close only when account mode and shadow position state are deterministic.

## Recovery Rules After Failure

- Do not auto-rearm after kill switch.
- Do not auto-rearm after reconciliation failure.
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

## Fail-Closed Defaults

Any missing, stale, invalid, or ambiguous risk input must block live order placement. The system should require operator action to resume rather than guessing.
