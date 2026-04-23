use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use relxen_app::{
    AppError, AppMetadata, AppResult, AppService, KlineRangeRequest, LiveDependencies,
    LiveExchangePort, MarketDataPort, MarketStream, MarketStreamEvent, MetricsPort, NoopPublisher,
    Repository, ServiceOptions,
};
use relxen_domain::{
    AsoMode, Candle, CreateLiveCredentialRequest, LiveAccountSnapshot, LiveAssetBalance,
    LiveCredentialId, LiveCredentialSecret, LiveCredentialValidationResult,
    LiveCredentialValidationStatus, LiveEnvironment, LiveOrderType, LiveSymbolFilterSummary,
    LiveSymbolRules, PositionSide, Settings, Symbol, SystemMetrics, Timeframe,
};
use relxen_infra::{MemorySecretStore, SqliteRepository};

#[derive(Debug, Clone, Copy)]
enum RangeResponse {
    Complete { bull: f64 },
    SkipFirst { bull: f64 },
}

struct ReleaseGateMarket {
    responses: Mutex<VecDeque<RangeResponse>>,
    subscriptions: Mutex<VecDeque<Vec<Result<MarketStreamEvent, AppError>>>>,
}

impl ReleaseGateMarket {
    fn new(responses: Vec<RangeResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            subscriptions: Mutex::new(VecDeque::new()),
        }
    }

    async fn push_subscription(&self, events: Vec<Result<MarketStreamEvent, AppError>>) {
        self.subscriptions.lock().await.push_back(events);
    }
}

#[async_trait]
impl MarketDataPort for ReleaseGateMarket {
    async fn fetch_klines_range(&self, request: KlineRangeRequest) -> AppResult<Vec<Candle>> {
        let response = self
            .responses
            .lock()
            .await
            .pop_front()
            .unwrap_or(RangeResponse::Complete { bull: 40.0 });
        let candles = range_candles(request, response);
        Ok(candles)
    }

    async fn subscribe_klines(
        &self,
        _symbol: Symbol,
        _timeframe: Timeframe,
    ) -> AppResult<MarketStream> {
        let events = self
            .subscriptions
            .lock()
            .await
            .pop_front()
            .unwrap_or_default();
        Ok(Box::pin(stream::iter(events).chain(stream::pending())))
    }
}

struct StaticMetrics;

impl MetricsPort for StaticMetrics {
    fn snapshot(&self) -> SystemMetrics {
        SystemMetrics {
            cpu_usage_percent: 0.0,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
            task_count: 1,
            collected_at: 1,
        }
    }
}

struct SqliteFakeLiveExchange;

#[async_trait]
impl LiveExchangePort for SqliteFakeLiveExchange {
    async fn validate_credentials(
        &self,
        environment: LiveEnvironment,
        credential_id: &LiveCredentialId,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveCredentialValidationResult> {
        Ok(LiveCredentialValidationResult {
            credential_id: credential_id.clone(),
            environment,
            status: LiveCredentialValidationStatus::Valid,
            validated_at: relxen_app::now_ms(),
            message: None,
        })
    }

    async fn fetch_account_snapshot(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountSnapshot> {
        Ok(LiveAccountSnapshot {
            environment,
            can_trade: true,
            multi_assets_margin: Some(false),
            total_wallet_balance: 1000.0,
            total_margin_balance: 1000.0,
            available_balance: 900.0,
            assets: vec![LiveAssetBalance {
                asset: "USDT".to_string(),
                wallet_balance: 1000.0,
                available_balance: 900.0,
                unrealized_pnl: 0.0,
            }],
            positions: Vec::new(),
            fetched_at: relxen_app::now_ms(),
        })
    }

    async fn fetch_symbol_rules(
        &self,
        environment: LiveEnvironment,
        symbol: Symbol,
    ) -> AppResult<LiveSymbolRules> {
        Ok(LiveSymbolRules {
            environment,
            symbol,
            status: "TRADING".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: symbol.quote_asset(),
            price_precision: 2,
            quantity_precision: 3,
            filters: LiveSymbolFilterSummary {
                tick_size: Some(0.1),
                step_size: Some(0.001),
                min_qty: Some(0.001),
                min_notional: Some(100.0),
            },
            fetched_at: relxen_app::now_ms(),
        })
    }

    async fn create_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<String> {
        Ok("sqlite-listen-key".to_string())
    }

    async fn keepalive_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _listen_key: &str,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn close_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _listen_key: &str,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn subscribe_user_data(
        &self,
        _environment: LiveEnvironment,
        _listen_key: &str,
    ) -> AppResult<relxen_app::LiveUserDataStream> {
        Ok(Box::pin(stream::pending::<
            Result<relxen_domain::LiveUserDataEvent, AppError>,
        >()))
    }

    async fn preflight_order_test(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _payload: &BTreeMap<String, String>,
    ) -> AppResult<()> {
        Ok(())
    }
}

struct TempDatabase {
    path: PathBuf,
    url: String,
}

impl TempDatabase {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "relxen-{label}-{}-{nonce}.sqlite3",
            std::process::id()
        ));
        let url = format!("sqlite://{}", path.display());
        Self { path, url }
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(self.path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(self.path.with_extension("sqlite3-shm"));
    }
}

