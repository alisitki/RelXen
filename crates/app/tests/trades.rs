mod support;

use std::time::Duration;

use relxen_app::{AppMetadata, AppService, OutboundEvent, Repository, ServiceOptions};
use relxen_domain::{AsoMode, Settings, Symbol, Timeframe, TradeAction, TradeSource};

use support::{
    arc, candle_with_bull_at_open_time, recent_open_time, stream_event, wait_until,
    CapturingPublisher, MockRepository, SequenceMarket, StaticMetrics,
};

fn intrabar_settings() -> Settings {
    Settings {
        active_symbol: Symbol::BtcUsdt,
        timeframe: Timeframe::M1,
        aso_length: 2,
        aso_mode: AsoMode::Intrabar,
        auto_restart_on_apply: false,
        ..Settings::default()
    }
}

fn anchored_candle(
    latest_closed_open_time: i64,
    offset_from_latest_closed: i64,
    bull: f64,
    closed: bool,
) -> relxen_domain::Candle {
    candle_with_bull_at_open_time(
        Symbol::BtcUsdt,
        Timeframe::M1,
        latest_closed_open_time + offset_from_latest_closed * Timeframe::M1.duration_ms(),
        bull,
        closed,
    )
}

#[tokio::test]
async fn trade_events_are_emitted_for_signal_manual_and_reset_transitions() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings())
        .await
        .unwrap();
    let anchor = recent_open_time(Timeframe::M1, 0);
    repository
        .seed_candles(&[
            anchored_candle(anchor, -1, 0.0, true),
            anchored_candle(anchor, 0, 40.0, true),
        ])
        .await;

    let market = arc(SequenceMarket::new(
        vec![vec![
            Ok(stream_event(anchored_candle(anchor, 1, 100.0, true), true)),
            Ok(stream_event(anchored_candle(anchor, 2, 20.0, true), true)),
            Ok(stream_event(anchored_candle(anchor, 3, 20.0, true), true)),
        ]],
        Vec::new(),
    ));
    let publisher = arc(CapturingPublisher::default());
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market,
        arc(StaticMetrics),
        publisher.clone(),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );

    service.initialize().await.unwrap();
    service.start_runtime().await.unwrap();

    wait_until("signal trade events", Duration::from_secs(5), || {
        publisher
            .events()
            .iter()
            .filter(|event| matches!(event, OutboundEvent::TradeAppended(_)))
            .count()
            >= 3
    })
    .await;

    service.close_all().await.unwrap();
    service.reset_paper().await.unwrap();
    service.stop_runtime().await.unwrap();

    let events = publisher.events();
    let trades: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            OutboundEvent::TradeAppended(trade) => Some(trade.clone()),
            _ => None,
        })
        .collect();

    assert!(events
        .iter()
        .any(|event| matches!(event, OutboundEvent::TradeHistoryReset)));
    assert_eq!(trades[0].action, TradeAction::Open);
    assert_eq!(trades[0].source, TradeSource::Signal);
    assert_eq!(trades[1].action, TradeAction::Reverse);
    assert_eq!(trades[1].source, TradeSource::Signal);
    assert_eq!(trades[2].action, TradeAction::Open);
    assert_eq!(trades[2].source, TradeSource::Signal);
    assert!(trades
        .iter()
        .any(|trade| trade.action == TradeAction::Close && trade.source == TradeSource::Manual));
}
