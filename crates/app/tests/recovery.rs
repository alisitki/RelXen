mod support;

use std::sync::atomic::Ordering;
use std::time::Duration;

use relxen_app::{
    AppMetadata, AppService, KlineRangeRequest, OutboundEvent, Repository, ServiceOptions,
};
use relxen_domain::{AsoMode, ConnectionStatus, PositionSide, Settings, SignalSide, Timeframe};

use support::{
    arc, assert_contains_status, candle_with_bull_at_open_time, recent_open_time, stream_error,
    stream_event, wait_until, CapturingPublisher, MockRepository, SequenceMarket, StaticMetrics,
};

fn intrabar_settings(timeframe: Timeframe) -> Settings {
    Settings {
        aso_length: 2,
        aso_mode: AsoMode::Intrabar,
        timeframe,
        ..Settings::default()
    }
}

fn anchored_candle(
    timeframe: Timeframe,
    latest_closed_open_time: i64,
    offset_from_latest_closed: i64,
    bull: f64,
    closed: bool,
) -> relxen_domain::Candle {
    candle_with_bull_at_open_time(
        relxen_domain::Symbol::BtcUsdt,
        timeframe,
        latest_closed_open_time + offset_from_latest_closed * timeframe.duration_ms(),
        bull,
        closed,
    )
}

#[tokio::test]
async fn websocket_interruptions_can_recover_on_one_minute_with_explicit_range_queries() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Timeframe::M1))
        .await
        .unwrap();
    let anchor = recent_open_time(Timeframe::M1, 0);
    repository
        .seed_candles(&[
            anchored_candle(Timeframe::M1, anchor, -1, 0.0, true),
            anchored_candle(Timeframe::M1, anchor, 0, 0.0, true),
        ])
        .await;

    let recovered = anchored_candle(Timeframe::M1, anchor, 1, 0.0, true);
    let live_partial = anchored_candle(Timeframe::M1, anchor, 2, 0.0, false);
    let live_closed = anchored_candle(Timeframe::M1, anchor, 2, 0.0, true);
    let market = arc(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![
                Ok(stream_event(live_partial.clone(), false)),
                Ok(stream_event(live_closed.clone(), true)),
            ],
        ],
        vec![vec![recovered.clone()]],
    ));
    let publisher = arc(CapturingPublisher::default());

    let service = AppService::new(
        AppMetadata::default(),
        repository.clone(),
        market.clone(),
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

    wait_until(
        "successful one-minute reconnect recovery",
        Duration::from_secs(5),
        || {
            publisher
                .connection_statuses()
                .contains(&ConnectionStatus::Connected)
                && market.range_called.load(Ordering::SeqCst) >= 1
        },
    )
    .await;

    let snapshot = service.get_bootstrap().await.unwrap();
    let range_requests = market.range_requests().await;
    let recovered_open_times: Vec<i64> = repository
        .all_klines()
        .await
        .into_iter()
        .filter(|candle| candle.closed)
        .map(|candle| candle.open_time)
        .collect();
    let statuses = publisher.connection_statuses();

    service.stop_runtime().await.unwrap();

    assert_eq!(market.range_called.load(Ordering::SeqCst), 1);
    assert_eq!(
        range_requests,
        vec![KlineRangeRequest {
            symbol: relxen_domain::Symbol::BtcUsdt,
            timeframe: Timeframe::M1,
            start_open_time: recovered.open_time,
            end_open_time: recovered.open_time,
        }]
    );
    assert_eq!(
        snapshot.connection_state.status,
        ConnectionStatus::Connected
    );
    assert!(!publisher.has_resync_required());
    assert_contains_status(&statuses, ConnectionStatus::Reconnecting);
    assert_contains_status(&statuses, ConnectionStatus::Stale);
    assert_contains_status(&statuses, ConnectionStatus::Resynced);
    assert_contains_status(&statuses, ConnectionStatus::Connected);
    assert!(recovered_open_times.contains(&recovered.open_time));
    assert!(recovered_open_times.contains(&live_closed.open_time));
}

