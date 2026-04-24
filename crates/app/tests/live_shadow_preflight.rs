mod support;

use std::sync::atomic::Ordering;

use relxen_app::{AppMetadata, AppService, LiveDependencies, Repository, ServiceOptions};
use relxen_domain::{
    AsoMode, Candle, CreateLiveCredentialRequest, LiveAutoExecutorRequest,
    LiveAutoExecutorStateKind, LiveBlockingReason, LiveCancelRequest,
    LiveCredentialValidationStatus, LiveEnvironment, LiveExecutionRequest, LiveFillRecord,
    LiveKillSwitchRequest, LiveOrderSide, LiveOrderStatus, LiveOrderType, LiveRiskLimits,
    LiveRiskProfile, LiveRuntimeState, LiveShadowBalance, LiveShadowOrder, LiveShadowPosition,
    LiveShadowStreamState, LiveUserDataEvent, MainnetAutoConfig, MainnetAutoDecisionOutcome,
    MainnetAutoLiveStartRequest, MainnetAutoRiskBudget, MainnetAutoRunMode, MainnetAutoState,
    Settings, Symbol, Timeframe, MAINNET_AUTO_LIVE_CONFIRMATION_TEXT,
};
use rust_decimal::Decimal;

use support::{
    arc, candle_with_bull_at_open_time, fake_reference_price, fake_symbol_rules,
    latest_closed_open_time, stream_event, FakeLiveExchange, MockRepository, SequenceMarket,
    StaticMetrics, TestSecretStore,
};

async fn live_shadow_service(
    exchange: std::sync::Arc<FakeLiveExchange>,
) -> std::sync::Arc<AppService> {
    live_shadow_service_with(exchange, Vec::new(), ServiceOptions::default()).await
}

async fn live_shadow_service_with(
    exchange: std::sync::Arc<FakeLiveExchange>,
    subscriptions: Vec<Vec<Result<relxen_app::MarketStreamEvent, relxen_app::AppError>>>,
    options: ServiceOptions,
) -> std::sync::Arc<AppService> {
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
    live_shadow_service_with_history(exchange, subscriptions, options, history).await
}

async fn live_shadow_service_with_history(
    exchange: std::sync::Arc<FakeLiveExchange>,
    subscriptions: Vec<Vec<Result<relxen_app::MarketStreamEvent, relxen_app::AppError>>>,
    options: ServiceOptions,
    history: Vec<Candle>,
) -> std::sync::Arc<AppService> {
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: AsoMode::Intrabar,
            auto_restart_on_apply: false,
            paper_enabled: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        arc(SequenceMarket::new(subscriptions, vec![history])),
        LiveDependencies::new(arc(TestSecretStore::default()), exchange),
        arc(StaticMetrics),
        arc(relxen_app::NoopPublisher),
        ServiceOptions {
            history_limit: 3,
            auto_start: false,
            ..options
        },
    );
    service.initialize().await.unwrap();
    service
}

