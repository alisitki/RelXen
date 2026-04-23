use std::collections::BTreeMap;
use std::str::FromStr;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    LiveAccountShadow, LiveBlockingReason, LiveEnvironment, LiveOrderIntent, LiveOrderPreview,
    LiveOrderSide, LiveOrderSizingBreakdown, LiveOrderType, LiveSymbolRules, Settings, SignalEvent,
    SignalSide, Symbol,
};

#[derive(Debug, Clone)]
pub struct LiveIntentInput {
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub settings: Settings,
    pub rules: LiveSymbolRules,
    pub shadow: LiveAccountShadow,
    pub latest_signal: Option<SignalEvent>,
    pub order_type: LiveOrderType,
    pub reference_price: Decimal,
    pub limit_price: Option<Decimal>,
    pub now_ms: i64,
}

pub fn build_live_order_preview(input: LiveIntentInput) -> LiveOrderPreview {
    let mut blocking = Vec::new();
    let mut errors = Vec::new();
    let mut notes = Vec::new();

    if input.rules.status != "TRADING" {
        blocking.push(LiveBlockingReason::RulesMissing);
        errors.push(format!("symbol status is {}", input.rules.status));
    }
    if input.symbol != Symbol::BtcUsdt && input.symbol != Symbol::BtcUsdc {
        blocking.push(LiveBlockingReason::UnsupportedSymbol);
    }
    if input.shadow.ambiguous {
        blocking.push(LiveBlockingReason::ShadowStateAmbiguous);
    }
    if input.shadow.multi_assets_margin.unwrap_or(false) {
        blocking.push(LiveBlockingReason::UnsupportedAccountMode);
        errors.push("multi-assets margin mode is unsupported for preflight".to_string());
    }
    if input
        .shadow
        .positions
        .iter()
        .any(|position| position.position_side != "BOTH")
    {
        blocking.push(LiveBlockingReason::UnsupportedAccountMode);
        errors.push("hedge mode position side is unsupported for preflight".to_string());
    }

    let signal = input.latest_signal.clone();
    let side = match signal.as_ref().map(|signal| signal.side) {
        Some(SignalSide::Sell) => LiveOrderSide::Sell,
        Some(SignalSide::Buy) | None => LiveOrderSide::Buy,
    };
    if signal.is_none() {
        notes.push(
            "no closed signal available; preview uses explicit operator BUY side".to_string(),
        );
    };

    let price = match input.order_type {
        LiveOrderType::Market => input.reference_price,
        LiveOrderType::Limit => {
            let Some(limit_price) = input.limit_price else {
                blocking.push(LiveBlockingReason::IntentUnavailable);
                errors.push("limit order preview requires limit_price".to_string());
                return blocked_preview(input.now_ms, blocking, errors);
            };
            let tick = decimal_from_option(input.rules.filters.tick_size, "tick_size", &mut errors);
            let rounded = tick
                .map(|tick| quantize_price(limit_price, tick, side))
                .unwrap_or(limit_price);
            if rounded != limit_price {
                notes.push(format!(
                    "limit price rounded from {limit_price} to {rounded}"
                ));
            }
            rounded
        }
    };

    let fixed_notional = decimal_from_f64(input.settings.fixed_notional);
    let leverage = decimal_from_f64(input.settings.leverage).max(Decimal::ONE);
    let available_balance =
        quote_available_balance(&input.shadow, input.symbol).unwrap_or(Decimal::ZERO);
    let raw_quantity = if price > Decimal::ZERO {
        fixed_notional / price
    } else {
        Decimal::ZERO
    };
    let step = decimal_from_option(input.rules.filters.step_size, "step_size", &mut errors);
    let rounded_quantity = step
        .map(|step| quantize_down(raw_quantity, step))
        .unwrap_or(raw_quantity);
    if rounded_quantity != raw_quantity {
        notes.push(format!(
            "quantity rounded down from {raw_quantity} to {rounded_quantity}"
        ));
    }

    let estimated_notional = rounded_quantity * price;
    let required_margin = estimated_notional / leverage;
    if required_margin > available_balance {
        blocking.push(LiveBlockingReason::IntentUnavailable);
        errors.push("required margin exceeds available live shadow balance".to_string());
    }
    if rounded_quantity <= Decimal::ZERO {
        blocking.push(LiveBlockingReason::PrecisionInvalid);
        errors.push("rounded quantity is zero".to_string());
    }
    if let Some(min_qty) = decimal_from_option(input.rules.filters.min_qty, "min_qty", &mut errors)
    {
        if rounded_quantity < min_qty {
            blocking.push(LiveBlockingReason::PrecisionInvalid);
            errors.push(format!(
                "quantity {rounded_quantity} is below min qty {min_qty}"
            ));
        }
    }
    if let Some(min_notional) = decimal_from_option(
        input.rules.filters.min_notional,
        "min_notional",
        &mut errors,
    ) {
        if estimated_notional < min_notional {
            blocking.push(LiveBlockingReason::MinNotional);
            errors.push(format!(
                "estimated notional {estimated_notional} is below min notional {min_notional}"
            ));
        }
    }

    blocking.sort_by_key(|reason| reason.as_str());
    blocking.dedup();

    if !blocking.is_empty() {
        return blocked_preview(input.now_ms, blocking, errors);
    }

    let quantity = decimal_to_exchange_string(rounded_quantity);
    let price_string = if input.order_type == LiveOrderType::Limit {
        Some(decimal_to_exchange_string(price))
    } else {
        None
    };
    let mut payload = BTreeMap::new();
    payload.insert("symbol".to_string(), input.symbol.as_str().to_string());
    payload.insert("side".to_string(), side.as_binance().to_string());
    payload.insert(
        "type".to_string(),
        input.order_type.as_binance().to_string(),
    );
    payload.insert("quantity".to_string(), quantity.clone());
    if let Some(price) = price_string.clone() {
        payload.insert("timeInForce".to_string(), "GTC".to_string());
        payload.insert("price".to_string(), price);
    }

    let intent_hash = intent_hash(IntentHashInput {
        environment: input.environment,
        symbol: input.symbol,
        side,
        order_type: input.order_type,
        quantity: &quantity,
        price: price_string.as_deref(),
        reduce_only: false,
        source_signal_id: signal.as_ref().map(|signal| signal.id.as_str()),
    });

    let intent = LiveOrderIntent {
        id: Uuid::new_v4().to_string(),
        intent_hash,
        environment: input.environment,
        symbol: input.symbol,
        side,
        order_type: input.order_type,
        quantity,
        price: price_string,
        reduce_only: false,
        time_in_force: if input.order_type == LiveOrderType::Limit {
            Some("GTC".to_string())
        } else {
            None
        },
        source_signal_id: signal.as_ref().map(|signal| signal.id.clone()),
        source_open_time: signal.as_ref().map(|signal| signal.open_time),
        reason: if signal.is_some() {
            "latest_closed_signal".to_string()
        } else {
            "operator_preview".to_string()
        },
        exchange_payload: payload,
        sizing: LiveOrderSizingBreakdown {
            requested_notional: decimal_to_exchange_string(fixed_notional),
            available_balance: decimal_to_exchange_string(available_balance),
            leverage: decimal_to_exchange_string(leverage),
            required_margin: decimal_to_exchange_string(required_margin),
            raw_quantity: decimal_to_exchange_string(raw_quantity),
            rounded_quantity: decimal_to_exchange_string(rounded_quantity),
            estimated_notional: decimal_to_exchange_string(estimated_notional),
        },
        validation_notes: notes,
        blocking_reasons: Vec::new(),
        can_preflight: input.environment == LiveEnvironment::Testnet,
        can_execute_now: input.environment == LiveEnvironment::Testnet,
        built_at: input.now_ms,
    };

    let mut preview_blocking = Vec::new();
    let mut message =
        "TESTNET PREVIEW READY. Actual placement is testnet-only and gated.".to_string();
    if input.environment != LiveEnvironment::Testnet {
        preview_blocking.push(LiveBlockingReason::PreflightNotSupportedOnMainnet);
        message = "Preflight is testnet-only in this batch.".to_string();
    }

    LiveOrderPreview {
        built_at: input.now_ms,
        intent: Some(intent),
        blocking_reasons: preview_blocking,
        validation_errors: errors,
        message,
    }
}

