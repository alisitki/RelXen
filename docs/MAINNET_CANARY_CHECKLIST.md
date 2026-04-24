# Mainnet Canary Checklist

## Current Recommendation

Default recommendation: GO for the bounded manual canary path that was exercised. Latest canary evidence: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/` and [LATEST_MAINNET_CANARY_REPORT.md](./LATEST_MAINNET_CANARY_REPORT.md).

MAINNET canary is engineered behind explicit server and operator gates, but the operator should not enable it from a fresh checkout without current testnet drill evidence.

Closure posture: prep/review only by default. Do not submit another MAINNET order unless the operator explicitly requests that run, `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` is enabled only for that session, MAINNET auto remains blocked, and every hard precondition below passes again with fresh evidence.

Latest second-canary dry-run evidence: `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`. The dry-run did not submit an order. It produced a fresh `BUY LIMIT BTCUSDT 0.001 @ 77800` readiness preview with reference `78294.8`, source `internal_market_candle`, age `25046 ms`, required margin `15.56`, available `USDT=25.0902305`, and mainnet canary disabled.

Latest second-canary execution evidence: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`. One real `BUY LIMIT BTCUSDT 0.001 @ 77800` order submitted with ACK, canceled, reconciled with `executed_qty=0.000`, left no position, passed restart repair, and disabled the canary flag afterward. The run exposed a cancel payload ergonomics issue that is now fixed: `POST /api/live/orders/:order_ref/cancel` uses the path `order_ref` as authoritative and no longer requires duplication in the JSON body.

Operator handoff: see [OPERATOR_HANDOFF.md](./OPERATOR_HANDOFF.md). It is the default reference for safe startup, status inspection, evidence review, and rollback/stop notes.

## Hard Preconditions

- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true` is intentionally set only for the canary session.
- TESTNET soak evidence exists and covers execution, reconciliation, kill switch, cancel or documented immediate-fill behavior, flatten when applicable, restart repair, and reconnect repair.
- Mainnet credential is explicitly selected and validated from OS secure storage or the env-backed `env-mainnet` summary. MAINNET env credentials must never auto-select.
- Active environment is `mainnet`.
- Active symbol is `BTCUSDT` or `BTCUSDC`.
- Account mode checks report one-way mode.
- Multi-assets mode check reports single-asset mode.
- Shadow sync is running and fresh.
- Shadow stream environment and shadow account environment match active `mainnet`.
- Mainnet available balance is sufficient for required margin plus fee/buffer on the smallest exchange-compliant non-marketable `LIMIT` preview.
- The approved max notional cap is compatible with exchange min quantity at the current reference price.
- Account/exchange leverage for the active symbol is no greater than the approved canary maximum.
- Symbol rules are present and fresh.
- Account snapshot is present and fresh.
- Operator risk profile is configured and conservative.
- Kill switch has been tested in the same session before canary.
- Live mode is armed.
- TESTNET auto mode is stopped.
- MAINNET auto execution remains unavailable.
- Preview is fresh and matches the displayed exact confirmation text.
- Preview is a `LIMIT`, has a fresh reference price, and remains non-marketable after tick-size rounding.
- A fresh preview still passes after the same-session kill-switch engage/release drill.
- Operator has reviewed max notional, max leverage, max daily loss, and flatten procedure.
- No TESTNET drill helper is enabled or used in the MAINNET session.
- The operator explicitly requested this canary run. A prior successful canary does not authorize a follow-up order by itself.

## Immediate No-Go Conditions

- No current TESTNET soak evidence bundle.
- Mainnet canary server gate is false or unknown.
- Kill switch engaged unintentionally or cannot be released.
- Shadow state is stale, degraded, ambiguous, or disconnected.
- Shadow stream/account environment does not match active `mainnet`.
- Available mainnet balance is zero or insufficient for required margin.
- Exchange min quantity makes the smallest current notional exceed the approved canary max-notional cap.
- Account/exchange leverage for the active symbol is above the approved canary maximum and RelXen cannot safely adjust it.
- Account mode is hedge mode or unknown.
- Multi-assets mode is enabled or unknown.
- Risk profile is missing, too broad, or unreviewed.
- Preview hash, symbol, side, quantity, or price has changed since operator review.
- Preview is `MARKET`, rounded marketable, or lacks a fresh reference price.
- Reference price becomes unavailable or stale after kill-switch release and cannot be refreshed from the explicit resolver.
- Any recent order is `unknown_needs_repair`, `submit_pending`, or otherwise unreconciled.
- Recent-window repair cannot prove current order/fill state.
- Operator cannot explain the difference between preflight, ACK, working, partial fill, fill, cancel, and flatten.

## Canary Execution Checklist

1. Start server with the canary flag enabled only for the session.
2. Confirm `/api/live/status` reports `mainnet_canary.enabled_by_server=true`.
3. Select the mainnet credential and validate it.
4. Configure the conservative risk profile.
5. Start shadow sync and refresh readiness.
6. Confirm one-way and single-asset mode.
7. Build the smallest acceptable non-marketable `LIMIT` preview and verify min quantity, min notional, required margin, and max-notional cap together.
8. Confirm `mainnet_canary_ready` and exact confirmation text.
9. Submit one manual canary order only.
10. Wait for authoritative reconciliation before any follow-up action.
11. Export evidence immediately after the action.
12. Disable the canary flag and restart back to the default blocked state.

## Rollback Procedure

- Engage kill switch.
- Cancel active-symbol open orders if shadow state is coherent.
- Flatten active-symbol position only if position state is deterministic and gates allow it.
- Stop TESTNET auto mode if running.
- Disable `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION`.
- Restart server and verify `mainnet_execution_blocked`.
- Preserve evidence bundle and logs for review.

## Required Evidence For Go

- Current TESTNET soak evidence bundle.
- Mainnet status before and after canary.
- Orders and fills exported after canary.
- Any repair/degradation logs.
- Operator note confirming no conditional/algo order was used.
- Operator note confirming no MAINNET auto-execution path was enabled.
- For further canaries, cancel evidence should prove the route path order reference and exact confirmation are handled without requiring duplicated body fields.
