mod support;

use relxen_app::{AppMetadata, AppService, LiveDependencies, Repository, ServiceOptions};
use relxen_domain::{
    AsoMode, CreateLiveCredentialRequest, LiveBlockingReason, LiveCancelRequest,
    LiveCredentialValidationStatus, LiveEnvironment, LiveExecutionRequest, LiveOrderSide,
    LiveOrderType, LiveRuntimeState, LiveShadowBalance, LiveShadowOrder, LiveShadowStreamState,
    LiveUserDataEvent, Settings, Symbol, Timeframe,
};

use support::{
    arc, candle_with_bull_at_open_time, latest_closed_open_time, FakeLiveExchange, MockRepository,
    SequenceMarket, StaticMetrics, TestSecretStore,
};

async fn live_shadow_service(
    exchange: std::sync::Arc<FakeLiveExchange>,
) -> std::sync::Arc<AppService> {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: AsoMode::Intrabar,
            auto_restart_on_apply: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let open_time = latest_closed_open_time(Timeframe::M1);
    let history = vec![
        candle_with_bull_at_open_time(
            Symbol::BtcUsdt,
            Timeframe::M1,
            open_time - 2 * Timeframe::M1.duration_ms(),
            40.0,
            true,
        ),
        candle_with_bull_at_open_time(
            Symbol::BtcUsdt,
            Timeframe::M1,
            open_time - Timeframe::M1.duration_ms(),
            40.0,
            true,
        ),
        candle_with_bull_at_open_time(Symbol::BtcUsdt, Timeframe::M1, open_time, 60.0, true),
    ];
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        arc(SequenceMarket::new(Vec::new(), vec![history])),
        LiveDependencies::new(arc(TestSecretStore::default()), exchange),
        arc(StaticMetrics),
        arc(relxen_app::NoopPublisher),
        ServiceOptions {
            history_limit: 3,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    service
}

async fn create_valid_credential(service: &AppService) {
    let credential = service
        .create_live_credential(CreateLiveCredentialRequest {
            alias: "shadow".to_string(),
            environment: LiveEnvironment::Testnet,
            api_key: "abcd1234efgh5678".to_string(),
            api_secret: "secret".to_string(),
        })
        .await
        .unwrap();
    let validation = service
        .validate_live_credential(credential.id)
        .await
        .unwrap();
    assert_eq!(validation.status, LiveCredentialValidationStatus::Valid);
    service.refresh_live_readiness().await.unwrap();
}

#[tokio::test]
async fn shadow_start_bootstraps_rest_attaches_stream_and_applies_account_update() {
    let exchange = arc(FakeLiveExchange::default());
    exchange
        .user_events
        .lock()
        .await
        .push_back(Ok(LiveUserDataEvent::AccountUpdate(
            relxen_domain::LiveAccountShadow {
                environment: LiveEnvironment::Testnet,
                balances: vec![LiveShadowBalance {
                    asset: "USDT".to_string(),
                    wallet_balance: "1234.5".to_string(),
                    cross_wallet_balance: Some("1200".to_string()),
                    balance_change: Some("0".to_string()),
                    updated_at: relxen_app::now_ms(),
                }],
                positions: Vec::new(),
                open_orders: Vec::new(),
                can_trade: true,
                multi_assets_margin: Some(false),
                position_mode: Some("one_way".to_string()),
                last_event_time: Some(relxen_app::now_ms()),
                last_rest_sync_at: None,
                updated_at: relxen_app::now_ms(),
                ambiguous: false,
                divergence_reasons: Vec::new(),
            },
        )));
    let service = live_shadow_service(exchange).await;
    create_valid_credential(&service).await;

    let started = service.start_live_shadow().await.unwrap();
    assert!(matches!(
        started.reconciliation.stream.state,
        LiveShadowStreamState::Running
    ));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        let updated = service
            .live_status()
            .await
            .unwrap()
            .reconciliation
            .shadow
            .as_ref()
            .and_then(|shadow| shadow.balances.first())
            .map(|balance| balance.wallet_balance.as_str() == "1234.5")
            .unwrap_or(false);
        if updated {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let stopped = service.stop_live_shadow().await.unwrap();
    assert_eq!(
        stopped.reconciliation.stream.state,
        LiveShadowStreamState::Stopped
    );
}

#[tokio::test]
async fn preflight_builds_precision_aware_intent_and_does_not_create_position() {
    let service = live_shadow_service(arc(FakeLiveExchange::default())).await;
    create_valid_credential(&service).await;
    service.start_live_shadow().await.unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();
    let intent = preview.intent.expect("intent should be built");
    assert!(intent.can_preflight);
    assert!(intent.can_execute_now);
    assert_eq!(intent.exchange_payload.get("type").unwrap(), "MARKET");
    assert!(intent.quantity.parse::<f64>().unwrap() > 0.0);

    let result = service.run_live_preflight().await.unwrap();
    assert!(result.accepted);
    assert_eq!(result.message, "PREFLIGHT PASSED. No order was placed.");
    assert!(service
        .get_bootstrap()
        .await
        .unwrap()
        .current_position
        .is_none());
}

#[tokio::test]
async fn testnet_execute_and_cancel_are_gated_and_persist_order_state() {
    let exchange = arc(FakeLiveExchange::default());
    let service = live_shadow_service(exchange).await;
    create_valid_credential(&service).await;
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();
    let intent_id = preview.intent.as_ref().unwrap().id.clone();
    let executed = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: Some(intent_id),
            confirm_testnet: true,
        })
        .await
        .unwrap();

    assert!(executed.accepted);
    let order = executed.order.unwrap();
    assert_eq!(order.status.as_str(), "working");
    assert_eq!(service.list_live_orders(10).await.unwrap().len(), 1);

    let canceled = service
        .cancel_live_order(LiveCancelRequest {
            order_ref: order.id,
            confirm_testnet: true,
        })
        .await
        .unwrap();
    assert!(canceled.accepted);
    assert_eq!(
        canceled.order.unwrap().status.as_str(),
        relxen_domain::LiveOrderStatus::Canceled.as_str()
    );
}

