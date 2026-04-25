pub mod aso;
pub mod live_intent;
pub mod mainnet_auto_policy;
pub mod models;
pub mod paper;
pub mod risk;
pub mod signals;

pub use aso::{compute_aso_series, warmup_candles_required, AsoCalculator};
pub use live_intent::{build_live_order_preview, quantize_down, quantize_price, LiveIntentInput};
pub use mainnet_auto_policy::{evaluate_aso_position_policy, AsoPolicyDecision, AsoPolicyInput};
pub use models::*;
pub use paper::{
    compute_performance, mark_to_market, open_position_size, reset_wallets, PaperEngine,
};
pub use risk::validate_settings;
pub use signals::{derive_signal_history, signal_from_points};