struct IntentHashInput<'a> {
    environment: LiveEnvironment,
    symbol: Symbol,
    side: LiveOrderSide,
    order_type: LiveOrderType,
    quantity: &'a str,
    price: Option<&'a str>,
    reduce_only: bool,
    source_signal_id: Option<&'a str>,
}

fn intent_hash(input: IntentHashInput<'_>) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        input.environment.as_str(),
        input.symbol.as_str(),
        input.side.as_binance(),
        input.order_type.as_binance(),
        input.quantity,
        input.price.unwrap_or("-"),
        input.reduce_only,
        input.source_signal_id.unwrap_or("-")
    )
}

pub fn quantize_down(value: Decimal, step: Decimal) -> Decimal {
    if step <= Decimal::ZERO {
        return value;
    }
    (value / step).floor() * step
}

pub fn quantize_price(value: Decimal, tick: Decimal, side: LiveOrderSide) -> Decimal {
    if tick <= Decimal::ZERO {
        return value;
    }
    let units = value / tick;
    match side {
        LiveOrderSide::Buy => units.floor() * tick,
        LiveOrderSide::Sell => units.ceil() * tick,
    }
}

fn blocked_preview(
    built_at: i64,
    mut blocking_reasons: Vec<LiveBlockingReason>,
    validation_errors: Vec<String>,
) -> LiveOrderPreview {
    blocking_reasons.sort_by_key(|reason| reason.as_str());
    blocking_reasons.dedup();
    LiveOrderPreview {
        built_at,
        intent: None,
        blocking_reasons,
        validation_errors,
        message: "PREFLIGHT BLOCKED. No exchange request was sent.".to_string(),
    }
}

