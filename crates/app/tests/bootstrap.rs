mod support;

use std::sync::atomic::Ordering;

use relxen_app::{
    now_ms, AppError, AppMetadata, AppService, KlineRangeRequest, NoopPublisher, Repository,
    ServiceOptions,
};
use relxen_domain::{AsoMode, Settings, Symbol, Timeframe};

use support::{arc, candle_with_bull_at_open_time, MockRepository, SequenceMarket, StaticMetrics};

fn recent_closed_window(
    symbol: Symbol,
    timeframe: Timeframe,
    count: usize,
    latest_closed_open_time: i64,
    bull: f64,
) -> Vec<relxen_domain::Candle> {
    let start_open_time = latest_closed_open_time - (count as i64 - 1) * timeframe.duration_ms();

    (0..count)
        .map(|index| {
            candle_with_bull_at_open_time(
                symbol,
                timeframe,
                start_open_time + index as i64 * timeframe.duration_ms(),
                bull,
                true,
            )
        })
        .collect()
}

fn intrabar_settings(symbol: Symbol, timeframe: Timeframe) -> Settings {
    Settings {
        active_symbol: symbol,
        timeframe,
        aso_length: 2,
        aso_mode: AsoMode::Intrabar,
        auto_restart_on_apply: false,
        ..Settings::default()
    }
}

#[tokio::test]
async fn bootstrap_backfills_with_explicit_ranged_queries_when_db_history_is_insufficient() {
    let repository = arc(MockRepository::default());
    let settings = intrabar_settings(Symbol::BtcUsdt, Timeframe::M1);
    repository.save_settings(&settings).await.unwrap();
    let anchor = Timeframe::M1.align_open_time(now_ms() - Timeframe::M1.duration_ms());

    let remote_window = recent_closed_window(Symbol::BtcUsdt, Timeframe::M1, 4, anchor, 0.0);
    repository.seed_candles(&remote_window[..1]).await;

    let market = arc(SequenceMarket::new(Vec::new(), vec![remote_window.clone()]));
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market.clone(),
        arc(StaticMetrics),
        arc(NoopPublisher),
        ServiceOptions {
            history_limit: 4,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );

    let snapshot = service.initialize().await.unwrap();
    let requests = market.range_requests().await;

    assert_eq!(market.range_called.load(Ordering::SeqCst), 1);
    assert_eq!(
        requests,
        vec![KlineRangeRequest {
            symbol: Symbol::BtcUsdt,
            timeframe: Timeframe::M1,
            start_open_time: remote_window.first().unwrap().open_time,
            end_open_time: remote_window.last().unwrap().open_time,
        }]
    );
    assert_eq!(snapshot.candles, remote_window);
    assert!(snapshot.aso_points.iter().any(|point| point.ready));
}

#[tokio::test]
async fn timeframe_change_triggers_deterministic_series_rebuild() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    let m1_anchor = Timeframe::M1.align_open_time(now_ms() - Timeframe::M1.duration_ms());

    let initial_m1 = recent_closed_window(Symbol::BtcUsdt, Timeframe::M1, 2, m1_anchor, 0.0);
    repository.seed_candles(&initial_m1).await;

    let m5_anchor = Timeframe::M5.align_open_time(now_ms() - Timeframe::M5.duration_ms());
    let rebuilt_m5 = recent_closed_window(Symbol::BtcUsdt, Timeframe::M5, 2, m5_anchor, 100.0);
    let market = arc(SequenceMarket::new(Vec::new(), vec![rebuilt_m5.clone()]));
    let service = AppService::new(
        AppMetadata::default(),
        repository.clone(),
        market.clone(),
        arc(StaticMetrics),
        arc(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );

    let initial = service.initialize().await.unwrap();
    assert!(initial
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M1));

    let snapshot = service
        .update_settings(intrabar_settings(Symbol::BtcUsdt, Timeframe::M5))
        .await
        .unwrap();
    let requests = market.range_requests().await;

    assert_eq!(market.range_called.load(Ordering::SeqCst), 1);
    assert_eq!(
        requests,
        vec![KlineRangeRequest {
            symbol: Symbol::BtcUsdt,
            timeframe: Timeframe::M5,
            start_open_time: rebuilt_m5.first().unwrap().open_time,
            end_open_time: rebuilt_m5.last().unwrap().open_time,
        }]
    );
    assert_eq!(snapshot.runtime_status.timeframe, Timeframe::M5);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M5));
    assert_eq!(
        repository.klines_for(Symbol::BtcUsdt, Timeframe::M5).await,
        rebuilt_m5
    );
}

#[tokio::test]
async fn symbol_change_triggers_deterministic_series_rebuild() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    let usdt_anchor = Timeframe::M1.align_open_time(now_ms() - Timeframe::M1.duration_ms());

    let initial_usdt = recent_closed_window(Symbol::BtcUsdt, Timeframe::M1, 2, usdt_anchor, 0.0);
    repository.seed_candles(&initial_usdt).await;

    let usdc_anchor = Timeframe::M1.align_open_time(now_ms() - Timeframe::M1.duration_ms());
    let rebuilt_usdc = recent_closed_window(Symbol::BtcUsdc, Timeframe::M1, 2, usdc_anchor, 100.0);
    let market = arc(SequenceMarket::new(Vec::new(), vec![rebuilt_usdc.clone()]));
    let service = AppService::new(
        AppMetadata::default(),
        repository.clone(),
        market.clone(),
        arc(StaticMetrics),
        arc(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );

    service.initialize().await.unwrap();

    let snapshot = service
        .update_settings(intrabar_settings(Symbol::BtcUsdc, Timeframe::M1))
        .await
        .unwrap();
    let requests = market.range_requests().await;

    assert_eq!(market.range_called.load(Ordering::SeqCst), 1);
    assert_eq!(
        requests,
        vec![KlineRangeRequest {
            symbol: Symbol::BtcUsdc,
            timeframe: Timeframe::M1,
            start_open_time: rebuilt_usdc.first().unwrap().open_time,
            end_open_time: rebuilt_usdc.last().unwrap().open_time,
        }]
    );
    assert_eq!(snapshot.active_symbol, Symbol::BtcUsdc);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.symbol == Symbol::BtcUsdc));
    assert_eq!(
        repository.klines_for(Symbol::BtcUsdc, Timeframe::M1).await,
        rebuilt_usdc
    );
}

#[tokio::test]
async fn ambiguous_history_rebuild_returns_typed_history_error() {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    let m1_anchor = Timeframe::M1.align_open_time(now_ms() - Timeframe::M1.duration_ms());

    let initial_m1 = recent_closed_window(Symbol::BtcUsdt, Timeframe::M1, 2, m1_anchor, 0.0);
    repository.seed_candles(&initial_m1).await;

    let m5_anchor = Timeframe::M5.align_open_time(now_ms() - Timeframe::M5.duration_ms());
    let incomplete_m5 = recent_closed_window(Symbol::BtcUsdt, Timeframe::M5, 2, m5_anchor, 0.0)
        .into_iter()
        .take(1)
        .collect::<Vec<_>>();
    let market = arc(SequenceMarket::new(Vec::new(), vec![incomplete_m5]));
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market,
        arc(StaticMetrics),
        arc(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );

    service.initialize().await.unwrap();

    let error = service
        .update_settings(intrabar_settings(Symbol::BtcUsdt, Timeframe::M5))
        .await
        .unwrap_err();

    match error {
        AppError::History(detail) => {
            assert!(detail.contains("expected 2 closed candles but found 1"));
        }
        other => panic!("expected AppError::History, got {other:?}"),
    }
}