async fn mainnet_live_auto_service_with(
    exchange: std::sync::Arc<FakeLiveExchange>,
    subscriptions: Vec<Vec<Result<relxen_app::MarketStreamEvent, relxen_app::AppError>>>,
    options: ServiceOptions,
) -> std::sync::Arc<AppService> {
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
    let repository = arc(MockRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: AsoMode::Intrabar,
            fixed_notional: 80.0,
            leverage: 5.0,
            auto_restart_on_apply: false,
            paper_enabled: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        arc(SequenceMarket::new(subscriptions, vec![history])),
        LiveDependencies::new(arc(TestSecretStore::default()), exchange),
        arc(StaticMetrics),
        arc(relxen_app::NoopPublisher),
        ServiceOptions {
            history_limit: 3,
            auto_start: false,
            ..options
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

async fn create_valid_credential_for(service: &AppService, environment: LiveEnvironment) {
    let credential = service
        .create_live_credential(CreateLiveCredentialRequest {
            alias: format!("{environment}-shadow"),
            environment,
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

fn permissive_risk_profile() -> LiveRiskProfile {
    LiveRiskProfile {
        configured: true,
        profile_name: Some("test-canary-risk".to_string()),
        limits: LiveRiskLimits {
            max_notional_per_order: "1000".to_string(),
            max_open_notional_active_symbol: "1000".to_string(),
            max_leverage: "10".to_string(),
            max_orders_per_session: 10,
            max_fills_per_session: 20,
            max_consecutive_rejections: 3,
            max_daily_realized_loss: "250".to_string(),
        },
        updated_at: relxen_app::now_ms(),
    }
}

fn live_auto_options() -> ServiceOptions {
    ServiceOptions {
        mainnet_auto_config: MainnetAutoConfig {
            enable_live_execution: true,
            mode: MainnetAutoRunMode::Live,
            max_runtime_minutes: 15,
            max_orders: 20,
            max_fills: 20,
            max_notional: "80".to_string(),
            max_daily_loss: "5".to_string(),
            require_flat_start: true,
            require_flat_stop: true,
            require_manual_canary_evidence: false,
            evidence_required: true,
            lesson_report_required: true,
        },
        ..ServiceOptions::default()
    }
}

fn live_auto_risk_budget() -> MainnetAutoRiskBudget {
    MainnetAutoRiskBudget {
        budget_id: "mainnet-auto-live-test-v1".to_string(),
        max_orders_per_session: 20,
        max_fills_per_session: 20,
        allowed_order_types: vec![LiveOrderType::Market],
        max_runtime_minutes: 15,
        ..MainnetAutoRiskBudget::default()
    }
}

fn live_auto_start_request() -> MainnetAutoLiveStartRequest {
    MainnetAutoLiveStartRequest {
        symbol: Symbol::BtcUsdt,
        duration_minutes: 15,
        order_type: LiveOrderType::Market,
        confirmation_text: MAINNET_AUTO_LIVE_CONFIRMATION_TEXT.to_string(),
    }
}

fn mainnet_confirmation(intent: &relxen_domain::LiveOrderIntent) -> String {
    format!(
        "SUBMIT MAINNET {} LIMIT {} {} @ {}",
        intent.side.as_binance(),
        intent.symbol,
        intent.quantity,
        intent.price.as_deref().unwrap_or_default()
    )
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
async fn live_status_account_snapshot_uses_fresh_shadow_positions() {
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
                    wallet_balance: "4999.9".to_string(),
                    cross_wallet_balance: Some("4994.5".to_string()),
                    balance_change: Some("0".to_string()),
                    updated_at: relxen_app::now_ms(),
                }],
                positions: vec![LiveShadowPosition {
                    symbol: Symbol::BtcUsdt,
                    position_side: "BOTH".to_string(),
                    position_amt: "0.0014".to_string(),
                    entry_price: "77928.8".to_string(),
                    unrealized_pnl: "0.2".to_string(),
                    margin_type: None,
                    isolated_wallet: None,
                    updated_at: relxen_app::now_ms(),
                }],
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
    service.start_live_shadow().await.unwrap();

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        let has_position = service
            .live_status()
            .await
            .unwrap()
            .account_snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot.positions.iter().any(|position| {
                    position.symbol == Symbol::BtcUsdt
                        && (position.position_amt - 0.0014).abs() < f64::EPSILON
                        && (position.entry_price - 77928.8).abs() < f64::EPSILON
                })
            })
            .unwrap_or(false);
        if has_position {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
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
    let service = live_shadow_service(exchange.clone()).await;
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
            confirm_mainnet_canary: false,
            confirmation_text: None,
        })
        .await
        .unwrap();

    assert!(executed.accepted);
    let order = executed.order.unwrap();
    assert_eq!(order.status.as_str(), "accepted");
    assert_eq!(order.response_type.as_deref(), Some("ACK"));
    assert_eq!(service.list_live_orders(10).await.unwrap().len(), 1);

    let canceled = service
        .cancel_live_order(LiveCancelRequest {
            order_ref: order.id.clone(),
            confirm_testnet: true,
            confirm_mainnet_canary: false,
            confirmation_text: None,
        })
        .await
        .unwrap();
    assert!(canceled.accepted);
    let canceled_order = canceled.order.unwrap();
    assert_eq!(canceled_order.id, order.id);
    assert_eq!(
        canceled_order.status.as_str(),
        relxen_domain::LiveOrderStatus::Canceled.as_str()
    );
    assert_eq!(exchange.submitted_orders.lock().await.len(), 1);
}

#[tokio::test]
async fn refresh_live_shadow_repairs_recent_order_and_fill_state() {
    let exchange = arc(FakeLiveExchange::default());
    let service = live_shadow_service(exchange.clone()).await;
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
            confirm_mainnet_canary: false,
            confirmation_text: None,
        })
        .await
        .unwrap();
    let order = executed.order.unwrap();

    {
        let mut submitted = exchange.submitted_orders.lock().await;
        let submitted_order = submitted
            .iter_mut()
            .find(|candidate| candidate.id == order.id)
            .expect("submitted order should exist in fake exchange");
        submitted_order.status = LiveOrderStatus::Filled;
        submitted_order.executed_qty = submitted_order.quantity.clone();
        submitted_order.avg_price = Some("100000".to_string());
        submitted_order.updated_at = relxen_app::now_ms();
    }
    exchange.user_trades.lock().await.push(LiveFillRecord {
        id: "repair-fill".to_string(),
        order_id: None,
        client_order_id: None,
        exchange_order_id: order.exchange_order_id.clone(),
        symbol: order.symbol,
        side: order.side,
        quantity: order.quantity.clone(),
        price: "100000".to_string(),
        commission: Some("0.01".to_string()),
        commission_asset: Some("USDT".to_string()),
        realized_pnl: Some("0".to_string()),
        trade_id: Some("repair-trade".to_string()),
        event_time: relxen_app::now_ms(),
        created_at: relxen_app::now_ms(),
    });

    service.refresh_live_shadow().await.unwrap();
    let repaired_order = service
        .list_live_orders(10)
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == order.id)
        .expect("repaired order should still be present");
    assert_eq!(repaired_order.status, LiveOrderStatus::Filled);
    let repaired_fill = service
        .list_live_fills(10)
        .await
        .unwrap()
        .into_iter()
        .find(|fill| fill.id == "repair-fill")
        .expect("recent-window repair should append fill");
    assert_eq!(repaired_fill.order_id.as_deref(), Some(order.id.as_str()));
}

