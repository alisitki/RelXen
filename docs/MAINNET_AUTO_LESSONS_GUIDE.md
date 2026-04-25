# Mainnet Auto Lessons Guide

## Purpose

MAINNET auto lesson reports explain what happened during a dry-run or session-scoped live run. They are analysis artifacts only. They must not automatically change strategy settings, risk budget, live enablement, leverage, symbols, order types, or credentials.

## Outputs

Each mainnet-auto evidence export writes:

- `lessons.md`: human-readable summary
- `lessons.json`: structured summary for tooling

The report includes:

- session mode: `dry_run` or `live`
- whether a live order was submitted
- position policy used
- desired-side evaluation count
- enter/hold/reverse/no-trade decision counts
- margin-type block count
- signals observed
- blocked decisions
- would-submit decisions
- duplicate suppressions
- top blockers
- watchdog stop reason, if any
- risk budget utilization
- reference price freshness summary
- ASO signal summary
- stale or degraded state notes
- checks before the next run
- recommendation

## Recommendation Meanings

- `safe_to_repeat_dry_run`: the session did not expose a blocker that prevents another dry-run.
- `needs_fix_before_live`: one or more gates blocked the session and should be reviewed before any live trial.
- `live_not_allowed`: live execution is not permitted by current config or safety state.
- `ready_for_explicit_live_trial`: dry-run produced a would-submit decision, but this is not authorization to trade. A separate explicit live-run task is still required.

The 2026-04-24 operator-DB dry-run generated `ready_for_explicit_live_trial` from the lesson generator after one `dry_run_would_submit` decision. For operator handoff, interpret that as `ready_to_prepare_explicit_live_auto_plan`, not approval to enable live MAINNET auto.

Mainnet Auto Live Support v1 generated live-session lessons for the 2026-04-25 approved 15-minute `BTCUSDT` run. The report recorded `mode=live`, zero signals, zero decisions, zero live orders, zero fills, watchdog stop `max_runtime_reached`, realized PnL `0`, fees `0`, final flat state, and recommendation `safe_to_repeat_dry_run`. That recommendation means review or dry-run can be repeated first; it does not authorize another live run automatically.

Mainnet Auto Policy Support v1 expands lesson context with `position_policy`, desired-side evaluation counts, action counts, and margin-type blockers. These fields explain whether `crossover_only`, `always_in_market`, or `flat_allowed` was active and whether cross/isolated policy affected the run. They remain analysis only.

## Review Checklist

Before considering any future live MAINNET auto task, confirm:

- `orders.json` and `fills.json` are empty for dry-run sessions
- live order submitted is `false` for dry-run
- blockers are understood
- duplicate suppression is working
- risk budget is conservative
- reference price is fresh enough for the intended order type
- account/shadow/rules state was fresh
- watchdog did not stop for an unresolved safety issue
- lesson recommendations were not applied automatically
- margin policy did not silently allow `cross` when `isolated` was required
- `always_in_market` activity or `flat_allowed` filtering is reviewed before repeating a live task

## Prohibited Uses

Do not use a lesson report to:

- auto-enable live MAINNET auto
- auto-change ASO settings
- auto-increase risk budget
- auto-change leverage
- add heatmap/liquidation context as a decision layer
- widen symbols beyond `BTCUSDT` / `BTCUSDC`
- authorize conditional/algo orders

The report is evidence for human review, not a control plane.
