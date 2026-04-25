use crate::models::{AsoPositionPolicy, MainnetAutoDesiredSide, MainnetAutoPolicyAction};

#[derive(Debug, Clone, PartialEq)]
pub struct AsoPolicyInput {
    pub policy: AsoPositionPolicy,
    pub bulls: Option<f64>,
    pub bears: Option<f64>,
    pub delta_threshold: f64,
    pub zone_threshold: f64,
    pub current_side: MainnetAutoDesiredSide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsoPolicyDecision {
    pub policy: AsoPositionPolicy,
    pub bulls: Option<f64>,
    pub bears: Option<f64>,
    pub delta: Option<f64>,
    pub zone: Option<f64>,
    pub desired_side: MainnetAutoDesiredSide,
    pub current_side: MainnetAutoDesiredSide,
    pub action: MainnetAutoPolicyAction,
    pub blocker: Option<String>,
    pub reason: String,
}

pub fn evaluate_aso_position_policy(input: AsoPolicyInput) -> AsoPolicyDecision {
    let base = AsoPolicyDecision {
        policy: input.policy,
        bulls: input.bulls,
        bears: input.bears,
        delta: None,
        zone: None,
        desired_side: MainnetAutoDesiredSide::None,
        current_side: input.current_side,
        action: MainnetAutoPolicyAction::NoTrade,
        blocker: None,
        reason: "no_trade".to_string(),
    };

    if input.policy == AsoPositionPolicy::CrossoverOnly {
        return AsoPolicyDecision {
            reason: "crossover_only_waits_for_new_closed_candle_cross".to_string(),
            ..base
        };
    }

    let Some((bulls, bears)) = valid_aso_pair(input.bulls, input.bears) else {
        return AsoPolicyDecision {
            blocker: Some("aso_state_invalid".to_string()),
            reason: "aso_state_invalid".to_string(),
            ..base
        };
    };

    let delta = (bulls - bears).abs();
    let zone = bulls.max(bears);
    let mut decision = AsoPolicyDecision {
        bulls: Some(bulls),
        bears: Some(bears),
        delta: Some(delta),
        zone: Some(zone),
        ..base
    };

    let desired_side = match input.policy {
        AsoPositionPolicy::CrossoverOnly => MainnetAutoDesiredSide::None,
        AsoPositionPolicy::AlwaysInMarket => desired_from_aso_state(bulls, bears),
        AsoPositionPolicy::FlatAllowed => {
            if delta < input.delta_threshold || zone < input.zone_threshold {
                MainnetAutoDesiredSide::None
            } else {
                desired_from_aso_state(bulls, bears)
            }
        }
    };
    decision.desired_side = desired_side;

    if desired_side == MainnetAutoDesiredSide::None {
        decision.action = if input.current_side == MainnetAutoDesiredSide::None {
            MainnetAutoPolicyAction::NoTrade
        } else {
            MainnetAutoPolicyAction::Hold
        };
        decision.reason = match input.policy {
            AsoPositionPolicy::AlwaysInMarket => "aso_state_equal_or_ambiguous".to_string(),
            AsoPositionPolicy::FlatAllowed if delta < input.delta_threshold => {
                "flat_allowed_delta_below_threshold".to_string()
            }
            AsoPositionPolicy::FlatAllowed => "flat_allowed_zone_below_threshold".to_string(),
            AsoPositionPolicy::CrossoverOnly => {
                "crossover_only_waits_for_new_closed_candle_cross".to_string()
            }
        };
        return decision;
    }

    decision.action = match (input.current_side, desired_side) {
        (MainnetAutoDesiredSide::None, MainnetAutoDesiredSide::Long) => {
            MainnetAutoPolicyAction::EnterLong
        }
        (MainnetAutoDesiredSide::None, MainnetAutoDesiredSide::Short) => {
            MainnetAutoPolicyAction::EnterShort
        }
        (current, desired) if current == desired => MainnetAutoPolicyAction::Hold,
        (_, _) => MainnetAutoPolicyAction::Reverse,
    };
    decision.reason = match decision.action {
        MainnetAutoPolicyAction::EnterLong => "desired_long_from_aso_state".to_string(),
        MainnetAutoPolicyAction::EnterShort => "desired_short_from_aso_state".to_string(),
        MainnetAutoPolicyAction::Hold => "desired_side_matches_current_position".to_string(),
        MainnetAutoPolicyAction::Reverse => {
            "desired_side_differs_from_current_position".to_string()
        }
        MainnetAutoPolicyAction::Close
        | MainnetAutoPolicyAction::NoTrade
        | MainnetAutoPolicyAction::Blocked => "no_trade".to_string(),
    };
    decision
}

fn valid_aso_pair(bulls: Option<f64>, bears: Option<f64>) -> Option<(f64, f64)> {
    let bulls = bulls?;
    let bears = bears?;
    if bulls.is_finite() && bears.is_finite() {
        Some((bulls, bears))
    } else {
        None
    }
}

fn desired_from_aso_state(bulls: f64, bears: f64) -> MainnetAutoDesiredSide {
    if bulls > bears {
        MainnetAutoDesiredSide::Long
    } else if bears > bulls {
        MainnetAutoDesiredSide::Short
    } else {
        MainnetAutoDesiredSide::None
    }
}