async fn service(
    database_url: &str,
    market: Arc<ReleaseGateMarket>,
    history_limit: usize,
) -> Arc<AppService> {
    let repository = Arc::new(SqliteRepository::connect(database_url).await.unwrap());
    AppService::new(
        AppMetadata::default(),
        repository,
        market,
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit,
            auto_start: false,
            ..ServiceOptions::default()
        },
    )
}

async fn service_with_live(
    database_url: &str,
    market: Arc<ReleaseGateMarket>,
    history_limit: usize,
    secret_store: Arc<MemorySecretStore>,
) -> Arc<AppService> {
    let repository = Arc::new(SqliteRepository::connect(database_url).await.unwrap());
    AppService::new_with_live(
        AppMetadata::default(),
        repository,
        market,
        LiveDependencies::new(secret_store, Arc::new(SqliteFakeLiveExchange)),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit,
            auto_start: false,
            ..ServiceOptions::default()
        },
    )
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

fn range_candles(request: KlineRangeRequest, response: RangeResponse) -> Vec<Candle> {
    let mut open_time = request.timeframe.align_open_time(request.start_open_time);
    let end_open_time = request.timeframe.align_open_time(request.end_open_time);
    let mut candles = Vec::new();
    let bull = match response {
        RangeResponse::Complete { bull } | RangeResponse::SkipFirst { bull } => bull,
    };

    while open_time <= end_open_time {
        candles.push(candle_with_bull(
            request.symbol,
            request.timeframe,
            open_time,
            bull,
            true,
        ));
        open_time = request.timeframe.next_open_time(open_time);
    }

    if matches!(response, RangeResponse::SkipFirst { .. }) && !candles.is_empty() {
        candles.remove(0);
    }

    candles
}

fn candle_with_bull(
    symbol: Symbol,
    timeframe: Timeframe,
    open_time: i64,
    bull: f64,
    closed: bool,
) -> Candle {
    let bull = bull.clamp(0.0, 100.0);
    let (open, close) = if bull <= 50.0 {
        (100.0, bull * 2.0)
    } else {
        (0.0, (bull - 50.0) * 2.0)
    };
    Candle {
        symbol,
        timeframe,
        open_time,
        close_time: timeframe.close_time_for_open(open_time),
        open,
        high: 100.0,
        low: 0.0,
        close,
        volume: 1.0,
        closed,
    }
}

fn stream_event(candle: Candle, closed: bool) -> Result<MarketStreamEvent, AppError> {
    Ok(MarketStreamEvent { candle, closed })
}

async fn wait_for_trade_count(service: &Arc<AppService>, minimum: usize) {
    timeout(Duration::from_secs(5), async move {
        loop {
            if service
                .list_trades(10)
                .await
                .map(|trades| trades.len() >= minimum)
                .unwrap_or(false)
            {
                return;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out while waiting for {minimum} paper trades"));
}

#[tokio::test]
async fn bootstrap_restart_keeps_settings_history_wallets_and_logs_coherent() {
    let db = TempDatabase::new("bootstrap-restart");
    let market = Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
        bull: 40.0,
    }]));
    let first = service(&db.url, market, 39).await;

    let first_snapshot = first.initialize().await.unwrap();
    let reset_snapshot = first.reset_paper().await.unwrap();

    assert_eq!(first_snapshot.settings.active_symbol, Symbol::BtcUsdt);
    assert_eq!(first_snapshot.candles.len(), 39);
    assert_eq!(reset_snapshot.wallets.len(), 2);

    let restarted = service(&db.url, Arc::new(ReleaseGateMarket::new(Vec::new())), 39).await;
    let restarted_snapshot = restarted.initialize().await.unwrap();

    assert_eq!(restarted_snapshot.settings.active_symbol, Symbol::BtcUsdt);
    assert_eq!(restarted_snapshot.candles.len(), 39);
    assert_eq!(restarted_snapshot.wallets.len(), 2);
    assert!(restarted_snapshot
        .recent_logs
        .iter()
        .any(|log| log.message.contains("paper account reset")));
}