#[tokio::test]
async fn kill_switch_blocks_new_execution_but_leaves_cancel_available() {
    let exchange = arc(FakeLiveExchange::default());
    let service = live_shadow_service(exchange).await;
    create_valid_credential(&service).await;
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    let preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();

    service
        .engage_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test_kill".to_string()),
        })
        .await
        .unwrap();
    let result = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: preview.intent.map(|intent| intent.id),
            confirm_testnet: true,
            confirm_mainnet_canary: false,
            confirmation_text: None,
        })
        .await
        .unwrap();

    assert!(!result.accepted);
    assert_eq!(
        result.blocking_reason,
        Some(LiveBlockingReason::KillSwitchEngaged)
    );
    assert!(service.live_status().await.unwrap().kill_switch.engaged);
}

#[tokio::test]
async fn mainnet_canary_requires_server_gate_risk_profile_and_exact_confirmation() {
    let service = live_shadow_service_with(
        arc(FakeLiveExchange::default()),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(19, 0)))
        .await
        .unwrap();

    let blocked = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: None,
            confirm_testnet: false,
            confirm_mainnet_canary: true,
            confirmation_text: Some("wrong".to_string()),
        })
        .await
        .unwrap();
    assert!(!blocked.accepted);
    assert_eq!(
        blocked.blocking_reason,
        Some(LiveBlockingReason::MainnetCanaryRiskProfileMissing)
    );

    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    let status = service.live_status().await.unwrap();
    assert!(status.mainnet_canary.enabled_by_server);
    assert!(status.mainnet_canary.risk_profile_configured);
    let confirmation = status
        .mainnet_canary
        .required_confirmation
        .expect("mainnet canary confirmation should be generated");

    let result = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: None,
            confirm_testnet: false,
            confirm_mainnet_canary: true,
            confirmation_text: Some(confirmation),
        })
        .await
        .unwrap();

    assert!(result.accepted);
    let order = result
        .order
        .expect("mainnet canary order should be accepted by fake exchange");
    assert_eq!(order.environment, LiveEnvironment::Mainnet);
    assert_eq!(order.response_type.as_deref(), Some("ACK"));
}