fn quote_available_balance(shadow: &LiveAccountShadow, symbol: Symbol) -> Option<Decimal> {
    let quote = symbol.quote_asset().as_str();
    shadow
        .balances
        .iter()
        .find(|balance| balance.asset == quote)
        .and_then(|balance| Decimal::from_str(&balance.wallet_balance).ok())
}

fn decimal_from_option(
    value: Option<f64>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<Decimal> {
    value.and_then(|value| {
        Decimal::from_str(&value.to_string())
            .map_err(|error| {
                errors.push(format!("invalid {label}: {error}"));
                error
            })
            .ok()
    })
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_str(&value.to_string()).unwrap_or(Decimal::ZERO)
}

fn decimal_to_exchange_string(value: Decimal) -> String {
    value
        .normalize()
        .to_string()
        .trim_end_matches(".0")
        .to_string()
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{quantize_down, quantize_price};
    use crate::LiveOrderSide;

    #[test]
    fn quantity_rounds_down_to_step() {
        assert_eq!(
            quantize_down(Decimal::new(1234567, 8), Decimal::new(1, 3)).to_string(),
            "0.012"
        );
    }

    #[test]
    fn price_rounds_by_side_to_tick() {
        assert_eq!(
            quantize_price(
                Decimal::new(100054, 2),
                Decimal::new(1, 1),
                LiveOrderSide::Buy
            )
            .to_string(),
            "1000.5"
        );
        assert_eq!(
            quantize_price(
                Decimal::new(100054, 2),
                Decimal::new(1, 1),
                LiveOrderSide::Sell
            )
            .to_string(),
            "1000.6"
        );
    }
}