#[tokio::test]
async fn timeframe_rebuild_survives_restart_without_mixed_runtime_series() {
    let db = TempDatabase::new("timeframe-rebuild");
    let repository = SqliteRepository::connect(&db.url).await.unwrap();
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    drop(repository);

    let market = Arc::new(ReleaseGateMarket::new(vec![
        RangeResponse::Complete { bull: 20.0 },
        RangeResponse::Complete { bull: 80.0 },
    ]));
    let first = service(&db.url, market, 2).await;
    let initial = first.initialize().await.unwrap();
    assert!(initial
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M1));

    let rebuilt = first
        .update_settings(intrabar_settings(Symbol::BtcUsdt, Timeframe::M5))
        .await
        .unwrap();
    assert_eq!(rebuilt.runtime_status.timeframe, Timeframe::M5);
    assert!(rebuilt
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M5));

    let restarted = service(
        &db.url,
        Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
            bull: 20.0,
        }])),
        2,
    )
    .await;
    let snapshot = restarted.initialize().await.unwrap();
    assert_eq!(snapshot.runtime_status.timeframe, Timeframe::M5);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M5));
}

#[tokio::test]
async fn symbol_rebuild_survives_restart_with_active_symbol_history_coherent() {
    let db = TempDatabase::new("symbol-rebuild");
    let repository = SqliteRepository::connect(&db.url).await.unwrap();
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    drop(repository);

    let market = Arc::new(ReleaseGateMarket::new(vec![
        RangeResponse::Complete { bull: 20.0 },
        RangeResponse::Complete { bull: 80.0 },
    ]));
    let first = service(&db.url, market, 2).await;
    first.initialize().await.unwrap();

    let rebuilt = first
        .update_settings(intrabar_settings(Symbol::BtcUsdc, Timeframe::M1))
        .await
        .unwrap();
    assert_eq!(rebuilt.active_symbol, Symbol::BtcUsdc);
    assert!(rebuilt
        .candles
        .iter()
        .all(|candle| candle.symbol == Symbol::BtcUsdc));

    let restarted = service(
        &db.url,
        Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
            bull: 20.0,
        }])),
        2,
    )
    .await;
    let snapshot = restarted.initialize().await.unwrap();
    assert_eq!(snapshot.active_symbol, Symbol::BtcUsdc);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.symbol == Symbol::BtcUsdc));
}

#[tokio::test]
async fn history_failure_keeps_sqlite_visible_state_on_last_valid_snapshot() {
    let db = TempDatabase::new("history-failure");
    let repository = SqliteRepository::connect(&db.url).await.unwrap();
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    drop(repository);

    let market = Arc::new(ReleaseGateMarket::new(vec![
        RangeResponse::Complete { bull: 20.0 },
        RangeResponse::SkipFirst { bull: 80.0 },
    ]));
    let first = service(&db.url, market, 2).await;
    first.initialize().await.unwrap();

    let error = first
        .update_settings(intrabar_settings(Symbol::BtcUsdt, Timeframe::M5))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::History(_)));

    let snapshot = first.get_bootstrap().await.unwrap();
    assert_eq!(snapshot.runtime_status.timeframe, Timeframe::M1);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M1));

    let repository = SqliteRepository::connect(&db.url).await.unwrap();
    let failed_timeframe_rows = repository
        .load_recent_klines(Symbol::BtcUsdt, Timeframe::M5, 10)
        .await
        .unwrap();
    assert!(failed_timeframe_rows.is_empty());
    assert_eq!(
        repository.load_settings().await.unwrap().timeframe,
        Timeframe::M1
    );
}

#[tokio::test]
async fn paper_trades_and_open_position_survive_restart() {
    let db = TempDatabase::new("trade-restart");
    let repository = SqliteRepository::connect(&db.url).await.unwrap();
    repository
        .save_settings(&intrabar_settings(Symbol::BtcUsdt, Timeframe::M1))
        .await
        .unwrap();
    drop(repository);

    let market = Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
        bull: 20.0,
    }]));
    let first = service(&db.url, market.clone(), 2).await;
    let snapshot = first.initialize().await.unwrap();
    let anchor = snapshot.candles.last().unwrap().open_time;
    market
        .push_subscription(vec![
            stream_event(
                candle_with_bull(
                    Symbol::BtcUsdt,
                    Timeframe::M1,
                    anchor - 4 * Timeframe::M1.duration_ms(),
                    100.0,
                    true,
                ),
                true,
            ),
            stream_event(
                candle_with_bull(
                    Symbol::BtcUsdt,
                    Timeframe::M1,
                    anchor - 3 * Timeframe::M1.duration_ms(),
                    20.0,
                    true,
                ),
                true,
            ),
            stream_event(
                candle_with_bull(
                    Symbol::BtcUsdt,
                    Timeframe::M1,
                    anchor - 2 * Timeframe::M1.duration_ms(),
                    20.0,
                    true,
                ),
                true,
            ),
        ])
        .await;

    first.start_runtime().await.unwrap();
    wait_for_trade_count(&first, 3).await;
    first.stop_runtime().await.unwrap();

    let restarted = service(&db.url, Arc::new(ReleaseGateMarket::new(Vec::new())), 2).await;
    let restarted_snapshot = restarted.initialize().await.unwrap();

    assert_eq!(restarted_snapshot.recent_trades.len(), 3);
    assert_eq!(
        restarted_snapshot
            .current_position
            .as_ref()
            .map(|position| position.side),
        Some(PositionSide::Short)
    );
    assert_eq!(restarted.list_trades(10).await.unwrap().len(), 3);
}