#[tokio::test]
async fn mainnet_canary_blocks_shadow_environment_mismatch() {
    let service = live_shadow_service_with(
        arc(FakeLiveExchange::default()),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
    )
    .await;
    create_valid_credential(&service).await;
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();

    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(19, 0)))
        .await
        .unwrap();
    assert!(preview
        .blocking_reasons
        .contains(&LiveBlockingReason::ShadowStateAmbiguous));
    let status = service.live_status().await.unwrap();
    assert!(status
        .execution
        .blocking_reasons
        .contains(&LiveBlockingReason::ShadowStateAmbiguous));
}

#[tokio::test]
async fn mainnet_canary_blocks_market_and_rounded_marketable_limits() {
    let service = live_shadow_service_with(
        arc(FakeLiveExchange::default()),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();

    let market_preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();
    assert!(market_preview
        .blocking_reasons
        .contains(&LiveBlockingReason::MainnetCanaryLimitRequired));

    let marketable_preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(2004, 2)))
        .await
        .unwrap();
    assert!(marketable_preview
        .blocking_reasons
        .contains(&LiveBlockingReason::MainnetCanaryLimitMarketable));
}

#[tokio::test]
async fn mainnet_canary_blocks_missing_reference_price() {
    let open_time = latest_closed_open_time(Timeframe::M1);
    let zero_reference_history = vec![
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
        candle_with_bull_at_open_time(Symbol::BtcUsdt, Timeframe::M1, open_time, 0.0, true),
    ];
    let service = live_shadow_service_with_history(
        arc(FakeLiveExchange {
            fail_reference_price: true,
            ..FakeLiveExchange::default()
        }),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
        zero_reference_history,
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(19, 0)))
        .await
        .unwrap();
    assert!(preview
        .blocking_reasons
        .contains(&LiveBlockingReason::ReferencePriceUnavailable));
}

#[tokio::test]
async fn mainnet_canary_blocks_stale_rest_reference_price() {
    let exchange = arc(FakeLiveExchange::default());
    let mut stale_reference =
        fake_reference_price(LiveEnvironment::Mainnet, Symbol::BtcUsdt, "2000");
    stale_reference.observed_at = Some(relxen_app::now_ms() - 60_000);
    *exchange.reference_price.lock().unwrap() = Some(stale_reference);
    let service = live_shadow_service_with(
        exchange,
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            live_intent_ttl_ms: 1_000,
            ..ServiceOptions::default()
        },
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .engage_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();
    service
        .release_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(1999, 0)))
        .await
        .unwrap();
    assert!(preview
        .blocking_reasons
        .contains(&LiveBlockingReason::ReferencePriceStale));
    assert_eq!(
        preview
            .reference_price
            .as_ref()
            .and_then(|reference| reference.blocking_reason),
        Some(LiveBlockingReason::ReferencePriceStale)
    );
}

