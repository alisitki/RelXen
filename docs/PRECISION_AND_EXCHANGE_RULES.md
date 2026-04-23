# Precision And Exchange Rules

## Paper V1 Numeric Status

RelXen Paper Mode V1 uses `f64` for simulated prices, quantities, fees, wallet balances, and PnL. That is acceptable for local paper trading because no exchange order is placed and the results are explicitly simulated.

`f64` is insufficient as final live-trading truth because exchange APIs enforce decimal grids, minimums, and rounding behavior. Binary floating-point can create values that look valid in the UI but are rejected by the exchange or rounded in an unsafe direction.

## Current Live-Intent Numeric Strategy

Paper mode remains `f64`. Live intent/preflight code now uses decimal arithmetic for execution-critical preview math, isolated from the paper engine:

- Decimal values are used for order quantity, price, notional, required margin, available balance checks, and exchange-rule rounding.
- Fixed-point integers remain a future option if the executor needs stricter tick/step storage internally.

Any future placement/cancel slice must continue using strict decimal or fixed-point handling for:

- order quantity
- order price when applicable
- notional
- fees
- realized PnL
- wallet balances used for risk checks
- exchange-rule rounding

Approximate/display math may remain acceptable for charts, percentages, UI summaries, and non-authoritative visual labels.

## Exchange Rule Surfaces

Future live execution must respect:

- Tick size for price.
- Step size for quantity.
- Minimum quantity.
- Minimum notional.
- Maximum quantity or notional if exposed by the exchange.
- Leverage brackets and max leverage for the chosen notional.
- Margin mode and position mode constraints.
- Reduce-only semantics if supported and used.
- Order type support per symbol/environment.
- Quantity and price rounding rules.

## Rounding Policy Guidance

- Entry quantity: Round down to the nearest valid step so required margin and notional do not exceed available balance.
- Exit quantity: Round to a valid step without leaving unintended dust; final flatten may require exchange-specific close-position or reduce-only handling.
- Price: Round to the nearest safe tick according to order side and order type. Do not round in a direction that increases risk unexpectedly.
- Fee calculation: Use exchange-reported fee for final truth. Estimate conservatively before order submission.
- PnL calculation: Use fills and exchange fees for final live truth.
- Wallet display: UI may format rounded decimal strings, but risk gates must use strict internal values.

## Symbol-Rules Cache

The symbol-rules provider belongs in `crates/infra` behind an app-level port. The app layer should validate settings and order intents against a fresh symbol-rules snapshot before live arming and before order submission.

Refresh expectations:

- Load rules at startup for the active symbol.
- Refresh before arming live mode.
- Refresh after exchange reconnect or validation failure.
- Expire rules after a configured TTL.

If rules are missing, stale, or inconsistent, live mode must fail closed.

## Completed Live-Intent Scope

The live-shadow/preflight and constrained testnet-executor slices include:

- Decimal type choice for intent/preflight math.
- Symbol-rules fetch and cache for `BTCUSDT` and `BTCUSDC`.
- Quantity and notional validators.
- Rounding helpers with unit tests.
- Fail-closed behavior when rules are unavailable.
- TESTNET-only `MARKET` / `LIMIT` execution payload construction from decimal intent values.
- TESTNET cancel/flatten flows that preserve decimal quantity formatting and reduce-only close intent construction.

It intentionally still defers:

- MAINNET order placement and cancel execution.
- Conditional/algo order precision policy.
- Multi-symbol rule caches.
- Advanced bracket simulation.
- Portfolio-level margin modeling beyond the active symbol.

## Dangerous Mistakes To Prevent

- Rounding an entry quantity up and exceeding available margin.
- Sending `0.0010000000000000002` when the symbol step size allows only `0.001`.
- Passing a minimum quantity check but failing minimum notional after price movement.
- Treating estimated paper fees as final live fees.
- Closing a rounded quantity that is slightly larger than the live position.
- Reusing stale tick size after the exchange updates symbol filters.
- Computing PnL from candle close instead of actual fill prices.
- Building a reduce-only order without verifying exchange support and position mode.
