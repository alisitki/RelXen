# Live Readiness

## Purpose

This document is the top-level entrypoint for post-v1 live-trading work. It exists to keep live execution bounded: RelXen now has credential, validation, account snapshot, symbol-rule, readiness, user-data shadow reconciliation, precision-aware intent preview, testnet preflight foundations, constrained TESTNET placement/cancel/flatten/auto-execution, kill switch controls, and a manual MAINNET canary path that is disabled by default.

## Current Repo Truth

- Paper Mode V1 is release-candidate complete.
- Paper mode remains complete and isolated.
- Live foundations are implemented for credential metadata, OS secure storage, signed read-only validation, account snapshots, active-symbol rules, readiness checks, arming, user-data shadow sync, order-intent preview, testnet `order/test` preflight, constrained TESTNET order/cancel/flatten, closed-candle TESTNET auto-execution, and manual MAINNET canary execution behind explicit canary gates.
- MAINNET execution is disabled by default and MAINNET auto-execution is not implemented.
- The repository can place TESTNET matching-engine `MARKET` / `LIMIT` orders only after explicit operator confirmation or explicit TESTNET auto start, with fail-closed gates.
- The repository can place a MAINNET canary `MARKET` / `LIMIT` order only when `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true`, a risk profile is configured, exact confirmation text is entered, and all execution/reconciliation gates pass.
- The repository stores live credential metadata in SQLite and raw secrets through the OS secure-storage abstraction only.

## Design Documents

- [Paper Mode V1 Release Status](./V1_RELEASE_STATUS.md)
- [Live Execution Boundary](./LIVE_EXECUTION_BOUNDARY.md)
- [Secret Storage Plan](./SECRET_STORAGE_PLAN.md)
- [Precision And Exchange Rules](./PRECISION_AND_EXCHANGE_RULES.md)
- [Live Risk Controls](./LIVE_RISK_CONTROLS.md)
- [Live Implementation Plan](./LIVE_IMPLEMENTATION_PLAN.md)
- [Testnet Soak Runbook](./TESTNET_SOAK_RUNBOOK.md)
- [Latest Testnet Soak Report](./LATEST_TESTNET_SOAK_REPORT.md)
- [Mainnet Canary Checklist](./MAINNET_CANARY_CHECKLIST.md)
- [Architecture](./ARCHITECTURE.md)
- [Runbook](./RUNBOOK.md)

## Glossary

- Paper mode: Local simulated execution using market data, local wallets, local positions, and no real orders.
- Live mode: Exchange-connected mode. In this repository state, TESTNET execution and default-off manual MAINNET canary execution exist; MAINNET auto remains blocked.
- Validation: A precondition check that proves a credential, rule set, account snapshot, setting, or intent is safe enough to proceed.
- Order intent: Internal instruction describing desired side, quantity, reduce-only behavior, and reason before exchange formatting.
- Preflight: Binance testnet `order/test` validation of a signed order payload. It validates request shape and exchange acceptance rules but does not place an order.
- Shadow state: Read-only best-effort account, position, order, and stream state reconstructed from REST and user-data events.
- Execution: The act of sending an order request to an exchange adapter. Current execution is operator-confirmed for TESTNET and default-off manual MAINNET canary; MAINNET auto remains blocked.
- Reconciliation: Comparing exchange order, fill, account, and position state against local state until live truth is known.
- Precision: Numeric representation and rounding discipline for prices, quantities, fees, notional, and PnL.
- Risk gate: A blocking policy check that must pass before arming, starting, or submitting an order.
- Fail-closed: Default behavior that blocks execution when state is missing, ambiguous, stale, or invalid.
- Armed/disarmed: Operator-controlled live readiness state. Disarmed means no live order may be placed.
- Kill switch: Operator or system action that immediately blocks new live orders and allows only safe recovery actions when deterministic.
- Canary gate: Explicit server-side and operator-side controls required before a manual MAINNET canary action can be submitted.
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
- TESTNET execution evidence must not be generalized to broad MAINNET operation. Manual MAINNET canary support is default-off and must remain bounded by canary-specific gates and exact confirmations.

## Intentionally Deferred

- Broad mainnet operation beyond manual canary execution.
- Conditional/algo orders.
- MAINNET auto-execution.
- Multi-symbol concurrent runtime.
- Broker-grade audit reporting.
- Advanced order types beyond the first constrained live slice.
- MAINNET canary operation before a real TESTNET soak evidence bundle is captured and reviewed.

## Design Rule

Future live implementation must proceed in small slices. The next implementation task is creating/selecting a valid TESTNET credential through the secure-store flow, then running the documented real TESTNET soak drill and attaching the generated evidence bundle to the latest soak report without enabling mainnet.