#[tokio::test]
async fn websocket_interruptions_can_recover_on_five_minute_timeframes() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Timeframe::M5))
        .await
        .unwrap();
    let anchor = recent_open_time(Timeframe::M5, 0);
    repository
        .seed_candles(&[
            anchored_candle(Timeframe::M5, anchor, -1, 0.0, true),
            anchored_candle(Timeframe::M5, anchor, 0, 0.0, true),
        ])
        .await;

    let recovered = vec![
        anchored_candle(Timeframe::M5, anchor, 1, 0.0, true),
        anchored_candle(Timeframe::M5, anchor, 2, 0.0, true),
    ];
    let live_partial = anchored_candle(Timeframe::M5, anchor, 3, 0.0, false);
    let live_closed = anchored_candle(Timeframe::M5, anchor, 3, 0.0, true);
    let market = arc(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![
                Ok(stream_event(live_partial, false)),
                Ok(stream_event(live_closed.clone(), true)),
            ],
        ],
        vec![recovered.clone()],
    ));
    let publisher = arc(CapturingPublisher::default());

    let service = AppService::new(
        AppMetadata::default(),
        repository.clone(),
        market.clone(),
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

    wait_until(
        "successful five-minute reconnect recovery",
        Duration::from_secs(5),
        || {
            market.range_called.load(Ordering::SeqCst) >= 1
                && publisher
                    .connection_statuses()
                    .contains(&ConnectionStatus::Connected)
        },
    )
    .await;

    let range_requests = market.range_requests().await;
    let recovered_open_times: Vec<i64> = repository
        .all_klines()
        .await
        .into_iter()
        .filter(|candle| candle.closed)
        .map(|candle| candle.open_time)
        .collect();

    service.stop_runtime().await.unwrap();

    assert_eq!(
        range_requests,
        vec![KlineRangeRequest {
            symbol: relxen_domain::Symbol::BtcUsdt,
            timeframe: Timeframe::M5,
            start_open_time: recovered[0].open_time,
            end_open_time: recovered[1].open_time,
        }]
    );
    assert!(recovered_open_times.contains(&recovered[0].open_time));
    assert!(recovered_open_times.contains(&recovered[1].open_time));
    assert!(recovered_open_times.contains(&live_closed.open_time));
}

#[tokio::test]
async fn reconnect_recovery_emits_exactly_one_missed_closed_candle_signal() {
    let repository = arc(MockRepository::default());
    let settings = intrabar_settings(Timeframe::M1);
    repository.save_settings(&settings).await.unwrap();
    let anchor = recent_open_time(Timeframe::M1, 0);
    repository
        .seed_candles(&[
            anchored_candle(Timeframe::M1, anchor, -1, 0.0, true),
            anchored_candle(Timeframe::M1, anchor, 0, 40.0, true),
        ])
        .await;

    let recovered = anchored_candle(Timeframe::M1, anchor, 1, 100.0, true);
    let live_partial = anchored_candle(Timeframe::M1, anchor, 2, 100.0, false);
    let live_closed = anchored_candle(Timeframe::M1, anchor, 2, 100.0, true);
    let market = arc(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![
                Ok(stream_event(live_partial, false)),
                Ok(stream_event(live_closed, true)),
            ],
        ],
        vec![vec![recovered.clone()]],
    ));
    let publisher = arc(CapturingPublisher::default());

    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market.clone(),
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

    wait_until("single recovered signal", Duration::from_secs(5), || {
        publisher
            .events()
            .iter()
            .filter(|event| matches!(event, OutboundEvent::SignalEmitted(_)))
            .count()
            == 1
            && market.range_called.load(Ordering::SeqCst) >= 1
    })
    .await;

    let signals = service.list_signals(10).await.unwrap();
    let trades = service.list_trades(10).await.unwrap();
    let snapshot = service.get_bootstrap().await.unwrap();

    service.stop_runtime().await.unwrap();

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].open_time, recovered.open_time);
    assert_eq!(signals[0].side, SignalSide::Buy);
    assert_eq!(trades.len(), 1);
    assert_eq!(
        snapshot
            .current_position
            .as_ref()
            .map(|position| position.side),
        Some(PositionSide::Long)
    );
}

