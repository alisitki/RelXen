use crate::models::{Settings, ALLOWED_SYMBOLS};

pub fn validate_settings(settings: &Settings) -> Result<(), String> {
    if settings.available_symbols.is_empty() {
        return Err("at least one symbol must be available".to_string());
    }

    for symbol in &settings.available_symbols {
        if !ALLOWED_SYMBOLS.contains(symbol) {
            return Err(format!("symbol {symbol} is not allowed in v1"));
        }
    }

    if !settings.available_symbols.contains(&settings.active_symbol) {
        return Err("active symbol must be part of available_symbols".to_string());
    }

    if settings.aso_length < 2 {
        return Err("aso_length must be >= 2".to_string());
    }

    if settings.leverage <= 0.0 {
        return Err("leverage must be > 0".to_string());
    }

    if !(0.0..1.0).contains(&settings.fee_rate) {
        return Err("fee_rate must be >= 0 and < 1".to_string());
    }

    if settings.fixed_notional <= 0.0 {
        return Err("fixed_notional must be > 0".to_string());
    }

    Ok(())
}