#[tokio::test]
async fn live_credential_metadata_persists_without_plaintext_secrets() {
    let db = TempDatabase::new("live-credential-metadata");
    let secret_store = Arc::new(MemorySecretStore::new());
    let service = service_with_live(
        &db.url,
        Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
            bull: 20.0,
        }])),
        2,
        secret_store.clone(),
    )
    .await;
    service.initialize().await.unwrap();

    let credential = service
        .create_live_credential(CreateLiveCredentialRequest {
            alias: "Testnet".to_string(),
            environment: LiveEnvironment::Testnet,
            api_key: "abcd1234efgh5678".to_string(),
            api_secret: "super-secret".to_string(),
        })
        .await
        .unwrap();
    service
        .validate_live_credential(credential.id.clone())
        .await
        .unwrap();

    let restarted = service_with_live(
        &db.url,
        Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
            bull: 20.0,
        }])),
        2,
        secret_store,
    )
    .await;
    let snapshot = restarted.initialize().await.unwrap();

    assert_eq!(
        snapshot
            .live_status
            .active_credential
            .as_ref()
            .map(|credential| credential.api_key_hint.as_str()),
        Some("abcd…5678")
    );
    assert_eq!(
        snapshot
            .live_status
            .active_credential
            .as_ref()
            .map(|credential| credential.validation_status),
        Some(LiveCredentialValidationStatus::Valid)
    );

    let sqlite = std::fs::read(&db.path).unwrap();
    let sqlite_text = String::from_utf8_lossy(&sqlite);
    assert!(!sqlite_text.contains("super-secret"));
    assert!(!sqlite_text.contains("abcd1234efgh5678"));
}

#[tokio::test]
async fn live_shadow_and_preflight_cache_persist_across_sqlite_restart() {
    let db = TempDatabase::new("live-shadow-preflight-cache");
    let secret_store = Arc::new(MemorySecretStore::new());
    let service = service_with_live(
        &db.url,
        Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
            bull: 20.0,
        }])),
        2,
        secret_store.clone(),
    )
    .await;
    service.initialize().await.unwrap();

    let credential = service
        .create_live_credential(CreateLiveCredentialRequest {
            alias: "Testnet".to_string(),
            environment: LiveEnvironment::Testnet,
            api_key: "abcd1234efgh5678".to_string(),
            api_secret: "super-secret".to_string(),
        })
        .await
        .unwrap();
    service
        .validate_live_credential(credential.id.clone())
        .await
        .unwrap();
    service.refresh_live_readiness().await.unwrap();

    let shadow = service.start_live_shadow().await.unwrap();
    assert!(shadow.reconciliation.shadow.is_some());
    let preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();
    assert!(preview.intent.is_some());
    let preflight = service.run_live_preflight().await.unwrap();
    assert!(preflight.accepted);
    service.stop_live_shadow().await.unwrap();

    let restarted = service_with_live(
        &db.url,
        Arc::new(ReleaseGateMarket::new(vec![RangeResponse::Complete {
            bull: 20.0,
        }])),
        2,
        secret_store,
    )
    .await;
    let snapshot = restarted.initialize().await.unwrap();

    assert!(snapshot.live_status.reconciliation.shadow.is_some());
    assert_eq!(snapshot.live_status.recent_preflights.len(), 1);
    assert_eq!(
        snapshot.live_status.recent_preflights[0].symbol,
        Symbol::BtcUsdt
    );
    assert!(snapshot.live_status.recent_preflights[0].accepted);

    let sqlite = std::fs::read(&db.path).unwrap();
    let sqlite_text = String::from_utf8_lossy(&sqlite);
    assert!(!sqlite_text.contains("super-secret"));
}