#[tokio::test]
async fn mainnet_canary_refreshes_after_kill_switch_release_from_rest_mark_price() {
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
    let service = live_shadow_service_with_history(
        arc(FakeLiveExchange::default()),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
        history,
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .engage_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();
    service
        .release_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(1999, 0)))
        .await
        .unwrap();
    assert!(preview.intent.is_some());
    let reference = preview.reference_price.as_ref().unwrap();
    assert_eq!(reference.source.as_deref(), Some("rest_mark_price"));
    assert_eq!(reference.price.as_deref(), Some("2000"));
    assert_eq!(
        preview
            .marketability_check
            .as_ref()
            .unwrap()
            .marketable_after_rounding,
        Some(false)
    );
}

#[tokio::test]
async fn mainnet_submit_forces_reference_refresh_after_kill_switch_release() {
    let exchange = arc(FakeLiveExchange::default());
    let open_time = latest_closed_open_time(Timeframe::M1);
    let mut history = vec![
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
    for candle in &mut history {
        candle.close = 2000.0;
        candle.high = 2001.0;
        candle.low = 1999.0;
    }
    let service = live_shadow_service_with_history(
        exchange.clone(),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
        history,
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(1999, 0)))
        .await
        .unwrap();
    let intent = preview.intent.clone().unwrap();
    service
        .engage_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();
    service
        .release_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();

    let result = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: Some(intent.id.clone()),
            confirm_testnet: false,
            confirm_mainnet_canary: true,
            confirmation_text: Some(mainnet_confirmation(&intent)),
        })
        .await
        .unwrap();
    assert!(result.accepted);
    assert_eq!(exchange.submitted_orders.lock().await.len(), 1);
}

#[tokio::test]
async fn mainnet_submit_blocks_when_fresh_refresh_makes_limit_marketable() {
    let exchange = arc(FakeLiveExchange::default());
    *exchange.reference_price.lock().unwrap() = Some(fake_reference_price(
        LiveEnvironment::Mainnet,
        Symbol::BtcUsdt,
        "1900",
    ));
    let open_time = latest_closed_open_time(Timeframe::M1);
    let mut history = vec![
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
    for candle in &mut history {
        candle.close = 2000.0;
        candle.high = 2001.0;
        candle.low = 1999.0;
    }
    let service = live_shadow_service_with_history(
        exchange.clone(),
        Vec::new(),
        ServiceOptions {
            enable_mainnet_canary_execution: true,
            ..ServiceOptions::default()
        },
        history,
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_live_risk_profile(permissive_risk_profile())
        .await
        .unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();

    let preview = service
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(1999, 0)))
        .await
        .unwrap();
    let intent = preview.intent.clone().unwrap();
    service
        .engage_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();
    service
        .release_live_kill_switch(LiveKillSwitchRequest {
            reason: Some("test".to_string()),
        })
        .await
        .unwrap();

    let result = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: Some(intent.id.clone()),
            confirm_testnet: false,
            confirm_mainnet_canary: true,
            confirmation_text: Some(mainnet_confirmation(&intent)),
        })
        .await
        .unwrap();
    assert!(!result.accepted);
    assert_eq!(
        result.blocking_reason,
        Some(LiveBlockingReason::MainnetCanaryLimitMarketable)
    );
    assert!(exchange.submitted_orders.lock().await.is_empty());
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
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(19, 0)))
        .await
        .unwrap();

    let result = service
        .execute_live_current_preview(LiveExecutionRequest {
            intent_id: None,
            confirm_testnet: true,
            confirm_mainnet_canary: false,
            confirmation_text: None,
        })
        .await
        .unwrap();
    assert!(!result.accepted);
    assert_eq!(
        result.blocking_reason,
        Some(LiveBlockingReason::MainnetCanaryDisabled)
    );
}

