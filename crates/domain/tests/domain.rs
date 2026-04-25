use std::collections::BTreeMap;

use relxen_domain::{
    compute_aso_series, compute_performance, derive_signal_history, evaluate_aso_position_policy,
    mark_to_market, open_position_size, reset_wallets, signal_from_points, AsoCalculator, AsoMode,
    AsoPolicyInput, AsoPositionPolicy, Candle, LiveMarginType, MainnetAutoAllowedMarginType,
    MainnetAutoDesiredSide, MainnetAutoPolicyAction, PaperEngine, PositionSide, QuoteAsset,
    Settings, SignalEvent, SignalSide, Symbol, Timeframe, TradeAction,
};

fn candle(index: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        symbol: Symbol::BtcUsdt,
        timeframe: Timeframe::M1,
        open_time: index * 60_000,
        close_time: (index + 1) * 60_000 - 1,
        open,
        high,
        low,
        close,
        volume: 1.0,
        closed: true,
    }
}

#[test]
fn aso_intrabar_mode_warms_and_outputs_complementary_values() {
    let candles = vec![
        candle(0, 10.0, 12.0, 9.0, 11.0),
        candle(1, 11.0, 13.0, 10.0, 12.0),
        candle(2, 12.0, 14.0, 11.0, 13.5),
    ];
    let points = compute_aso_series(&candles, 3, AsoMode::Intrabar);
    assert_eq!(points.len(), 3);
    assert!(!points[1].ready);
    assert!(points[2].ready);
    let bulls = points[2].bulls.unwrap();
    let bears = points[2].bears.unwrap();
    assert!((bulls + bears - 100.0).abs() < 0.0001);
    assert!(bulls > bears);
}

#[test]
fn aso_group_mode_uses_group_window() {
    let candles = vec![
        candle(0, 10.0, 11.0, 9.0, 10.5),
        candle(1, 10.5, 12.0, 10.0, 11.5),
        candle(2, 11.5, 13.0, 11.0, 12.5),
        candle(3, 12.5, 14.0, 12.0, 13.0),
        candle(4, 13.0, 15.0, 12.5, 14.5),
    ];
    let points = compute_aso_series(&candles, 3, AsoMode::Group);
    assert!(!points[3].ready);
    assert!(points[4].ready);
    let bulls = points[4].bulls.unwrap();
    let bears = points[4].bears.unwrap();
    assert!((bulls + bears - 100.0).abs() < 0.0001);
    assert!(bulls > bears);
}

#[test]
fn aso_both_mode_uses_intrabar_and_group() {
    let candles = vec![
        candle(0, 10.0, 12.0, 9.0, 11.0),
        candle(1, 11.0, 13.0, 10.0, 12.0),
        candle(2, 12.0, 14.0, 11.0, 13.0),
        candle(3, 13.0, 15.0, 12.0, 14.0),
        candle(4, 14.0, 16.0, 13.0, 15.0),
    ];
    let points = compute_aso_series(&candles, 3, AsoMode::Both);
    assert_eq!(points.iter().filter(|point| point.ready).count(), 1);
    let last = points.last().unwrap();
    assert!(last.ready);
    assert!(last.bulls.unwrap() > last.bears.unwrap());
}

#[test]
fn aso_warmup_behavior_matches_two_length_minus_one_for_both() {
    let mut calculator = AsoCalculator::new(4, AsoMode::Both);
    let mut last_ready = false;
    for index in 0..7 {
        let point = calculator.push_closed(candle(
            index,
            10.0 + index as f64,
            12.0 + index as f64,
            9.0 + index as f64,
            11.0 + index as f64,
        ));
        last_ready = point.ready;
        if index < 6 {
            assert!(!point.ready);
        }
    }
    assert!(last_ready);
}

#[test]
fn signal_generation_happens_only_on_closed_candle_crossover() {
    let previous = relxen_domain::AsoPoint {
        open_time: 0,
        bulls: Some(45.0),
        bears: Some(55.0),
        length: 20,
        mode: AsoMode::Both,
        ready: true,
    };
    let current = relxen_domain::AsoPoint {
        open_time: 60_000,
        bulls: Some(55.0),
        bears: Some(45.0),
        length: 20,
        mode: AsoMode::Both,
        ready: true,
    };
    let signal = signal_from_points(Symbol::BtcUsdt, Timeframe::M1, &previous, &current).unwrap();
    assert_eq!(signal.side, SignalSide::Buy);
    assert!(signal.closed_only);
}