#[tokio::test]
async fn mainnet_execution_is_explicitly_blocked() {
    let service = live_shadow_service(arc(FakeLiveExchange::default())).await;
    let credential = service
        .create_live_credential(CreateLiveCredentialRequest {
            alias: "mainnet".to_string(),
            environment: LiveEnvironment::Mainnet,
            api_key: "abcd1234efgh5678".to_string(),
            api_secret: "secret".to_string(),
        })
        .await
        .unwrap();
    service
        .validate_live_credential(credential.id)
        .await
        .unwrap();
    service.refresh_live_readiness().await.unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();

    let result = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: None,
            confirm_testnet: true,
        })
        .await
        .unwrap();
    assert!(!result.accepted);
    assert_eq!(
        result.blocking_reason,
        Some(LiveBlockingReason::MainnetExecutionBlocked)
    );
}

#[tokio::test]
async fn user_data_order_trade_update_records_authoritative_fill() {
    let exchange = arc(FakeLiveExchange::default());
    exchange
        .user_events
        .lock()
        .await
        .push_back(Ok(LiveUserDataEvent::OrderTradeUpdate(Box::new(
            LiveShadowOrder {
                order_id: "99".to_string(),
                client_order_id: Some("rx_exec_fill_test".to_string()),
                symbol: Symbol::BtcUsdt,
                side: LiveOrderSide::Buy,
                order_type: LiveOrderType::Market,
                time_in_force: None,
                original_qty: "0.001".to_string(),
                executed_qty: "0.001".to_string(),
                price: None,
                avg_price: Some("100000".to_string()),
                status: "FILLED".to_string(),
                execution_type: Some("TRADE".to_string()),
                reduce_only: false,
                position_side: Some("BOTH".to_string()),
                last_filled_qty: Some("0.001".to_string()),
                last_filled_price: Some("100000".to_string()),
                commission: Some("0.04".to_string()),
                commission_asset: Some("USDT".to_string()),
                trade_id: Some("123".to_string()),
                last_update_time: relxen_app::now_ms(),
            },
        ))));
    let service = live_shadow_service(exchange).await;
    create_valid_credential(&service).await;
    service.start_live_shadow().await.unwrap();

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        if !service.list_live_fills(10).await.unwrap().is_empty() {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let fills = service.list_live_fills(10).await.unwrap();
    assert_eq!(fills[0].trade_id.as_deref(), Some("123"));
    assert_eq!(fills[0].quantity, "0.001");
}

#[tokio::test]
async fn mainnet_preflight_is_blocked_before_exchange_call() {
    let service = live_shadow_service(arc(FakeLiveExchange::default())).await;
    let credential = service
        .create_live_credential(CreateLiveCredentialRequest {
            alias: "mainnet".to_string(),
            environment: LiveEnvironment::Mainnet,
            api_key: "abcd1234efgh5678".to_string(),
            api_secret: "secret".to_string(),
        })
        .await
        .unwrap();
    service
        .validate_live_credential(credential.id)
        .await
        .unwrap();
    service.refresh_live_readiness().await.unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();

    let result = service.run_live_preflight().await.unwrap();
    assert!(!result.accepted);
    assert_eq!(
        result.local_blocking_reason,
        Some(LiveBlockingReason::PreflightNotSupportedOnMainnet)
    );
}

#[tokio::test]
async fn unsupported_shadow_mode_blocks_intent_preview() {
    let mut exchange = FakeLiveExchange::default();
    let mut account = support::fake_account_snapshot(LiveEnvironment::Testnet);
    account.multi_assets_margin = Some(true);
    exchange.account = Some(account);
    let service = live_shadow_service(arc(exchange)).await;
    create_valid_credential(&service).await;
    service.start_live_shadow().await.unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();
    assert!(preview
        .blocking_reasons
        .contains(&LiveBlockingReason::UnsupportedAccountMode));
    assert_eq!(
        service.live_status().await.unwrap().state,
        LiveRuntimeState::PreflightBlocked
    );
}