#[tokio::test]
async fn forced_user_data_reconnect_repairs_shadow_before_resubscribe() {
    let exchange = arc(FakeLiveExchange::default());
    let service = live_shadow_service_with(
        exchange.clone(),
        Vec::new(),
        ServiceOptions {
            live_user_stream_forced_reconnect_ms: 10,
            ..ServiceOptions::default()
        },
    )
    .await;
    create_valid_credential(&service).await;
    service.start_live_shadow().await.unwrap();

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        if exchange.listen_key_creates.load(Ordering::SeqCst) >= 2
            && exchange.stream_subscriptions.load(Ordering::SeqCst) >= 2
            && service
                .live_status()
                .await
                .unwrap()
                .reconciliation
                .stream
                .detail
                .as_deref()
                == Some("forced 24h user-data reconnect completed")
        {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(exchange.listen_key_closes.load(Ordering::SeqCst) >= 1);
    assert!(
        service
            .live_status()
            .await
            .unwrap()
            .execution
            .repair_recent_window_only
    );
    service.stop_live_shadow().await.unwrap();
}

#[tokio::test]
async fn testnet_auto_executor_submits_closed_candle_signal_once() {
    let exchange = arc(FakeLiveExchange::default());
    let open_time = latest_closed_open_time(Timeframe::M1) + Timeframe::M1.duration_ms();
    let event = stream_event(
        candle_with_bull_at_open_time(Symbol::BtcUsdt, Timeframe::M1, open_time, 20.0, true),
        true,
    );
    let service = live_shadow_service_with(
        exchange.clone(),
        vec![vec![Ok(event.clone()), Ok(event)]],
        ServiceOptions::default(),
    )
    .await;
    create_valid_credential(&service).await;
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    let auto_status = service
        .start_live_auto_executor(LiveAutoExecutorRequest {
            confirm_testnet_auto: true,
        })
        .await
        .unwrap()
        .auto_executor;
    assert_eq!(auto_status.state, LiveAutoExecutorStateKind::Running);

    service.start_runtime().await.unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        if !exchange.submitted_orders.lock().await.is_empty() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            let signals = service.list_signals(10).await.unwrap();
            let auto = service.live_status().await.unwrap().auto_executor;
            panic!("timed out waiting for auto order; signals={signals:?}; auto={auto:?}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    service.stop_runtime().await.unwrap();

    let submitted = exchange.submitted_orders.lock().await.clone();
    assert_eq!(submitted.len(), 1);
    let status = service.live_status().await.unwrap().auto_executor;
    assert_eq!(status.last_signal_open_time, Some(open_time));
    assert_eq!(status.last_order_id, Some(submitted[0].id.clone()));
    assert_eq!(status.blocking_reasons, Vec::<LiveBlockingReason>::new());
}

#[tokio::test]
async fn testnet_auto_drill_helper_replays_latest_signal_once_and_suppresses_duplicate() {
    let exchange = arc(FakeLiveExchange::default());
    let open_time = latest_closed_open_time(Timeframe::M1) + Timeframe::M1.duration_ms();
    let event = stream_event(
        candle_with_bull_at_open_time(Symbol::BtcUsdt, Timeframe::M1, open_time, 20.0, true),
        true,
    );
    let service = live_shadow_service_with(
        exchange.clone(),
        vec![vec![Ok(event)]],
        ServiceOptions {
            enable_testnet_drill_helpers: true,
            ..ServiceOptions::default()
        },
    )
    .await;
    create_valid_credential(&service).await;
    service.start_runtime().await.unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        if !service.list_signals(1).await.unwrap().is_empty() {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    service.stop_runtime().await.unwrap();
    service.arm_live().await.unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .start_live_auto_executor(LiveAutoExecutorRequest {
            confirm_testnet_auto: true,
        })
        .await
        .unwrap();

    let first = service.drill_replay_latest_auto_signal().await.unwrap();
    assert_eq!(
        first.auto_executor.state,
        LiveAutoExecutorStateKind::Running
    );

    let submitted = exchange.submitted_orders.lock().await.clone();
    assert_eq!(submitted.len(), 1);

    let second = service.drill_replay_latest_auto_signal().await.unwrap();
    assert_eq!(
        second.auto_executor.state,
        LiveAutoExecutorStateKind::Running
    );
    assert_eq!(
        second.auto_executor.blocking_reasons,
        vec![LiveBlockingReason::DuplicateSignalSuppressed]
    );

    let resubmitted = exchange.submitted_orders.lock().await.clone();
    assert_eq!(resubmitted.len(), 1);
}

#[tokio::test]
async fn mainnet_auto_is_disabled_by_default_and_live_start_blocks() {
    let exchange = arc(FakeLiveExchange::default());
    let service =
        live_shadow_service_with(exchange.clone(), Vec::new(), ServiceOptions::default()).await;

    let status = service.mainnet_auto_status().await.unwrap();
    assert_eq!(status.state, MainnetAutoState::Disabled);
    assert!(!status.config.enable_live_execution);
    assert!(status
        .current_blockers
        .contains(&"mainnet_auto_config_disabled".to_string()));

    let blocked = service.start_mainnet_auto_live(None).await.unwrap();
    assert_eq!(blocked.state, MainnetAutoState::ConfigBlocked);
    assert_eq!(blocked.live_orders_submitted, 0);
    assert!(blocked
        .current_blockers
        .contains(&"mainnet_auto_config_disabled".to_string()));
    assert!(exchange.submitted_orders.lock().await.is_empty());
}

#[tokio::test]
async fn mainnet_auto_dry_run_records_decision_without_order_submission() {
    let exchange = arc(FakeLiveExchange::default());
    let open_time = latest_closed_open_time(Timeframe::M1) + Timeframe::M1.duration_ms();
    let event = stream_event(
        candle_with_bull_at_open_time(Symbol::BtcUsdt, Timeframe::M1, open_time, 20.0, true),
        true,
    );
    let service = live_shadow_service_with(
        exchange.clone(),
        vec![vec![Ok(event)]],
        ServiceOptions::default(),
    )
    .await;
    service.start_runtime().await.unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        if !service.list_signals(1).await.unwrap().is_empty() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out waiting for a closed-candle signal before mainnet auto dry-run");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    service.stop_runtime().await.unwrap();

    let status = service.start_mainnet_auto_dry_run().await.unwrap();
    assert_eq!(status.state, MainnetAutoState::DryRunRunning);
    assert_eq!(status.live_orders_submitted, 0);
    assert_eq!(status.dry_run_orders_submitted, 0);
    assert!(matches!(
        status.last_decision_outcome,
        Some(MainnetAutoDecisionOutcome::SkippedConfigBlocked)
            | Some(MainnetAutoDecisionOutcome::SkippedStaleReferencePrice)
            | Some(MainnetAutoDecisionOutcome::DryRunWouldSubmit)
    ));
    let decisions = service.list_mainnet_auto_decisions(10).await.unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].mode, relxen_domain::MainnetAutoRunMode::DryRun);
    assert!(exchange.submitted_orders.lock().await.is_empty());
    let lessons = service.latest_mainnet_auto_lessons().await.unwrap();
    assert!(lessons.is_some());
    assert!(!lessons.unwrap().live_order_submitted);

    let duplicate_status = service.start_mainnet_auto_dry_run().await.unwrap();
    assert_eq!(
        duplicate_status.last_decision_outcome,
        Some(MainnetAutoDecisionOutcome::SkippedDuplicate)
    );
    let decisions = service.list_mainnet_auto_decisions(10).await.unwrap();
    assert_eq!(decisions.len(), 2);
    assert!(decisions[0]
        .blocking_reasons
        .iter()
        .chain(decisions[1].blocking_reasons.iter())
        .any(|reason| reason == "duplicate_signal_detected"));
    assert!(exchange.submitted_orders.lock().await.is_empty());
}

#[tokio::test]
async fn mainnet_auto_live_start_requires_exact_session_confirmation() {
    let exchange = arc(FakeLiveExchange::default());
    let service =
        mainnet_live_auto_service_with(exchange.clone(), Vec::new(), live_auto_options()).await;

    let blocked = service
        .start_mainnet_auto_live(Some(MainnetAutoLiveStartRequest {
            confirmation_text: "wrong confirmation".to_string(),
            ..live_auto_start_request()
        }))
        .await
        .unwrap();

    assert_ne!(blocked.state, MainnetAutoState::LiveRunning);
    assert!(blocked
        .current_blockers
        .contains(&"mainnet_auto_session_confirmation_missing".to_string()));
    assert!(exchange.submitted_orders.lock().await.is_empty());
}

#[tokio::test]
async fn mainnet_auto_live_start_runs_session_without_immediate_order() {
    let mut fake = FakeLiveExchange::default();
    let mut rules = fake_symbol_rules(LiveEnvironment::Mainnet, Symbol::BtcUsdt);
    rules.filters.min_notional = Some(50.0);
    fake.rules = Some(rules);
    fake.reference_price = std::sync::Mutex::new(Some(fake_reference_price(
        LiveEnvironment::Mainnet,
        Symbol::BtcUsdt,
        "2000",
    )));
    let exchange = arc(fake);
    let service =
        mainnet_live_auto_service_with(exchange.clone(), Vec::new(), live_auto_options()).await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_mainnet_auto_risk_budget(live_auto_risk_budget())
        .await
        .unwrap();
    service.start_live_shadow().await.unwrap();

    let status = service
        .start_mainnet_auto_live(Some(live_auto_start_request()))
        .await
        .unwrap();

    assert_eq!(status.state, MainnetAutoState::LiveRunning);
    assert_eq!(status.mode, MainnetAutoRunMode::Live);
    assert_eq!(status.live_orders_submitted, 0);
    assert!(status.expires_at.is_some());
    assert!(exchange.submitted_orders.lock().await.is_empty());
}

#[tokio::test]
async fn mainnet_auto_live_closed_candle_signal_submits_with_mocked_adapter_only() {
    let mut fake = FakeLiveExchange::default();
    let mut rules = fake_symbol_rules(LiveEnvironment::Mainnet, Symbol::BtcUsdt);
    rules.filters.min_notional = Some(50.0);
    fake.rules = Some(rules);
    fake.reference_price = std::sync::Mutex::new(Some(fake_reference_price(
        LiveEnvironment::Mainnet,
        Symbol::BtcUsdt,
        "2000",
    )));
    let exchange = arc(fake);
    let open_time = latest_closed_open_time(Timeframe::M1) + Timeframe::M1.duration_ms();
    let event = stream_event(
        candle_with_bull_at_open_time(Symbol::BtcUsdt, Timeframe::M1, open_time, 20.0, true),
        true,
    );
    let service = mainnet_live_auto_service_with(
        exchange.clone(),
        vec![vec![Ok(event)]],
        live_auto_options(),
    )
    .await;
    create_valid_credential_for(&service, LiveEnvironment::Mainnet).await;
    service
        .configure_mainnet_auto_risk_budget(live_auto_risk_budget())
        .await
        .unwrap();
    service.start_live_shadow().await.unwrap();
    service
        .start_mainnet_auto_live(Some(live_auto_start_request()))
        .await
        .unwrap();

    service.start_runtime().await.unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if !exchange.submitted_orders.lock().await.is_empty() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out waiting for mocked MAINNET auto live order submission");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    service.stop_runtime().await.unwrap();

    let submitted = exchange.submitted_orders.lock().await.clone();
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].environment, LiveEnvironment::Mainnet);
    assert_eq!(submitted[0].symbol, Symbol::BtcUsdt);
    assert_eq!(submitted[0].order_type, LiveOrderType::Market);
    let status = service.mainnet_auto_status().await.unwrap();
    assert_eq!(status.live_orders_submitted, 1);
    assert_eq!(
        status.last_decision_outcome,
        Some(MainnetAutoDecisionOutcome::LiveSubmitRequested)
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
                self_trade_prevention_mode: None,
                price_match: None,
                expire_reason: None,
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
        .build_live_intent_preview(LiveOrderType::Limit, Some(Decimal::new(19, 0)))
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