#[test]
fn paper_engine_open_reverse_and_close_behavior() {
    let settings = Settings::default();
    let mut engine = PaperEngine::new(&settings, 1);
    let buy_signal = SignalEvent {
        id: "s1".to_string(),
        symbol: Symbol::BtcUsdt,
        timeframe: Timeframe::M1,
        open_time: 0,
        side: SignalSide::Buy,
        bulls: 60.0,
        bears: 40.0,
        closed_only: true,
    };
    engine
        .apply_signal(&settings, &buy_signal, 50_000.0, 1)
        .unwrap();
    assert_eq!(engine.position.as_ref().unwrap().side, PositionSide::Long);

    let sell_signal = SignalEvent {
        side: SignalSide::Sell,
        id: "s2".to_string(),
        ..buy_signal.clone()
    };
    engine
        .apply_signal(&settings, &sell_signal, 50_500.0, 2)
        .unwrap();
    assert_eq!(engine.position.as_ref().unwrap().side, PositionSide::Short);

    engine.close_all(settings.fee_rate, 49_500.0, 3).unwrap();
    assert!(engine.position.is_none());
    assert!(engine
        .trades
        .iter()
        .any(|trade| trade.action == TradeAction::Reverse));
    assert!(engine
        .trades
        .iter()
        .any(|trade| trade.action == TradeAction::Close));
}

#[test]
fn fee_handling_is_applied_on_entry_and_exit() {
    let settings = Settings {
        fixed_notional: 100.0,
        fee_rate: 0.001,
        ..Settings::default()
    };
    let mut engine = PaperEngine::new(&settings, 1);
    let signal = SignalEvent {
        id: "s1".to_string(),
        symbol: Symbol::BtcUsdt,
        timeframe: Timeframe::M1,
        open_time: 0,
        side: SignalSide::Buy,
        bulls: 55.0,
        bears: 45.0,
        closed_only: true,
    };
    engine.apply_signal(&settings, &signal, 100.0, 1).unwrap();
    let wallet_after_open = engine.wallets.get(&QuoteAsset::Usdt).unwrap().clone();
    assert!(wallet_after_open.fees_paid > 0.0);
    engine.close_all(settings.fee_rate, 100.0, 2).unwrap();
    let wallet_after_close = engine.wallets.get(&QuoteAsset::Usdt).unwrap();
    assert!(wallet_after_close.fees_paid > wallet_after_open.fees_paid);
}

#[test]
fn insufficient_balance_prevents_opening_when_nothing_is_affordable() {
    assert_eq!(open_position_size(0.0, 100.0, 5.0, 0.0004), 0.0);
}

#[test]
fn wallet_separation_by_quote_asset_is_preserved() {
    let mut initial = BTreeMap::new();
    initial.insert(QuoteAsset::Usdt, 1000.0);
    initial.insert(QuoteAsset::Usdc, 500.0);
    let wallets = reset_wallets(&initial, 1);
    assert_eq!(wallets.get(&QuoteAsset::Usdt).unwrap().balance, 1000.0);
    assert_eq!(wallets.get(&QuoteAsset::Usdc).unwrap().balance, 500.0);
}

#[test]
fn performance_uses_mark_to_market() {
    let settings = Settings::default();
    let mut engine = PaperEngine::new(&settings, 1);
    let signal = SignalEvent {
        id: "s1".to_string(),
        symbol: Symbol::BtcUsdt,
        timeframe: Timeframe::M1,
        open_time: 0,
        side: SignalSide::Buy,
        bulls: 60.0,
        bears: 40.0,
        closed_only: true,
    };
    engine.apply_signal(&settings, &signal, 100.0, 1).unwrap();
    mark_to_market(&mut engine.wallets, &mut engine.position, 110.0, 2);
    let perf = compute_performance(&engine.wallets, &engine.position, &engine.trades);
    assert!(perf.unrealized_pnl > 0.0);
}

