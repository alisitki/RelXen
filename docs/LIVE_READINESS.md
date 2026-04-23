# Live Readiness

## Purpose

This document is the top-level entrypoint for post-v1 live-trading work. It exists to keep live execution bounded: RelXen now has credential, validation, account snapshot, symbol-rule, readiness, user-data shadow reconciliation, precision-aware intent preview, testnet preflight foundations, and constrained TESTNET-only placement/cancel/flatten. MAINNET execution remains blocked.

## Current Repo Truth

- Paper Mode V1 is release-candidate complete.
- Paper mode remains complete and isolated.
- Live foundations are implemented for credential metadata, OS secure storage, signed read-only validation, account snapshots, active-symbol rules, readiness checks, arming, user-data shadow sync, order-intent preview, testnet `order/test` preflight, and constrained TESTNET-only order/cancel/flatten.
- MAINNET order execution is not implemented.
- The repository can place TESTNET matching-engine `MARKET` / `LIMIT` orders only after explicit operator confirmation and fail-closed gates.
- The repository stores live credential metadata in SQLite and raw secrets through the OS secure-storage abstraction only.

## Design Documents

- [Paper Mode V1 Release Status](./V1_RELEASE_STATUS.md)
- [Live Execution Boundary](./LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](./SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](./PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](./LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](./LIVE_IMPLEMENTATION_PLAN.md)
- [Architecture](./ARCHITECTURE.md)
- [Runbook](./RUNBOOK.md)

## Glossary

- Paper mode: Local simulated execution using market data, local wallets, local positions, and no real orders.
- Live mode: Exchange-connected mode. In this repository state, actual execution is constrained to TESTNET only; MAINNET remains blocked.
- Validation: A precondition check that proves a credential, rule set, account snapshot, setting, or intent is safe enough to proceed.
- Order intent: Internal instruction describing desired side, quantity, reduce-only behavior, and reason before exchange formatting.
- Preflight: Binance testnet `order/test` validation of a signed order payload. It validates request shape and exchange acceptance rules but does not place an order.
- Shadow state: Read-only best-effort account, position, order, and stream state reconstructed from REST and user-data events.
- Execution: The act of sending an order request to an exchange adapter. Current execution is TESTNET-only and operator-confirmed.
- Reconciliation: Comparing exchange order, fill, account, and position state against local state until live truth is known.
- Precision: Numeric representation and rounding discipline for prices, quantities, fees, notional, and PnL.
- Risk gate: A blocking policy check that must pass before arming, starting, or submitting an order.
- Fail-closed: Default behavior that blocks execution when state is missing, ambiguous, stale, or invalid.
- Armed/disarmed: Operator-controlled live readiness state. Disarmed means no live order may be placed.
- Kill switch: Operator or system action that immediately blocks new live orders and starts a safe stop/flatten process when configured.
- `resync_required`: Existing market-data event telling the frontend to reload a fresh snapshot because deterministic delta continuity cannot be proven.

## Reusable From Paper Architecture

- Candle ingestion and bounded historical loading.
- ASO calculation and closed-candle crossover signal generation.
- Settings validation and runtime orchestration patterns.
- Snapshot plus WebSocket delta model.
- SQLite persistence foundations for non-secret metadata and audit records.
- Operator UI shell and text-first status discipline.
- Reconnect recovery concept for market-data continuity.
- Live credential/readiness foundations, shadow status, and existing websocket status-event plumbing.
- Testnet execution records and fill reconciliation patterns for future broader execution work.

## Must Not Be Reused Blindly From Paper Mode

- Paper engine wallet and position state must not become live account truth.
- `f64` math must not be the final live execution truth model.
- Paper sizing must not be sent to the exchange without symbol-rule validation.
- Signal events must not directly become exchange orders.
- Local persisted state must not override exchange-reconciled account or position state.
- Preflight success must not be treated as an order placement or live position mutation.
- TESTNET execution evidence must not be generalized to MAINNET without a separate mainnet-readiness slice.

## Intentionally Deferred

- Mainnet order submission and cancel execution.
- Conditional/algo orders.
- Autonomous strategy-driven testnet/live execution.
- Multi-symbol concurrent runtime.
- Broker-grade audit reporting.
- Advanced order types beyond the first constrained live slice.

## Design Rule

Future live implementation must proceed in small slices. The next implementation task is an explicit kill switch and strategy-driven closed-candle TESTNET auto-executor behind arming, duplicate-signal suppression, and exchange-authoritative fill reconciliation. It must not add mainnet placement.