#[tokio::test]
async fn recovery_including_an_open_candle_does_not_emit_a_false_closed_signal() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Timeframe::M1))
        .await
        .unwrap();
    let anchor = recent_open_time(Timeframe::M1, 0);
    repository
        .seed_candles(&[
            anchored_candle(Timeframe::M1, anchor, -1, 0.0, true),
            anchored_candle(Timeframe::M1, anchor, 0, 0.0, true),
        ])
        .await;

    let recovered = anchored_candle(Timeframe::M1, anchor, 1, 0.0, true);
    let live_partial = anchored_candle(Timeframe::M1, anchor, 2, 100.0, false);
    let market = arc(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![Ok(stream_event(live_partial.clone(), false))],
        ],
        vec![vec![recovered.clone()]],
    ));
    let publisher = arc(CapturingPublisher::default());

    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market.clone(),
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

    wait_until(
        "partial live candle after recovery",
        Duration::from_secs(5),
        || {
            publisher.events().iter().any(|event| matches!(
            event,
            OutboundEvent::CandlePartial(candle) if candle.open_time == live_partial.open_time
        ))
        },
    )
    .await;

    let signals = service.list_signals(10).await.unwrap();
    let range_requests = market.range_requests().await;

    service.stop_runtime().await.unwrap();

    assert!(signals.is_empty());
    assert!(!publisher
        .events()
        .iter()
        .any(|event| matches!(event, OutboundEvent::SignalEmitted(_))));
    assert_eq!(
        range_requests,
        vec![KlineRangeRequest {
            symbol: relxen_domain::Symbol::BtcUsdt,
            timeframe: Timeframe::M1,
            start_open_time: recovered.open_time,
            end_open_time: recovered.open_time,
        }]
    );
}

#[tokio::test]
async fn irrecoverable_gap_emits_resync_required() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Timeframe::M1))
        .await
        .unwrap();
    let anchor = recent_open_time(Timeframe::M1, 0);
    repository
        .seed_candles(&[
            anchored_candle(Timeframe::M1, anchor, -1, 0.0, true),
            anchored_candle(Timeframe::M1, anchor, 0, 0.0, true),
        ])
        .await;

    let market = arc(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![Ok(stream_event(
                anchored_candle(Timeframe::M1, anchor, 4, 0.0, false),
                false,
            ))],
        ],
        vec![vec![
            anchored_candle(Timeframe::M1, anchor, 2, 0.0, true),
            anchored_candle(Timeframe::M1, anchor, 3, 0.0, true),
        ]],
    ));
    let publisher = arc(CapturingPublisher::default());

    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market.clone(),
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

    wait_until("resync required event", Duration::from_secs(5), || {
        publisher.has_resync_required()
    })
    .await;

    let range_requests = market.range_requests().await;
    let statuses = publisher.connection_statuses();
    let events = publisher.events();

    service.stop_runtime().await.unwrap();

    assert_eq!(market.range_called.load(Ordering::SeqCst), 1);
    assert_eq!(
        range_requests,
        vec![KlineRangeRequest {
            symbol: relxen_domain::Symbol::BtcUsdt,
            timeframe: Timeframe::M1,
            start_open_time: anchored_candle(Timeframe::M1, anchor, 1, 0.0, true).open_time,
            end_open_time: anchored_candle(Timeframe::M1, anchor, 3, 0.0, true).open_time,
        }]
    );
    assert_contains_status(&statuses, ConnectionStatus::Stale);
    assert!(events.iter().any(|event| matches!(
        event,
        OutboundEvent::ResyncRequired { reason }
            if reason.contains("returned 2 candles but 3 were required")
    )));
}