#[test]
fn derive_signal_history_returns_only_crossovers() {
    let points = vec![
        relxen_domain::AsoPoint {
            open_time: 0,
            bulls: Some(40.0),
            bears: Some(60.0),
            length: 20,
            mode: AsoMode::Both,
            ready: true,
        },
        relxen_domain::AsoPoint {
            open_time: 1,
            bulls: Some(60.0),
            bears: Some(40.0),
            length: 20,
            mode: AsoMode::Both,
            ready: true,
        },
        relxen_domain::AsoPoint {
            open_time: 2,
            bulls: Some(70.0),
            bears: Some(30.0),
            length: 20,
            mode: AsoMode::Both,
            ready: true,
        },
    ];
    let history = derive_signal_history(Symbol::BtcUsdt, Timeframe::M1, &points);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].side, SignalSide::Buy);
}

#[test]
fn mainnet_auto_default_position_policy_is_crossover_only() {
    assert_eq!(
        relxen_domain::MainnetAutoConfig::default().position_policy,
        AsoPositionPolicy::CrossoverOnly
    );
}

#[test]
fn always_in_market_chooses_long_when_bulls_exceed_bears() {
    let decision = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::AlwaysInMarket,
        bulls: Some(60.0),
        bears: Some(40.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(decision.desired_side, MainnetAutoDesiredSide::Long);
    assert_eq!(decision.action, MainnetAutoPolicyAction::EnterLong);
}

#[test]
fn always_in_market_chooses_short_when_bears_exceed_bulls() {
    let decision = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::AlwaysInMarket,
        bulls: Some(35.0),
        bears: Some(65.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(decision.desired_side, MainnetAutoDesiredSide::Short);
    assert_eq!(decision.action, MainnetAutoPolicyAction::EnterShort);
}

#[test]
fn always_in_market_does_nothing_on_equal_or_invalid_aso_state() {
    let equal = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::AlwaysInMarket,
        bulls: Some(50.0),
        bears: Some(50.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(equal.desired_side, MainnetAutoDesiredSide::None);
    assert_eq!(equal.action, MainnetAutoPolicyAction::NoTrade);

    let invalid = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::AlwaysInMarket,
        bulls: None,
        bears: Some(50.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(invalid.action, MainnetAutoPolicyAction::NoTrade);
    assert_eq!(invalid.blocker.as_deref(), Some("aso_state_invalid"));
}

#[test]
fn flat_allowed_blocks_weak_delta() {
    let decision = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::FlatAllowed,
        bulls: Some(52.0),
        bears: Some(48.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(decision.desired_side, MainnetAutoDesiredSide::None);
    assert_eq!(decision.action, MainnetAutoPolicyAction::NoTrade);
    assert_eq!(decision.reason, "flat_allowed_delta_below_threshold");
}

#[test]
fn flat_allowed_allows_long_when_delta_and_zone_pass() {
    let decision = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::FlatAllowed,
        bulls: Some(62.0),
        bears: Some(38.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(decision.desired_side, MainnetAutoDesiredSide::Long);
    assert_eq!(decision.action, MainnetAutoPolicyAction::EnterLong);
}

#[test]
fn flat_allowed_allows_short_when_delta_and_zone_pass() {
    let decision = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::FlatAllowed,
        bulls: Some(35.0),
        bears: Some(65.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(decision.desired_side, MainnetAutoDesiredSide::Short);
    assert_eq!(decision.action, MainnetAutoPolicyAction::EnterShort);
}

#[test]
fn crossover_only_preserves_wait_for_signal_behavior() {
    let decision = evaluate_aso_position_policy(AsoPolicyInput {
        policy: AsoPositionPolicy::CrossoverOnly,
        bulls: Some(65.0),
        bears: Some(35.0),
        delta_threshold: 5.0,
        zone_threshold: 55.0,
        current_side: MainnetAutoDesiredSide::None,
    });
    assert_eq!(decision.desired_side, MainnetAutoDesiredSide::None);
    assert_eq!(decision.action, MainnetAutoPolicyAction::NoTrade);
}

#[test]
fn margin_policy_allows_only_explicit_margin_type() {
    assert!(MainnetAutoAllowedMarginType::Isolated.allows(LiveMarginType::Isolated));
    assert!(!MainnetAutoAllowedMarginType::Isolated.allows(LiveMarginType::Cross));
    assert!(MainnetAutoAllowedMarginType::Cross.allows(LiveMarginType::Cross));
    assert!(MainnetAutoAllowedMarginType::Any.allows(LiveMarginType::Cross));
    assert!(MainnetAutoAllowedMarginType::Any.allows(LiveMarginType::Isolated));
    assert!(!MainnetAutoAllowedMarginType::Any.allows(LiveMarginType::Unknown));
}
