use std::collections::{BTreeMap, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use futures::StreamExt;
use rust_decimal::Decimal;

use relxen_domain::{
    build_live_order_preview, compute_aso_series, compute_performance, derive_signal_history,
    mark_to_market, quantize_down, reset_wallets, signal_from_points, validate_settings,
    warmup_candles_required, AsoCalculator, Candle, ConnectionState, ConnectionStatus,
    CreateLiveCredentialRequest, DisarmLiveModeRequest, ExecutionMode, LiveAccountShadow,
    LiveAccountSnapshot, LiveAssetBalance, LiveAutoExecutorRequest, LiveAutoExecutorStateKind,
    LiveAutoExecutorStatus, LiveBlockingReason, LiveCancelAllRequest, LiveCancelRequest,
    LiveCancelResult, LiveCredentialId, LiveCredentialMetadata, LiveCredentialSecret,
    LiveCredentialSummary, LiveCredentialValidationResult, LiveCredentialValidationStatus,
    LiveEnvironment, LiveExecutionAvailability, LiveExecutionRequest, LiveExecutionResult,
    LiveExecutionSnapshot, LiveExecutionState, LiveFillRecord, LiveFlattenRequest,
    LiveFlattenResult, LiveGateCheck, LiveIntentInput, LiveIntentLock, LiveIntentLockStatus,
    LiveKillSwitchRequest, LiveKillSwitchState, LiveModePreference, LiveOrderPreflightResult,
    LiveOrderPreview, LiveOrderRecord, LiveOrderSide, LiveOrderStatus, LiveOrderType,
    LivePositionSnapshot, LiveReadinessSnapshot, LiveReconciliationStatus, LiveRiskProfile,
    LiveRuntimeState, LiveShadowBalance, LiveShadowOrder, LiveShadowPosition,
    LiveShadowStreamState, LiveShadowStreamStatus, LiveStartCheck, LiveStateRecord,
    LiveStatusSnapshot, LiveSymbolRules, LiveUserDataEvent, LiveWarning, LogEvent, PaperEngine,
    PerformanceStats, Position, QuoteAsset, RuntimeStatus, SetLiveModePreferenceRequest, Settings,
    SignalEvent, SystemMetrics, Trade, UpdateLiveCredentialRequest, Wallet, ALLOWED_SYMBOLS,
};

use crate::events::{AppMetadata, BootstrapPayload, OutboundEvent};
use crate::history::{
    build_history_load_plan, merge_candles, select_closed_window, validate_closed_window,
};
use crate::ports::{
    EventPublisher, KlineRangeRequest, LiveExchangePort, MarketDataPort, MarketStreamEvent,
    MetricsPort, Repository, SecretStore, UnavailableLiveExchange, UnavailableSecretStore,
};
use crate::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct ServiceOptions {
    pub history_limit: usize,
    pub recent_signals_limit: usize,
    pub recent_trades_limit: usize,
    pub recent_logs_limit: usize,
    pub recovery_limit: usize,
    pub auto_start: bool,
    pub live_validation_ttl_ms: i64,
    pub live_snapshot_stale_ms: i64,
    pub live_shadow_stale_ms: i64,
    pub recent_preflight_limit: usize,
    pub recent_live_order_limit: usize,
    pub recent_live_fill_limit: usize,
    pub live_intent_ttl_ms: i64,
    pub enable_mainnet_canary_execution: bool,
    pub enable_testnet_drill_helpers: bool,
    pub live_user_stream_forced_reconnect_ms: i64,
    pub live_repair_recent_window_limit: usize,
}

impl Default for ServiceOptions {
    fn default() -> Self {
        Self {
            history_limit: 500,
            recent_signals_limit: 200,
            recent_trades_limit: 200,
            recent_logs_limit: 200,
            recovery_limit: 64,
            auto_start: true,
            live_validation_ttl_ms: 24 * 60 * 60 * 1_000,
            live_snapshot_stale_ms: 5 * 60 * 1_000,
            live_shadow_stale_ms: 90_000,
            recent_preflight_limit: 50,
            recent_live_order_limit: 50,
            recent_live_fill_limit: 100,
            live_intent_ttl_ms: 30_000,
            enable_mainnet_canary_execution: false,
            enable_testnet_drill_helpers: false,
            live_user_stream_forced_reconnect_ms: 24 * 60 * 60 * 1_000,
            live_repair_recent_window_limit: 100,
        }
    }
}

struct RuntimeHandle {
    stop_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarketEventOrigin {
    Live,
    Recovery,
}

#[derive(Debug)]
enum RecoveryDecision {
    NotNeeded,
    Recovered { recovered_closed: usize },
    HardResync { reason: String },
}

#[derive(Debug, Clone, Copy)]
struct RecoveryPlan {
    fetch_request: KlineRangeRequest,
    last_persisted_closed_open_time: i64,
    first_stream_open_time: i64,
    gap_closed_candles: usize,
    required_context_closed: usize,
    available_context_closed: usize,
}

struct AppState {
    metadata: AppMetadata,
    settings: Settings,
    runtime_status: RuntimeStatus,
    connection_state: ConnectionState,
    candles: Vec<Candle>,
    aso_points: Vec<relxen_domain::AsoPoint>,
    signals: Vec<SignalEvent>,
    engine: PaperEngine,
    performance: PerformanceStats,
    live_status: LiveStatusSnapshot,
    system_metrics: SystemMetrics,
    logs: Vec<LogEvent>,
    calculator: AsoCalculator,
    initialized: bool,
    last_partial_publish_ms: i64,
    resynced_live_events_remaining: usize,
}

impl AppState {
    fn snapshot(&self, options: &ServiceOptions) -> BootstrapPayload {
        let mut recent_signals = self.signals.clone();
        if recent_signals.len() > options.recent_signals_limit {
            recent_signals =
                recent_signals.split_off(recent_signals.len() - options.recent_signals_limit);
        }

        let mut recent_trades = self.engine.trades.clone();
        if recent_trades.len() > options.recent_trades_limit {
            recent_trades =
                recent_trades.split_off(recent_trades.len() - options.recent_trades_limit);
        }

        let mut recent_logs = self.logs.clone();
        if recent_logs.len() > options.recent_logs_limit {
            recent_logs = recent_logs.split_off(recent_logs.len() - options.recent_logs_limit);
        }

        BootstrapPayload {
            metadata: self.metadata.clone(),
            runtime_status: self.runtime_status.clone(),
            settings: self.settings.clone(),
            allowed_symbols: BootstrapPayload::allowed_symbols(),
            active_symbol: self.settings.active_symbol,
            candles: self.candles.clone(),
            aso_points: self.aso_points.clone(),
            recent_signals,
            recent_trades,
            current_position: self.engine.position.clone(),
            wallets: self.engine.wallets.values().cloned().collect(),
            performance: self.performance.clone(),
            connection_state: self.connection_state.clone(),
            live_status: self.live_status.clone(),
            system_metrics: self.system_metrics.clone(),
            recent_logs,
        }
    }
}

pub struct AppService {
    repository: Arc<dyn Repository>,
    market_data: Arc<dyn MarketDataPort>,
    secret_store: Arc<dyn SecretStore>,
    live_exchange: Arc<dyn LiveExchangePort>,
    metrics: Arc<dyn MetricsPort>,
    publisher: Arc<dyn EventPublisher>,
    options: ServiceOptions,
    state: Mutex<AppState>,
    runtime: Mutex<Option<RuntimeHandle>>,
    live_shadow_runtime: Mutex<Option<RuntimeHandle>>,
}

#[derive(Clone)]
pub struct LiveDependencies {
    pub secret_store: Arc<dyn SecretStore>,
    pub live_exchange: Arc<dyn LiveExchangePort>,
}

impl LiveDependencies {
    pub fn new(
        secret_store: Arc<dyn SecretStore>,
        live_exchange: Arc<dyn LiveExchangePort>,
    ) -> Self {
        Self {
            secret_store,
            live_exchange,
        }
    }

    pub fn unavailable() -> Self {
        Self::new(
            Arc::new(UnavailableSecretStore),
            Arc::new(UnavailableLiveExchange),
        )
    }
}

impl AppService {
    pub fn new(
        metadata: AppMetadata,
        repository: Arc<dyn Repository>,
        market_data: Arc<dyn MarketDataPort>,
        metrics: Arc<dyn MetricsPort>,
        publisher: Arc<dyn EventPublisher>,
        options: ServiceOptions,
    ) -> Arc<Self> {
        Self::new_with_live(
            metadata,
            repository,
            market_data,
            LiveDependencies::unavailable(),
            metrics,
            publisher,
            options,
        )
    }

    pub fn new_with_live(
        metadata: AppMetadata,
        repository: Arc<dyn Repository>,
        market_data: Arc<dyn MarketDataPort>,
        live_dependencies: LiveDependencies,
        metrics: Arc<dyn MetricsPort>,
        publisher: Arc<dyn EventPublisher>,
        options: ServiceOptions,
    ) -> Arc<Self> {
        let settings = Settings::default();
        let engine = PaperEngine::new(&settings, metadata.started_at);
        let system_metrics = metrics.snapshot();
        let runtime_status = RuntimeStatus {
            running: false,
            execution_mode: ExecutionMode::Paper,
            active_symbol: settings.active_symbol,
            timeframe: settings.timeframe,
            activity: None,
            last_error: None,
            started_at: None,
        };
        let connection_state = ConnectionState {
            status: ConnectionStatus::Disconnected,
            status_since: Some(metadata.started_at),
            last_message_time: None,
            reconnect_attempts: 0,
            resync_required: false,
            detail: None,
        };
        let performance = compute_performance(&engine.wallets, &engine.position, &engine.trades);
        let live_status = LiveStatusSnapshot {
            updated_at: metadata.started_at,
            ..LiveStatusSnapshot::default()
        };

        Arc::new(Self {
            repository,
            market_data,
            secret_store: live_dependencies.secret_store,
            live_exchange: live_dependencies.live_exchange,
            metrics,
            publisher,
            options,
            state: Mutex::new(AppState {
                metadata,
                settings: settings.clone(),
                runtime_status,
                connection_state,
                candles: Vec::new(),
                aso_points: Vec::new(),
                signals: Vec::new(),
                engine,
                performance,
                live_status,
                system_metrics,
                logs: Vec::new(),
                calculator: AsoCalculator::new(settings.aso_length, settings.aso_mode),
                initialized: false,
                last_partial_publish_ms: 0,
                resynced_live_events_remaining: 0,
            }),
            runtime: Mutex::new(None),
            live_shadow_runtime: Mutex::new(None),
        })
    }

    pub async fn initialize(self: &Arc<Self>) -> AppResult<BootstrapPayload> {
        let snapshot = self.rebuild_state("bootstrap").await?;
        let _ = self.repair_live_execution_recent_window().await;
        if self.options.auto_start {
            self.start_runtime().await?;
        }
        Ok(snapshot)
    }

    pub async fn bootstrap(self: &Arc<Self>) -> AppResult<BootstrapPayload> {
        self.rebuild_state("manual bootstrap").await
    }

    pub async fn get_bootstrap(&self) -> AppResult<BootstrapPayload> {
        let state = self.state.lock().await;
        if !state.initialized {
            return Err(AppError::NotFound(
                "application state is not initialized".to_string(),
            ));
        }
        Ok(state.snapshot(&self.options))
    }

    pub async fn get_settings(&self) -> AppResult<Settings> {
        Ok(self.state.lock().await.settings.clone())
    }

    pub async fn update_settings(
        self: &Arc<Self>,
        mut settings: Settings,
    ) -> AppResult<BootstrapPayload> {
        settings
            .available_symbols
            .retain(|symbol| ALLOWED_SYMBOLS.contains(symbol));
        if settings.available_symbols.is_empty() {
            settings.available_symbols = ALLOWED_SYMBOLS.to_vec();
        }
        validate_settings(&settings)?;

        let (restart_needed, was_running) = {
            let state = self.state.lock().await;
            if state.engine.position.is_some()
                && state.settings.active_symbol != settings.active_symbol
            {
                return Err(AppError::Conflict(
                    "cannot change active symbol while a position is open".to_string(),
                ));
            }
            let restart_needed = state.runtime_status.running
                && settings_requires_rebuild(&state.settings, &settings);
            (restart_needed, state.runtime_status.running)
        };

        info!(
            event = "settings_rebuild_started",
            active_symbol = %settings.active_symbol,
            timeframe = %settings.timeframe,
            restart_needed,
            auto_restart_on_apply = settings.auto_restart_on_apply,
            "starting settings rebuild"
        );
        self.set_runtime_activity(Some(relxen_domain::RuntimeActivity::Rebuilding))
            .await;

        let stopped_for_rebuild = restart_needed && was_running;
        if stopped_for_rebuild {
            self.stop_runtime().await?;
            self.set_runtime_activity(Some(relxen_domain::RuntimeActivity::Rebuilding))
                .await;
        }

        let mut result = self
            .rebuild_state_with_settings("settings apply", Some(settings.clone()))
            .await;

        if let Ok(snapshot) = result.as_mut() {
            if stopped_for_rebuild && settings.auto_restart_on_apply {
                self.start_runtime().await?;
                *snapshot = self.get_bootstrap().await?;
            }
        } else {
            if stopped_for_rebuild {
                if let Err(resume_error) = self.start_runtime().await {
                    warn!(
                        event = "settings_rebuild_failed",
                        active_symbol = %settings.active_symbol,
                        timeframe = %settings.timeframe,
                        detail = %resume_error,
                        "failed to resume previous runtime after settings rebuild failure"
                    );
                }
            }
        }

        self.set_runtime_activity(None).await;
        match result {
            Ok(snapshot) => {
                self.record_log("info", "settings", "settings updated".to_string())
                    .await?;
                info!(
                    event = "settings_rebuild_finished",
                    active_symbol = %snapshot.active_symbol,
                    timeframe = %snapshot.runtime_status.timeframe,
                    candles = snapshot.candles.len(),
                    "settings rebuild finished"
                );
                Ok(snapshot)
            }
            Err(error) => {
                warn!(
                    event = "settings_rebuild_failed",
                    active_symbol = %settings.active_symbol,
                    timeframe = %settings.timeframe,
                    detail = %error,
                    "settings rebuild failed"
                );
                let _ = self
                    .record_log(
                        "error",
                        "settings",
                        format!("settings rebuild failed: {error}"),
                    )
                    .await;
                Err(error)
            }
        }
    }

    pub async fn start_runtime(self: &Arc<Self>) -> AppResult<RuntimeStatus> {
        let mut runtime = self.runtime.lock().await;
        if runtime.is_some() {
            return Ok(self.state.lock().await.runtime_status.clone());
        }

        {
            let mut state = self.state.lock().await;
            state.runtime_status.running = true;
            state.runtime_status.activity = None;
            state.runtime_status.started_at = Some(now_ms());
            state.runtime_status.last_error = None;
        }
        self.publisher.publish(OutboundEvent::RuntimeChanged(
            self.state.lock().await.runtime_status.clone(),
        ));
        self.record_log("info", "runtime", "runtime started".to_string())
            .await?;

        let (stop_tx, stop_rx) = oneshot::channel();
        let service = Arc::clone(self);
        let join_handle = tokio::spawn(async move {
            service.run_runtime_loop(stop_rx).await;
        });
        *runtime = Some(RuntimeHandle {
            stop_tx,
            join_handle,
        });

        Ok(self.state.lock().await.runtime_status.clone())
    }

    pub async fn stop_runtime(&self) -> AppResult<RuntimeStatus> {
        let handle = self.runtime.lock().await.take();
        if let Some(handle) = handle {
            let _ = handle.stop_tx.send(());
            let _ = handle.join_handle.await;
        }

        {
            let mut state = self.state.lock().await;
            state.runtime_status.running = false;
            state.runtime_status.activity = None;
            state.connection_state.status = ConnectionStatus::Disconnected;
            state.connection_state.status_since = Some(now_ms());
            state.connection_state.reconnect_attempts = 0;
            state.connection_state.resync_required = false;
            state.connection_state.detail = Some("stopped by user".to_string());
            state.resynced_live_events_remaining = 0;
        }
        let status = self.state.lock().await.runtime_status.clone();
        self.publisher
            .publish(OutboundEvent::RuntimeChanged(status.clone()));
        self.record_log("info", "runtime", "runtime stopped".to_string())
            .await?;
        Ok(status)
    }

    pub async fn close_all(&self) -> AppResult<BootstrapPayload> {
        let mut state = self.state.lock().await;
        let price = state
            .engine
            .position
            .as_ref()
            .map(|position| position.mark_price)
            .or_else(|| state.candles.last().map(|candle| candle.close))
            .unwrap_or(0.0);

        let fee_rate = state.settings.fee_rate;
        state
            .engine
            .close_all(fee_rate, price, now_ms())
            .map_err(AppError::Validation)?;
        state.performance = compute_performance(
            &state.engine.wallets,
            &state.engine.position,
            &state.engine.trades,
        );
        let snapshot = state.snapshot(&self.options);
        let wallets: Vec<Wallet> = state.engine.wallets.values().cloned().collect();
        let position = state.engine.position.clone();
        let trade = state.engine.trades.last().cloned();
        drop(state);

        self.repository.save_wallets(&wallets).await?;
        self.repository.save_position(position.as_ref()).await?;
        if let Some(trade) = trade.as_ref() {
            self.repository.append_trade(trade).await?;
            info!(
                event = "trade_event_emitted",
                trade_id = %trade.id,
                symbol = %trade.symbol,
                action = ?trade.action,
                source = ?trade.source,
                "publishing trade websocket event"
            );
            self.publisher
                .publish(OutboundEvent::TradeAppended(trade.clone()));
        }
        self.publisher
            .publish(OutboundEvent::PositionUpdated(position));
        self.publisher
            .publish(OutboundEvent::WalletUpdated(wallets));
        self.publisher.publish(OutboundEvent::PerformanceUpdated(
            snapshot.performance.clone(),
        ));
        self.record_log("info", "paper", "manual full close executed".to_string())
            .await?;
        Ok(snapshot)
    }

    pub async fn reset_paper(&self) -> AppResult<BootstrapPayload> {
        let mut state = self.state.lock().await;
        let settings = state.settings.clone();
        state.engine.reset(&settings, now_ms());
        state.performance = compute_performance(
            &state.engine.wallets,
            &state.engine.position,
            &state.engine.trades,
        );
        let snapshot = state.snapshot(&self.options);
        let wallets: Vec<Wallet> = state.engine.wallets.values().cloned().collect();
        drop(state);

        self.repository.clear_trades().await?;
        self.repository.save_wallets(&wallets).await?;
        self.repository.save_position(None).await?;
        info!(
            event = "trade_event_emitted",
            reset = true,
            "publishing trade history reset websocket event"
        );
        self.publisher.publish(OutboundEvent::TradeHistoryReset);
        self.publisher.publish(OutboundEvent::PositionUpdated(None));
        self.publisher
            .publish(OutboundEvent::WalletUpdated(wallets));
        self.publisher.publish(OutboundEvent::PerformanceUpdated(
            snapshot.performance.clone(),
        ));
        self.record_log("info", "paper", "paper account reset".to_string())
            .await?;
        Ok(snapshot)
    }

    pub async fn list_trades(&self, limit: usize) -> AppResult<Vec<Trade>> {
        let mut trades = self.state.lock().await.engine.trades.clone();
        if trades.len() > limit {
            trades = trades.split_off(trades.len() - limit);
        }
        Ok(trades)
    }

    pub async fn list_signals(&self, limit: usize) -> AppResult<Vec<SignalEvent>> {
        let mut signals = self.state.lock().await.signals.clone();
        if signals.len() > limit {
            signals = signals.split_off(signals.len() - limit);
        }
        Ok(signals)
    }

    pub async fn list_logs(&self, limit: usize) -> AppResult<Vec<LogEvent>> {
        let mut logs = self.state.lock().await.logs.clone();
        if logs.len() > limit {
            logs = logs.split_off(logs.len() - limit);
        }
        Ok(logs)
    }

    pub async fn live_status(&self) -> AppResult<LiveStatusSnapshot> {
        Ok(self.state.lock().await.live_status.clone())
    }

    pub async fn list_live_credentials(&self) -> AppResult<Vec<LiveCredentialSummary>> {
        self.repository.list_live_credentials().await
    }

    pub async fn create_live_credential(
        &self,
        payload: CreateLiveCredentialRequest,
    ) -> AppResult<LiveCredentialSummary> {
        let alias = payload.alias.trim();
        if alias.is_empty() {
            return Err(AppError::Validation(
                "credential alias cannot be empty".to_string(),
            ));
        }
        validate_secret_input(&payload.api_key, &payload.api_secret)?;
        self.secret_store.ensure_available().await?;

        let now = now_ms();
        let id = LiveCredentialId::new(Uuid::new_v4().to_string());
        let secret = LiveCredentialSecret {
            api_key: payload.api_key,
            api_secret: payload.api_secret,
        };
        self.secret_store.store(&id, &secret).await?;
        let mut credential = LiveCredentialMetadata {
            id: id.clone(),
            alias: alias.to_string(),
            environment: payload.environment,
            api_key_hint: mask_api_key(&secret.api_key),
            validation_status: LiveCredentialValidationStatus::Unknown,
            last_validated_at: None,
            last_validation_error: None,
            is_active: false,
            created_at: now,
            updated_at: now,
        };
        self.repository.upsert_live_credential(&credential).await?;
        if self
            .repository
            .active_live_credential(payload.environment)
            .await?
            .is_none()
        {
            self.repository
                .select_live_credential(&id, payload.environment)
                .await?;
            credential.is_active = true;
            self.set_live_environment(payload.environment).await?;
        }
        info!(
            event = "credential_saved",
            credential_id = %credential.id,
            environment = %credential.environment,
            "saved live credential metadata and secure secret"
        );
        self.record_log("info", "live", "live credential saved".to_string())
            .await?;
        self.refresh_live_status_from_repository().await?;
        Ok(credential)
    }

    pub async fn update_live_credential(
        &self,
        id: LiveCredentialId,
        payload: UpdateLiveCredentialRequest,
    ) -> AppResult<LiveCredentialSummary> {
        let mut credential = self
            .repository
            .get_live_credential(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("live credential not found: {id}")))?;

        if let Some(alias) = payload.alias.as_deref() {
            let alias = alias.trim();
            if alias.is_empty() {
                return Err(AppError::Validation(
                    "credential alias cannot be empty".to_string(),
                ));
            }
            credential.alias = alias.to_string();
        }

        if let Some(environment) = payload.environment {
            credential.environment = environment;
        }

        match (payload.api_key, payload.api_secret) {
            (Some(api_key), Some(api_secret)) => {
                validate_secret_input(&api_key, &api_secret)?;
                self.secret_store.ensure_available().await?;
                self.secret_store
                    .store(
                        &id,
                        &LiveCredentialSecret {
                            api_key,
                            api_secret,
                        },
                    )
                    .await?;
                let secret = self.secret_store.read(&id).await?;
                credential.api_key_hint = mask_api_key(&secret.api_key);
                credential.validation_status = LiveCredentialValidationStatus::Unknown;
                credential.last_validated_at = None;
                credential.last_validation_error = None;
            }
            (None, None) => {}
            _ => {
                return Err(AppError::Validation(
                    "api_key and api_secret must be provided together".to_string(),
                ));
            }
        }

        credential.updated_at = now_ms();
        self.repository.upsert_live_credential(&credential).await?;
        if credential.is_active {
            self.repository
                .select_live_credential(&credential.id, credential.environment)
                .await?;
            self.set_live_environment(credential.environment).await?;
        }
        info!(
            event = "credential_saved",
            credential_id = %credential.id,
            environment = %credential.environment,
            "updated live credential metadata"
        );
        self.refresh_live_status_from_repository().await?;
        Ok(credential)
    }

    pub async fn delete_live_credential(&self, id: LiveCredentialId) -> AppResult<()> {
        let credential = self.repository.get_live_credential(&id).await?;
        if credential.is_none() {
            let _ = self.secret_store.delete(&id).await;
            return Ok(());
        }
        self.secret_store.delete(&id).await?;
        self.repository.delete_live_credential(&id).await?;
        let mut live_state = self.repository.load_live_state().await?;
        live_state.armed = false;
        live_state.updated_at = now_ms();
        self.repository.save_live_state(&live_state).await?;
        info!(
            event = "credential_deleted",
            credential_id = %id,
            "deleted live credential metadata and secure secret"
        );
        self.record_log("info", "live", "live credential deleted".to_string())
            .await?;
        self.refresh_live_status_from_repository().await?;
        Ok(())
    }

    pub async fn select_live_credential(
        &self,
        id: LiveCredentialId,
    ) -> AppResult<LiveStatusSnapshot> {
        let credential = self
            .repository
            .get_live_credential(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("live credential not found: {id}")))?;
        self.repository
            .select_live_credential(&id, credential.environment)
            .await?;
        self.set_live_environment(credential.environment).await?;
        self.refresh_live_status_from_repository().await
    }

    pub async fn validate_live_credential(
        &self,
        id: LiveCredentialId,
    ) -> AppResult<LiveCredentialValidationResult> {
        let mut credential = self
            .repository
            .get_live_credential(&id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("live credential not found: {id}")))?;
        info!(
            event = "credential_validation_started",
            credential_id = %id,
            environment = %credential.environment,
            "starting live credential validation"
        );
        let secret = match self.secret_store.read(&id).await {
            Ok(secret) => secret,
            Err(error @ AppError::SecureStoreUnavailable(_)) => {
                credential.validation_status =
                    LiveCredentialValidationStatus::SecureStoreUnavailable;
                credential.last_validated_at = Some(now_ms());
                credential.last_validation_error = Some(error.to_string());
                credential.updated_at = now_ms();
                self.repository.upsert_live_credential(&credential).await?;
                warn!(
                    event = "secure_store_unavailable",
                    credential_id = %id,
                    detail = %error,
                    "secure store unavailable during credential validation"
                );
                return Ok(LiveCredentialValidationResult {
                    credential_id: id,
                    environment: credential.environment,
                    status: LiveCredentialValidationStatus::SecureStoreUnavailable,
                    validated_at: now_ms(),
                    message: Some(error.to_string()),
                });
            }
            Err(error) => return Err(error),
        };

        let result = self
            .live_exchange
            .validate_credentials(credential.environment, &id, &secret)
            .await?;
        credential.validation_status = result.status;
        credential.last_validated_at = Some(result.validated_at);
        credential.last_validation_error = result.message.clone();
        credential.updated_at = now_ms();
        self.repository.upsert_live_credential(&credential).await?;

        if result.status.is_valid() {
            info!(
                event = "credential_validation_succeeded",
                credential_id = %id,
                environment = %credential.environment,
                "live credential validation succeeded"
            );
        } else {
            warn!(
                event = "credential_validation_failed",
                credential_id = %id,
                environment = %credential.environment,
                status = %result.status.as_str(),
                "live credential validation failed"
            );
        }
        self.refresh_live_status_from_repository().await?;
        Ok(result)
    }

    pub async fn refresh_live_readiness(&self) -> AppResult<LiveStatusSnapshot> {
        let snapshot = self.evaluate_live_readiness(true).await?;
        self.store_live_status(snapshot.clone()).await;
        info!(
            event = "live_readiness_refreshed",
            state = ?snapshot.state,
            environment = %snapshot.environment,
            blocking_reasons = ?snapshot.readiness.blocking_reasons,
            "live readiness refreshed"
        );
        Ok(snapshot)
    }

    pub async fn arm_live(&self) -> AppResult<LiveStatusSnapshot> {
        let mut snapshot = self.evaluate_live_readiness(true).await?;
        if !snapshot.readiness.can_arm {
            return Err(AppError::Conflict(format!(
                "live mode cannot be armed: {:?}",
                snapshot.readiness.blocking_reasons
            )));
        }
        let mut state = self.repository.load_live_state().await?;
        state.armed = true;
        state.mode_preference = LiveModePreference::LiveReadOnly;
        state.updated_at = now_ms();
        self.repository.save_live_state(&state).await?;
        snapshot = self.evaluate_live_readiness(false).await?;
        self.store_live_status(snapshot.clone()).await;
        info!(
            event = "live_armed",
            environment = %snapshot.environment,
            "live read-only mode armed"
        );
        self.record_log("info", "live", "live read-only mode armed".to_string())
            .await?;
        Ok(snapshot)
    }

    pub async fn disarm_live(
        &self,
        _request: DisarmLiveModeRequest,
    ) -> AppResult<LiveStatusSnapshot> {
        let mut live_state = self.repository.load_live_state().await?;
        live_state.armed = false;
        live_state.mode_preference = LiveModePreference::Paper;
        live_state.updated_at = now_ms();
        self.repository.save_live_state(&live_state).await?;
        let snapshot = self.evaluate_live_readiness(false).await?;
        self.store_live_status(snapshot.clone()).await;
        info!(event = "live_disarmed", "live read-only mode disarmed");
        self.record_log("info", "live", "live read-only mode disarmed".to_string())
            .await?;
        Ok(snapshot)
    }

    pub async fn set_live_mode_preference(
        &self,
        request: SetLiveModePreferenceRequest,
    ) -> AppResult<LiveStatusSnapshot> {
        let mut live_state = self.repository.load_live_state().await?;
        live_state.mode_preference = request.mode_preference;
        if request.mode_preference == LiveModePreference::Paper {
            live_state.armed = false;
        }
        live_state.updated_at = now_ms();
        self.repository.save_live_state(&live_state).await?;
        let snapshot = self.evaluate_live_readiness(false).await?;
        self.store_live_status(snapshot.clone()).await;
        Ok(snapshot)
    }

    pub async fn live_start_check(&self) -> AppResult<LiveStartCheck> {
        let mut snapshot = self.evaluate_live_readiness(false).await?;
        if !snapshot
            .readiness
            .blocking_reasons
            .contains(&LiveBlockingReason::ExecutionNotImplemented)
        {
            snapshot
                .readiness
                .blocking_reasons
                .push(LiveBlockingReason::ExecutionNotImplemented);
        }
        snapshot.state = LiveRuntimeState::StartBlocked;
        snapshot.readiness.state = LiveRuntimeState::StartBlocked;
        snapshot.execution_availability = live_execution_unavailable();
        self.store_live_status(snapshot.clone()).await;
        info!(
            event = "live_start_blocked",
            reason = %LiveBlockingReason::ExecutionNotImplemented.as_str(),
            "autonomous live start remains blocked"
        );
        Ok(LiveStartCheck {
            allowed: false,
            blocking_reasons: snapshot.readiness.blocking_reasons.clone(),
            message:
                "Autonomous live start is not implemented; use manual TESTNET execution controls."
                    .to_string(),
            readiness: snapshot.readiness,
        })
    }

    pub async fn start_live_shadow(self: &Arc<Self>) -> AppResult<LiveStatusSnapshot> {
        let mut runtime = self.live_shadow_runtime.lock().await;
        if runtime.is_some() {
            return self.live_status().await;
        }

        let (credential, secret, environment) = self.active_live_secret().await?;
        if !credential.validation_status.is_valid() {
            return Err(AppError::Conflict(
                "live shadow sync requires a validated active credential".to_string(),
            ));
        }

        let mut account = self
            .live_exchange
            .fetch_account_snapshot(environment, &secret)
            .await?;
        let account_mode = self
            .live_exchange
            .fetch_account_mode(environment, &secret)
            .await?;
        account.position_mode = account_mode.position_mode;
        account.multi_assets_margin = account_mode.multi_assets_margin;
        account.account_mode_checked_at = Some(account_mode.fetched_at);
        let active_symbol = self.state.lock().await.settings.active_symbol;
        let rules = self
            .live_exchange
            .fetch_symbol_rules(environment, active_symbol)
            .await?;
        let now = now_ms();
        let mut shadow = account_snapshot_to_shadow(account, now);
        shadow.last_rest_sync_at = Some(now);
        shadow.updated_at = now;
        shadow.ambiguous = false;
        shadow.divergence_reasons.clear();
        self.repository.save_live_shadow(&shadow).await?;

        let mut reconciliation = LiveReconciliationStatus {
            state: LiveRuntimeState::ShadowStarting,
            stream: LiveShadowStreamStatus {
                state: LiveShadowStreamState::Starting,
                environment,
                status_since: now,
                started_at: Some(now),
                last_rest_sync_at: Some(now),
                detail: Some("starting user-data stream".to_string()),
                ..LiveShadowStreamStatus::default()
            },
            shadow: Some(shadow.clone()),
            blocking_reasons: Vec::new(),
            warnings: Vec::new(),
            updated_at: now,
        };
        self.repository
            .save_live_reconciliation(&reconciliation)
            .await?;
        self.publish_reconciliation(reconciliation.clone()).await;

        let listen_key = self
            .live_exchange
            .create_listen_key(environment, &secret)
            .await?;
        info!(
            event = "listen_key_created",
            environment = %environment,
            "created Binance USD-M user-data stream listenKey"
        );
        let stream = self
            .live_exchange
            .subscribe_user_data(environment, &listen_key)
            .await?;

        reconciliation.state = LiveRuntimeState::ShadowRunning;
        reconciliation.stream.state = LiveShadowStreamState::Running;
        reconciliation.stream.listen_key_hint = Some(mask_listen_key(&listen_key));
        reconciliation.stream.status_since = now_ms();
        reconciliation.stream.detail = Some("user-data stream running".to_string());
        reconciliation.updated_at = now_ms();
        self.repository
            .save_live_reconciliation(&reconciliation)
            .await?;
        self.publish_reconciliation(reconciliation).await;

        let mut status = self.refresh_live_status_from_repository().await?;
        status.symbol_rules = Some(rules);
        self.store_live_status(status.clone()).await;
        self.record_log("info", "live", "live shadow sync started".to_string())
            .await?;

        let (stop_tx, stop_rx) = oneshot::channel();
        let service = Arc::clone(self);
        let join_handle = tokio::spawn(async move {
            service
                .run_live_shadow_loop(
                    stop_rx,
                    environment,
                    credential.id,
                    secret,
                    listen_key,
                    stream,
                )
                .await;
        });
        *runtime = Some(RuntimeHandle {
            stop_tx,
            join_handle,
        });

        Ok(status)
    }

    pub async fn stop_live_shadow(&self) -> AppResult<LiveStatusSnapshot> {
        let handle = self.live_shadow_runtime.lock().await.take();
        if let Some(handle) = handle {
            let _ = handle.stop_tx.send(());
            let _ = handle.join_handle.await;
        }
        let mut reconciliation = self.load_reconciliation_cache().await?;
        reconciliation.state = LiveRuntimeState::ArmedReadOnly;
        reconciliation.stream.state = LiveShadowStreamState::Stopped;
        reconciliation.stream.status_since = now_ms();
        reconciliation.stream.detail = Some("stopped by operator".to_string());
        reconciliation.updated_at = now_ms();
        self.repository
            .save_live_reconciliation(&reconciliation)
            .await?;
        self.publish_reconciliation(reconciliation).await;
        info!(event = "live_shadow_stopped", "live shadow sync stopped");
        self.record_log("info", "live", "live shadow sync stopped".to_string())
            .await?;
        self.refresh_live_status_from_repository().await
    }

    pub async fn refresh_live_shadow(&self) -> AppResult<LiveStatusSnapshot> {
        let (_credential, secret, environment) = self.active_live_secret().await?;
        let mut account = self
            .live_exchange
            .fetch_account_snapshot(environment, &secret)
            .await?;
        let account_mode = self
            .live_exchange
            .fetch_account_mode(environment, &secret)
            .await?;
        account.position_mode = account_mode.position_mode;
        account.multi_assets_margin = account_mode.multi_assets_margin;
        account.account_mode_checked_at = Some(account_mode.fetched_at);
        let now = now_ms();
        let mut shadow = account_snapshot_to_shadow(account, now);
        shadow.last_rest_sync_at = Some(now);
        shadow.ambiguous = false;
        shadow.divergence_reasons.clear();
        self.repository.save_live_shadow(&shadow).await?;
        let mut reconciliation = self.load_reconciliation_cache().await?;
        reconciliation.state = LiveRuntimeState::ShadowRunning;
        reconciliation.shadow = Some(shadow);
        reconciliation.stream.last_rest_sync_at = Some(now);
        reconciliation.stream.stale = false;
        reconciliation.blocking_reasons.clear();
        reconciliation.updated_at = now;
        self.repository
            .save_live_reconciliation(&reconciliation)
            .await?;
        self.publish_reconciliation(reconciliation).await;
        info!(
            event = "live_shadow_resynced",
            "live shadow state refreshed from REST"
        );
        self.refresh_live_status_from_repository().await
    }

    pub async fn build_live_intent_preview(
        &self,
        order_type: LiveOrderType,
        limit_price: Option<Decimal>,
    ) -> AppResult<LiveOrderPreview> {
        let live_status = self.state.lock().await.live_status.clone();
        let settings = self.state.lock().await.settings.clone();
        let latest_signal = self.state.lock().await.signals.last().cloned();
        let reference_price = self
            .state
            .lock()
            .await
            .candles
            .last()
            .map(|candle| Decimal::from_str(&candle.close.to_string()).unwrap_or(Decimal::ZERO))
            .unwrap_or(Decimal::ZERO);
        let rules = live_status
            .symbol_rules
            .clone()
            .ok_or_else(|| AppError::Conflict("symbol rules are missing".to_string()))?;
        let shadow = live_status
            .reconciliation
            .shadow
            .clone()
            .ok_or_else(|| AppError::Conflict("live shadow state is missing".to_string()))?;
        let preview = build_live_order_preview(LiveIntentInput {
            environment: live_status.environment,
            symbol: settings.active_symbol,
            settings,
            rules,
            shadow,
            latest_signal,
            order_type,
            reference_price,
            limit_price,
            now_ms: now_ms(),
        });
        info!(
            event = if preview.intent.is_some() { "live_intent_built" } else { "live_intent_blocked" },
            blocking_reasons = ?preview.blocking_reasons,
            "live order intent preview evaluated"
        );
        let mut status = self.state.lock().await.live_status.clone();
        status.intent_preview = Some(preview.clone());
        status.state = if preview.intent.is_some() && preview.blocking_reasons.is_empty() {
            LiveRuntimeState::PreflightReady
        } else {
            LiveRuntimeState::PreflightBlocked
        };
        status.updated_at = now_ms();
        self.store_live_status(status.clone()).await;
        let _ = self.refresh_live_status_from_repository().await;
        self.publisher
            .publish(OutboundEvent::LiveIntentPreviewUpdated(Box::new(
                preview.clone(),
            )));
        Ok(preview)
    }

    pub async fn run_live_preflight(&self) -> AppResult<LiveOrderPreflightResult> {
        let (credential, secret, environment) = self.active_live_secret().await?;
        let preview =
            if let Some(preview) = self.state.lock().await.live_status.intent_preview.clone() {
                preview
            } else {
                self.build_live_intent_preview(LiveOrderType::Market, None)
                    .await?
            };
        let now = now_ms();
        let Some(intent) = preview.intent.clone() else {
            let result = LiveOrderPreflightResult {
                id: Uuid::new_v4().to_string(),
                credential_id: Some(credential.id),
                environment,
                symbol: self.state.lock().await.settings.active_symbol,
                side: None,
                order_type: None,
                payload: BTreeMap::new(),
                accepted: false,
                exchange_error_code: None,
                exchange_error_message: Some(preview.message),
                local_blocking_reason: preview.blocking_reasons.first().copied(),
                source_signal_id: None,
                message: "PREFLIGHT BLOCKED locally. No exchange request was sent.".to_string(),
                created_at: now,
            };
            self.repository.append_live_preflight(&result).await?;
            self.publish_preflight(result.clone()).await;
            return Ok(result);
        };
        if environment != LiveEnvironment::Testnet {
            let result = LiveOrderPreflightResult {
                id: Uuid::new_v4().to_string(),
                credential_id: Some(credential.id),
                environment,
                symbol: intent.symbol,
                side: Some(intent.side),
                order_type: Some(intent.order_type),
                payload: intent.exchange_payload,
                accepted: false,
                exchange_error_code: None,
                exchange_error_message: None,
                local_blocking_reason: Some(LiveBlockingReason::PreflightNotSupportedOnMainnet),
                source_signal_id: intent.source_signal_id,
                message: "PREFLIGHT BLOCKED: testnet-only in this batch.".to_string(),
                created_at: now,
            };
            self.repository.append_live_preflight(&result).await?;
            self.publish_preflight(result.clone()).await;
            return Ok(result);
        }
        info!(event = "live_preflight_started", symbol = %intent.symbol, side = ?intent.side, "starting Binance order/test preflight");
        let exchange_result = self
            .live_exchange
            .preflight_order_test(environment, &secret, &intent.exchange_payload)
            .await;
        let (accepted, error_message) = match exchange_result {
            Ok(()) => {
                info!(event = "live_preflight_passed", symbol = %intent.symbol, "order/test preflight passed");
                (true, None)
            }
            Err(error) => {
                warn!(event = "live_preflight_failed", detail = %error, "order/test preflight failed");
                (false, Some(error.to_string()))
            }
        };
        let result = LiveOrderPreflightResult {
            id: Uuid::new_v4().to_string(),
            credential_id: Some(credential.id),
            environment,
            symbol: intent.symbol,
            side: Some(intent.side),
            order_type: Some(intent.order_type),
            payload: intent.exchange_payload,
            accepted,
            exchange_error_code: None,
            exchange_error_message: error_message,
            local_blocking_reason: None,
            source_signal_id: intent.source_signal_id,
            message: if accepted {
                "PREFLIGHT PASSED. No order was placed.".to_string()
            } else {
                "PREFLIGHT FAILED. No order was placed.".to_string()
            },
            created_at: now,
        };
        self.repository.append_live_preflight(&result).await?;
        self.publish_preflight(result.clone()).await;
        Ok(result)
    }

    pub async fn list_live_preflights(
        &self,
        limit: usize,
    ) -> AppResult<Vec<LiveOrderPreflightResult>> {
        self.repository.list_live_preflights(limit).await
    }

    pub async fn list_live_orders(&self, limit: usize) -> AppResult<Vec<LiveOrderRecord>> {
        self.repository.list_live_orders(limit).await
    }

    pub async fn list_live_fills(&self, limit: usize) -> AppResult<Vec<LiveFillRecord>> {
        self.repository.list_live_fills(limit).await
    }

    pub async fn repair_live_execution_recent_window(&self) -> AppResult<LiveStatusSnapshot> {
        let (_credential, secret, environment) = match self.active_live_secret().await {
            Ok(parts) => parts,
            Err(_) => return self.refresh_live_status_from_repository().await,
        };
        let symbol = self.state.lock().await.settings.active_symbol;
        let recent_orders = self
            .repository
            .list_live_orders(self.options.recent_live_order_limit)
            .await?;
        let mut repaired_any = false;
        for order in recent_orders
            .into_iter()
            .filter(|order| order.symbol == symbol && order.status.is_open())
            .take(self.options.live_repair_recent_window_limit)
        {
            info!(
                event = "live_order_repair_started",
                client_order_id = %order.client_order_id,
                recent_window_only = true,
                "repairing recent-window live order from authoritative exchange state"
            );
            match self
                .live_exchange
                .query_order(
                    environment,
                    &secret,
                    order.symbol,
                    Some(&order.client_order_id),
                    order.exchange_order_id.as_deref(),
                )
                .await
            {
                Ok(Some(exchange_order)) => {
                    let repaired = merge_exchange_order(order, exchange_order);
                    self.repository.upsert_live_order(&repaired).await?;
                    self.publish_order_and_execution(repaired, false).await?;
                    repaired_any = true;
                }
                Ok(None) => {
                    let mut unknown = order;
                    unknown.status = LiveOrderStatus::UnknownNeedsRepair;
                    unknown.last_error =
                        Some("recent-window repair could not find order".to_string());
                    unknown.updated_at = now_ms();
                    self.repository.upsert_live_order(&unknown).await?;
                    self.publish_order_and_execution(unknown, false).await?;
                    repaired_any = true;
                }
                Err(error) => {
                    warn!(
                        event = "live_order_repair_finished",
                        client_order_id = %order.client_order_id,
                        detail = %error,
                        "recent-window live order repair failed"
                    );
                }
            }
        }
        if let Ok(fills) = self
            .live_exchange
            .list_user_trades(
                environment,
                &secret,
                symbol,
                self.options.live_repair_recent_window_limit,
            )
            .await
        {
            for fill in fills {
                self.repository.append_live_fill(&fill).await?;
                self.publish_fill_and_execution(fill).await?;
                repaired_any = true;
            }
        }
        let snapshot = self.refresh_live_status_from_repository().await?;
        if repaired_any {
            self.publisher.publish(OutboundEvent::LiveExecutionResynced);
        }
        Ok(snapshot)
    }

    pub async fn engage_live_kill_switch(
        &self,
        request: LiveKillSwitchRequest,
    ) -> AppResult<LiveStatusSnapshot> {
        let now = now_ms();
        let state = LiveKillSwitchState {
            engaged: true,
            reason: request
                .reason
                .or_else(|| Some("operator_engaged".to_string())),
            engaged_at: Some(now),
            released_at: None,
            updated_at: now,
        };
        self.repository.save_live_kill_switch(&state).await?;
        info!(event = "kill_switch_engaged", "live kill switch engaged");
        let snapshot = self.refresh_live_status_from_repository().await?;
        self.publisher
            .publish(OutboundEvent::LiveKillSwitchUpdated(state));
        Ok(snapshot)
    }

    pub async fn release_live_kill_switch(
        &self,
        request: LiveKillSwitchRequest,
    ) -> AppResult<LiveStatusSnapshot> {
        let now = now_ms();
        let state = LiveKillSwitchState {
            engaged: false,
            reason: request
                .reason
                .or_else(|| Some("operator_released".to_string())),
            engaged_at: None,
            released_at: Some(now),
            updated_at: now,
        };
        self.repository.save_live_kill_switch(&state).await?;
        info!(event = "kill_switch_released", "live kill switch released");
        let snapshot = self.refresh_live_status_from_repository().await?;
        self.publisher
            .publish(OutboundEvent::LiveKillSwitchUpdated(state));
        Ok(snapshot)
    }

    pub async fn configure_live_risk_profile(
        &self,
        mut profile: LiveRiskProfile,
    ) -> AppResult<LiveStatusSnapshot> {
        profile.configured = true;
        profile.updated_at = now_ms();
        self.repository.save_live_risk_profile(&profile).await?;
        let snapshot = self.refresh_live_status_from_repository().await?;
        Ok(snapshot)
    }

    pub async fn start_live_auto_executor(
        &self,
        request: LiveAutoExecutorRequest,
    ) -> AppResult<LiveStatusSnapshot> {
        let now = now_ms();
        let live_state = self.repository.load_live_state().await?;
        let mut auto = self.repository.load_live_auto_executor().await?;
        if live_state.environment != LiveEnvironment::Testnet {
            auto.state = LiveAutoExecutorStateKind::Blocked;
            auto.blocking_reasons = vec![LiveBlockingReason::MainnetAutoBlocked];
            auto.last_message = Some("Auto execution is TESTNET-only.".to_string());
            auto.updated_at = now;
            self.repository.save_live_auto_executor(&auto).await?;
            self.publisher
                .publish(OutboundEvent::LiveAutoStateUpdated(auto.clone()));
            return self.refresh_live_status_from_repository().await;
        }
        if !request.confirm_testnet_auto {
            auto.state = LiveAutoExecutorStateKind::Blocked;
            auto.blocking_reasons = vec![LiveBlockingReason::AutoExecutorStopped];
            auto.last_message =
                Some("TESTNET auto start requires explicit confirmation.".to_string());
            auto.updated_at = now;
            self.repository.save_live_auto_executor(&auto).await?;
            self.publisher
                .publish(OutboundEvent::LiveAutoStateUpdated(auto.clone()));
            return self.refresh_live_status_from_repository().await;
        }
        let status = self.refresh_live_status_from_repository().await?;
        if !status.execution.can_submit
            && !matches!(
                status.execution.blocking_reasons.as_slice(),
                [LiveBlockingReason::IntentUnavailable] | []
            )
        {
            auto.state = LiveAutoExecutorStateKind::Blocked;
            auto.blocking_reasons = status.execution.blocking_reasons.clone();
            auto.last_message = Some("Auto execution gates are blocked.".to_string());
        } else {
            auto.state = LiveAutoExecutorStateKind::Running;
            auto.environment = LiveEnvironment::Testnet;
            auto.order_type = LiveOrderType::Market;
            auto.started_at = Some(now);
            auto.stopped_at = None;
            auto.blocking_reasons.clear();
            auto.last_message = Some("TESTNET auto executor running.".to_string());
        }
        auto.updated_at = now;
        self.repository.save_live_auto_executor(&auto).await?;
        info!(event = "auto_executor_started", state = ?auto.state, "live auto executor start requested");
        self.publisher
            .publish(OutboundEvent::LiveAutoStateUpdated(auto));
        self.refresh_live_status_from_repository().await
    }

    pub async fn stop_live_auto_executor(&self) -> AppResult<LiveStatusSnapshot> {
        let now = now_ms();
        let mut auto = self.repository.load_live_auto_executor().await?;
        auto.state = LiveAutoExecutorStateKind::Stopped;
        auto.stopped_at = Some(now);
        auto.blocking_reasons = vec![LiveBlockingReason::AutoExecutorStopped];
        auto.last_message = Some("TESTNET auto executor stopped.".to_string());
        auto.updated_at = now;
        self.repository.save_live_auto_executor(&auto).await?;
        info!(
            event = "auto_executor_stopped",
            "live auto executor stopped"
        );
        self.publisher
            .publish(OutboundEvent::LiveAutoStateUpdated(auto));
        self.refresh_live_status_from_repository().await
    }

    pub async fn drill_replay_latest_auto_signal(&self) -> AppResult<LiveStatusSnapshot> {
        if !self.options.enable_testnet_drill_helpers {
            return Err(AppError::Live(
                "testnet drill helpers are disabled by server policy".to_string(),
            ));
        }
        let live_state = self.repository.load_live_state().await?;
        if live_state.environment != LiveEnvironment::Testnet {
            return Err(AppError::Conflict(
                "testnet auto drill is only available on TESTNET".to_string(),
            ));
        }
        let auto = self.repository.load_live_auto_executor().await?;
        if auto.state != LiveAutoExecutorStateKind::Running {
            return Err(AppError::Conflict(
                "start TESTNET auto executor before running the auto drill helper".to_string(),
            ));
        }

        let latest_signal = {
            let persisted = self
                .repository
                .list_signals(32)
                .await?
                .into_iter()
                .filter(|signal| signal.closed_only)
                .max_by_key(|signal| signal.open_time);
            if let Some(signal) = persisted {
                signal
            } else {
                let state = self.state.lock().await;
                state
                    .signals
                    .iter()
                    .filter(|signal| signal.closed_only)
                    .max_by_key(|signal| signal.open_time)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::NotFound(
                            "no closed-candle signal is available for the TESTNET auto drill"
                                .to_string(),
                        )
                    })?
            }
        };
        let reference_price = if let Some(close) = {
            let state = self.state.lock().await;
            state.candles.last().map(|candle| candle.close)
        } {
            close
        } else {
            self.repository
                .load_recent_klines(latest_signal.symbol, latest_signal.timeframe, 1)
                .await?
                .last()
                .map(|candle| candle.close)
                .ok_or_else(|| {
                    AppError::Conflict(
                        "latest market price is unavailable for TESTNET auto drill".to_string(),
                    )
                })?
        };
        info!(
            event = "testnet_auto_drill_replay_started",
            signal_id = %latest_signal.id,
            signal_open_time = latest_signal.open_time,
            "replaying latest persisted closed-candle signal through TESTNET auto executor"
        );
        self.maybe_auto_execute_signal(latest_signal, reference_price)
            .await?;
        self.refresh_live_status_from_repository().await
    }

    pub async fn execute_live_current_preview(
        &self,
        request: LiveExecutionRequest,
    ) -> AppResult<LiveExecutionResult> {
        let now = now_ms();
        let (credential, secret, environment) = self.active_live_secret().await?;
        let status = self.refresh_live_status_from_repository().await?;
        let Some(preview) = status.intent_preview else {
            return Ok(blocked_execution_result(
                LiveBlockingReason::IntentUnavailable,
                "No live intent preview exists.",
                now,
            ));
        };
        let Some(intent) = preview.intent else {
            return Ok(blocked_execution_result(
                preview
                    .blocking_reasons
                    .first()
                    .copied()
                    .unwrap_or(LiveBlockingReason::IntentUnavailable),
                "Live intent preview is blocked.",
                now,
            ));
        };
        if let Some(intent_id) = request.intent_id.as_deref() {
            if intent_id != intent.id {
                return Ok(blocked_execution_result(
                    LiveBlockingReason::PreviewMismatch,
                    "Displayed preview no longer matches the execution request.",
                    now,
                ));
            }
        }
        if environment == LiveEnvironment::Testnet && !request.confirm_testnet {
            return Ok(blocked_execution_result(
                LiveBlockingReason::ExecutionStatusUnknown,
                "Execution requires explicit TESTNET confirmation.",
                now,
            ));
        }
        if environment == LiveEnvironment::Mainnet {
            let required = mainnet_confirmation_phrase(&intent);
            if !self.options.enable_mainnet_canary_execution {
                self.publisher.publish(OutboundEvent::LiveExecutionBlocked {
                    reason: LiveBlockingReason::MainnetCanaryDisabled
                        .as_str()
                        .to_string(),
                });
                return Ok(blocked_execution_result(
                    LiveBlockingReason::MainnetCanaryDisabled,
                    "MAINNET canary execution is disabled by server policy.",
                    now,
                ));
            }
            if !status.risk_profile.configured {
                return Ok(blocked_execution_result(
                    LiveBlockingReason::MainnetCanaryRiskProfileMissing,
                    "MAINNET canary requires an explicit operator-configured risk profile.",
                    now,
                ));
            }
            if !request.confirm_mainnet_canary
                || request.confirmation_text.as_deref() != Some(required.as_str())
            {
                return Ok(blocked_execution_result(
                    LiveBlockingReason::MainnetConfirmationMissing,
                    &format!("MAINNET canary requires exact confirmation: {required}"),
                    now,
                ));
            }
        }
        if !status.execution.can_submit {
            let reason = status
                .execution
                .blocking_reasons
                .first()
                .copied()
                .unwrap_or(LiveBlockingReason::ExecutionStatusUnknown);
            self.publisher.publish(OutboundEvent::LiveExecutionBlocked {
                reason: reason.as_str().to_string(),
            });
            return Ok(blocked_execution_result(
                reason,
                &format!("{} execution blocked: {}", environment, reason.as_str()),
                now,
            ));
        }

        let client_order_id = client_order_id("rx_exec");
        let mut payload = intent.exchange_payload.clone();
        payload.insert("newClientOrderId".to_string(), client_order_id.clone());
        payload.insert("newOrderRespType".to_string(), "ACK".to_string());
        if intent.reduce_only {
            payload.insert("reduceOnly".to_string(), "true".to_string());
        }

        let mut local_order = LiveOrderRecord {
            id: client_order_id.clone(),
            credential_id: Some(credential.id.clone()),
            environment,
            symbol: intent.symbol,
            side: intent.side,
            order_type: intent.order_type,
            status: LiveOrderStatus::SubmitPending,
            client_order_id: client_order_id.clone(),
            exchange_order_id: None,
            quantity: intent.quantity.clone(),
            price: intent.price.clone(),
            executed_qty: "0".to_string(),
            avg_price: None,
            reduce_only: intent.reduce_only,
            time_in_force: intent.time_in_force.clone(),
            intent_id: Some(intent.id.clone()),
            intent_hash: Some(intent.intent_hash.clone()),
            source_signal_id: intent.source_signal_id.clone(),
            source_open_time: intent.source_open_time,
            reason: intent.reason.clone(),
            payload: payload.clone(),
            response_type: Some("ACK".to_string()),
            self_trade_prevention_mode: None,
            price_match: None,
            expire_reason: None,
            last_error: None,
            submitted_at: now,
            updated_at: now,
        };
        self.repository.upsert_live_order(&local_order).await?;
        self.publish_order_and_execution(local_order.clone(), true)
            .await?;
        info!(
            event = "live_order_submitted",
            client_order_id = %client_order_id,
            symbol = %intent.symbol,
            side = %intent.side.as_binance(),
            order_type = %intent.order_type.as_binance(),
            "local submit-pending order persisted before exchange submission"
        );

        match self
            .live_exchange
            .submit_order(environment, &secret, &payload)
            .await
        {
            Ok(exchange_order) => {
                local_order = merge_exchange_ack(local_order, exchange_order);
                self.repository.upsert_live_order(&local_order).await?;
                self.publish_order_and_execution(local_order.clone(), false)
                    .await?;
                info!(
                    event = "live_order_acknowledged",
                    client_order_id = %local_order.client_order_id,
                    exchange_order_id = ?local_order.exchange_order_id,
                    status = %local_order.status.as_str(),
                    "Binance acknowledged order submission; lifecycle waits for authoritative reconciliation"
                );
                Ok(LiveExecutionResult {
                    accepted: true,
                    order: Some(local_order),
                    blocking_reason: None,
                    message:
                        "Order submitted with ACK handling. Final lifecycle waits for authoritative exchange reconciliation."
                            .to_string(),
                    created_at: now_ms(),
                })
            }
            Err(error) => {
                warn!(
                    event = "live_order_repair_started",
                    client_order_id = %client_order_id,
                    detail = %error,
                    "submission result ambiguous or rejected; querying authoritative order state"
                );
                match self
                    .live_exchange
                    .query_order(
                        environment,
                        &secret,
                        intent.symbol,
                        Some(&client_order_id),
                        None,
                    )
                    .await
                {
                    Ok(Some(exchange_order)) => {
                        local_order = merge_exchange_order(local_order, exchange_order);
                        self.repository.upsert_live_order(&local_order).await?;
                        self.publish_order_and_execution(local_order.clone(), false)
                            .await?;
                        info!(
                            event = "live_order_repair_finished",
                            client_order_id = %client_order_id,
                            repaired = true,
                            "submission state repaired from exchange order query"
                        );
                        Ok(LiveExecutionResult {
                            accepted: true,
                            order: Some(local_order),
                            blocking_reason: None,
                            message: "TESTNET order state repaired from exchange query after ambiguous submission.".to_string(),
                            created_at: now_ms(),
                        })
                    }
                    Ok(None) | Err(_) => {
                        local_order.status = LiveOrderStatus::UnknownNeedsRepair;
                        local_order.last_error = Some(error.to_string());
                        local_order.updated_at = now_ms();
                        self.repository.upsert_live_order(&local_order).await?;
                        self.publish_order_and_execution(local_order.clone(), false)
                            .await?;
                        warn!(
                            event = "execution_degraded",
                            client_order_id = %client_order_id,
                            detail = %error,
                            "submission outcome could not be authoritatively repaired"
                        );
                        Ok(LiveExecutionResult {
                            accepted: false,
                            order: Some(local_order),
                            blocking_reason: Some(LiveBlockingReason::ExecutionStatusUnknown),
                            message: "Execution status is unknown. New submissions are blocked until repair.".to_string(),
                            created_at: now_ms(),
                        })
                    }
                }
            }
        }
    }

    pub async fn cancel_live_order(
        &self,
        request: LiveCancelRequest,
    ) -> AppResult<LiveCancelResult> {
        let now = now_ms();
        let (_credential, secret, environment) = self.active_live_secret().await?;
        if environment == LiveEnvironment::Testnet && !request.confirm_testnet {
            return Ok(blocked_cancel_result(
                LiveBlockingReason::ExecutionStatusUnknown,
                "Cancel requires explicit TESTNET confirmation.",
                now,
            ));
        }
        let mut order = self
            .repository
            .get_live_order(&request.order_ref)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("live order not found: {}", request.order_ref))
            })?;
        if environment == LiveEnvironment::Mainnet {
            let required = format!("CANCEL MAINNET {} {}", order.symbol, order.client_order_id);
            if !self.options.enable_mainnet_canary_execution {
                return Ok(blocked_cancel_result(
                    LiveBlockingReason::MainnetCanaryDisabled,
                    "MAINNET canary cancel is disabled by server policy.",
                    now,
                ));
            }
            if !request.confirm_mainnet_canary
                || request.confirmation_text.as_deref() != Some(required.as_str())
            {
                return Ok(blocked_cancel_result(
                    LiveBlockingReason::MainnetConfirmationMissing,
                    &format!("MAINNET canary cancel requires exact confirmation: {required}"),
                    now,
                ));
            }
        }
        order.status = LiveOrderStatus::CancelPending;
        order.updated_at = now;
        self.repository.upsert_live_order(&order).await?;
        self.publish_order_and_execution(order.clone(), false)
            .await?;
        info!(
            event = "live_cancel_requested",
            order_ref = %request.order_ref,
            client_order_id = %order.client_order_id,
            "submitting Binance testnet cancel request"
        );
        match self
            .live_exchange
            .cancel_order(
                environment,
                &secret,
                order.symbol,
                Some(&order.client_order_id),
                order.exchange_order_id.as_deref(),
            )
            .await
        {
            Ok(exchange_order) => {
                let order = merge_exchange_order(order, exchange_order);
                self.repository.upsert_live_order(&order).await?;
                self.publish_order_and_execution(order.clone(), false)
                    .await?;
                info!(
                    event = "live_cancel_succeeded",
                    client_order_id = %order.client_order_id,
                    status = %order.status.as_str(),
                    "Binance testnet cancel acknowledged"
                );
                Ok(LiveCancelResult {
                    accepted: true,
                    order: Some(order),
                    blocking_reason: None,
                    message:
                        "TESTNET cancel submitted; final state follows exchange reconciliation."
                            .to_string(),
                    created_at: now_ms(),
                })
            }
            Err(error) => {
                order.status = LiveOrderStatus::UnknownNeedsRepair;
                order.last_error = Some(error.to_string());
                order.updated_at = now_ms();
                self.repository.upsert_live_order(&order).await?;
                self.publish_order_and_execution(order.clone(), false)
                    .await?;
                warn!(
                    event = "live_cancel_failed",
                    client_order_id = %order.client_order_id,
                    detail = %error,
                    "Binance testnet cancel failed"
                );
                Ok(LiveCancelResult {
                    accepted: false,
                    order: Some(order),
                    blocking_reason: Some(LiveBlockingReason::CancelFailed),
                    message: format!("TESTNET cancel failed: {error}"),
                    created_at: now_ms(),
                })
            }
        }
    }

    pub async fn cancel_all_live_orders(
        &self,
        request: LiveCancelAllRequest,
    ) -> AppResult<Vec<LiveCancelResult>> {
        let live_state = self.repository.load_live_state().await?;
        if live_state.environment == LiveEnvironment::Testnet && !request.confirm_testnet {
            return Ok(vec![blocked_cancel_result(
                LiveBlockingReason::ExecutionStatusUnknown,
                "TESTNET cancel-all requires explicit confirmation.",
                now_ms(),
            )]);
        }
        if live_state.environment == LiveEnvironment::Mainnet {
            let symbol = self.state.lock().await.settings.active_symbol;
            let required = format!("CANCEL ALL MAINNET {symbol}");
            if !self.options.enable_mainnet_canary_execution {
                return Ok(vec![blocked_cancel_result(
                    LiveBlockingReason::MainnetCanaryDisabled,
                    "MAINNET canary cancel-all is disabled by server policy.",
                    now_ms(),
                )]);
            }
            if !request.confirm_mainnet_canary
                || request.confirmation_text.as_deref() != Some(required.as_str())
            {
                return Ok(vec![blocked_cancel_result(
                    LiveBlockingReason::MainnetConfirmationMissing,
                    &format!("MAINNET canary cancel-all requires exact confirmation: {required}"),
                    now_ms(),
                )]);
            }
        }
        let symbol = self.state.lock().await.settings.active_symbol;
        let orders = self
            .repository
            .list_live_orders(self.options.recent_live_order_limit)
            .await?;
        let open_orders: Vec<_> = orders
            .into_iter()
            .filter(|order| order.symbol == symbol && order.status.is_open())
            .collect();
        let mut results = Vec::new();
        for order in open_orders {
            results.push(
                self.cancel_live_order(LiveCancelRequest {
                    order_ref: order.id,
                    confirm_testnet: request.confirm_testnet,
                    confirm_mainnet_canary: request.confirm_mainnet_canary,
                    confirmation_text: Some(format!(
                        "CANCEL MAINNET {} {}",
                        order.symbol, order.client_order_id
                    )),
                })
                .await?,
            );
        }
        Ok(results)
    }

    pub async fn flatten_live_position(
        &self,
        request: LiveFlattenRequest,
    ) -> AppResult<LiveFlattenResult> {
        let now = now_ms();
        let (credential, secret, environment) = self.active_live_secret().await?;
        if environment == LiveEnvironment::Testnet && !request.confirm_testnet {
            return Ok(blocked_flatten_result(
                LiveBlockingReason::ExecutionStatusUnknown,
                "Flatten requires explicit TESTNET confirmation.",
                now,
            ));
        }
        let active_symbol = self.state.lock().await.settings.active_symbol;
        if environment == LiveEnvironment::Mainnet {
            let required = format!("FLATTEN MAINNET {active_symbol}");
            if !self.options.enable_mainnet_canary_execution {
                return Ok(blocked_flatten_result(
                    LiveBlockingReason::MainnetCanaryDisabled,
                    "MAINNET canary flatten is disabled by server policy.",
                    now,
                ));
            }
            if !request.confirm_mainnet_canary
                || request.confirmation_text.as_deref() != Some(required.as_str())
            {
                return Ok(blocked_flatten_result(
                    LiveBlockingReason::MainnetConfirmationMissing,
                    &format!("MAINNET canary flatten requires exact confirmation: {required}"),
                    now,
                ));
            }
        }
        let status = self.refresh_live_status_from_repository().await?;
        if status.execution.blocking_reasons.iter().any(|reason| {
            matches!(
                reason,
                LiveBlockingReason::ShadowStateAmbiguous
                    | LiveBlockingReason::StaleShadowState
                    | LiveBlockingReason::UnsupportedAccountMode
            )
        }) {
            return Ok(blocked_flatten_result(
                LiveBlockingReason::ShadowStateAmbiguous,
                "Flatten blocked because live shadow state is not safe enough.",
                now,
            ));
        }
        let Some(shadow) = status.reconciliation.shadow.clone() else {
            return Ok(blocked_flatten_result(
                LiveBlockingReason::AccountSnapshotMissing,
                "Flatten blocked because live shadow position is missing.",
                now,
            ));
        };
        let Some(position) = shadow
            .positions
            .iter()
            .find(|position| position.symbol == active_symbol)
            .cloned()
        else {
            return Ok(LiveFlattenResult {
                accepted: true,
                canceled_orders: Vec::new(),
                flatten_order: None,
                blocking_reason: None,
                message: "No active-symbol shadow position to flatten.".to_string(),
                created_at: now,
            });
        };
        let position_amt = Decimal::from_str(&position.position_amt).unwrap_or(Decimal::ZERO);
        if position_amt == Decimal::ZERO {
            return Ok(LiveFlattenResult {
                accepted: true,
                canceled_orders: Vec::new(),
                flatten_order: None,
                blocking_reason: None,
                message: "Active-symbol shadow position is already flat.".to_string(),
                created_at: now,
            });
        }
        let rules = status
            .symbol_rules
            .clone()
            .ok_or_else(|| AppError::Conflict("symbol rules are missing".to_string()))?;
        let step = rules
            .filters
            .step_size
            .and_then(|step| Decimal::from_str(&step.to_string()).ok())
            .unwrap_or(Decimal::ZERO);
        let quantity = quantize_down(position_amt.abs(), step);
        if quantity <= Decimal::ZERO {
            return Ok(blocked_flatten_result(
                LiveBlockingReason::PrecisionInvalid,
                "Flatten quantity rounded to zero.",
                now,
            ));
        }
        let side = if position_amt > Decimal::ZERO {
            LiveOrderSide::Sell
        } else {
            LiveOrderSide::Buy
        };
        self.publisher.publish(OutboundEvent::LiveFlattenStarted {
            symbol: active_symbol,
        });
        info!(
            event = "live_flatten_requested",
            symbol = %active_symbol,
            side = %side.as_binance(),
            quantity = %decimal_to_exchange_string(quantity),
            "starting testnet flatten flow"
        );
        let canceled = self
            .cancel_all_live_orders(LiveCancelAllRequest {
                confirm_testnet: request.confirm_testnet,
                confirm_mainnet_canary: request.confirm_mainnet_canary,
                confirmation_text: Some(format!("CANCEL ALL MAINNET {active_symbol}")),
            })
            .await?;
        let client_order_id = client_order_id("rx_flat");
        let mut payload = BTreeMap::new();
        payload.insert("symbol".to_string(), active_symbol.as_str().to_string());
        payload.insert("side".to_string(), side.as_binance().to_string());
        payload.insert("type".to_string(), "MARKET".to_string());
        payload.insert("quantity".to_string(), decimal_to_exchange_string(quantity));
        payload.insert("reduceOnly".to_string(), "true".to_string());
        payload.insert("newClientOrderId".to_string(), client_order_id.clone());
        payload.insert("newOrderRespType".to_string(), "ACK".to_string());
        let local_order = LiveOrderRecord {
            id: client_order_id.clone(),
            credential_id: Some(credential.id),
            environment,
            symbol: active_symbol,
            side,
            order_type: LiveOrderType::Market,
            status: LiveOrderStatus::SubmitPending,
            client_order_id: client_order_id.clone(),
            exchange_order_id: None,
            quantity: payload
                .get("quantity")
                .cloned()
                .unwrap_or_else(|| "0".to_string()),
            price: None,
            executed_qty: "0".to_string(),
            avg_price: None,
            reduce_only: true,
            time_in_force: None,
            intent_id: None,
            intent_hash: None,
            source_signal_id: None,
            source_open_time: None,
            reason: "manual_flatten".to_string(),
            payload: payload.clone(),
            response_type: Some("ACK".to_string()),
            self_trade_prevention_mode: None,
            price_match: None,
            expire_reason: None,
            last_error: None,
            submitted_at: now,
            updated_at: now,
        };
        self.repository.upsert_live_order(&local_order).await?;
        self.publish_order_and_execution(local_order.clone(), true)
            .await?;
        match self
            .live_exchange
            .submit_order(environment, &secret, &payload)
            .await
        {
            Ok(exchange_order) => {
                let order = merge_exchange_ack(local_order, exchange_order);
                self.repository.upsert_live_order(&order).await?;
                self.publish_order_and_execution(order.clone(), false)
                    .await?;
                self.publisher.publish(OutboundEvent::LiveFlattenFinished {
                    message:
                        "TESTNET flatten submitted; final state follows exchange reconciliation."
                            .to_string(),
                });
                info!(
                    event = "live_flatten_succeeded",
                    client_order_id = %order.client_order_id,
                    "testnet flatten order acknowledged"
                );
                Ok(LiveFlattenResult {
                    accepted: true,
                    canceled_orders: canceled
                        .into_iter()
                        .filter_map(|result| result.order)
                        .collect(),
                    flatten_order: Some(order),
                    blocking_reason: None,
                    message:
                        "TESTNET flatten submitted; final state follows exchange reconciliation."
                            .to_string(),
                    created_at: now_ms(),
                })
            }
            Err(error) => {
                warn!(
                    event = "live_flatten_failed",
                    detail = %error,
                    "testnet flatten order submission failed"
                );
                let failed_order = LiveOrderRecord {
                    status: LiveOrderStatus::UnknownNeedsRepair,
                    last_error: Some(error.to_string()),
                    updated_at: now_ms(),
                    ..local_order
                };
                self.repository.upsert_live_order(&failed_order).await?;
                self.publish_order_and_execution(failed_order.clone(), false)
                    .await?;
                Ok(LiveFlattenResult {
                    accepted: false,
                    canceled_orders: canceled
                        .into_iter()
                        .filter_map(|result| result.order)
                        .collect(),
                    flatten_order: Some(failed_order),
                    blocking_reason: Some(LiveBlockingReason::FlattenFailed),
                    message: format!("TESTNET flatten failed: {error}"),
                    created_at: now_ms(),
                })
            }
        }
    }

    async fn set_live_environment(&self, environment: LiveEnvironment) -> AppResult<()> {
        let mut live_state = self.repository.load_live_state().await?;
        live_state.environment = environment;
        live_state.updated_at = now_ms();
        self.repository.save_live_state(&live_state).await
    }

    async fn refresh_live_status_from_repository(&self) -> AppResult<LiveStatusSnapshot> {
        let snapshot = self.load_live_status_snapshot().await?;
        self.store_live_status(snapshot.clone()).await;
        Ok(snapshot)
    }

    async fn store_live_status(&self, snapshot: LiveStatusSnapshot) {
        {
            let mut state = self.state.lock().await;
            state.live_status = snapshot.clone();
        }
        self.publisher
            .publish(OutboundEvent::LiveStatusUpdated(Box::new(snapshot)));
    }

    async fn load_live_status_snapshot(&self) -> AppResult<LiveStatusSnapshot> {
        let live_state = self.repository.load_live_state().await?;
        let active_credential = self
            .repository
            .active_live_credential(live_state.environment)
            .await?;
        let current_live = self.state.lock().await.live_status.clone();
        let reconciliation = self.load_reconciliation_cache().await?;
        let account_snapshot = refresh_account_snapshot_from_shadow(
            current_live.account_snapshot,
            reconciliation.shadow.as_ref(),
        );
        Ok(build_live_status(LiveStatusBuildInput {
            live_state,
            active_credential,
            account_snapshot,
            symbol_rules: current_live.symbol_rules,
            reconciliation,
            intent_preview: current_live.intent_preview,
            recent_preflights: self
                .repository
                .list_live_preflights(self.options.recent_preflight_limit)
                .await
                .unwrap_or_default(),
            execution: self.load_execution_cache().await?,
            kill_switch: self.repository.load_live_kill_switch().await?,
            risk_profile: self.repository.load_live_risk_profile().await?,
            auto_executor: self.repository.load_live_auto_executor().await?,
            paper_position_open: self.current_paper_position_open().await,
            extra_blocking: Vec::new(),
            now_ms: now_ms(),
            options: &self.options,
        }))
    }

    async fn load_reconciliation_cache(&self) -> AppResult<LiveReconciliationStatus> {
        let mut reconciliation = self
            .repository
            .load_live_reconciliation()
            .await?
            .unwrap_or_default();
        if let Some(shadow) = self.repository.load_live_shadow().await? {
            reconciliation.shadow = Some(shadow);
        }
        Ok(reconciliation)
    }

    async fn load_execution_cache(&self) -> AppResult<LiveExecutionSnapshot> {
        let mut execution = self
            .repository
            .load_live_execution()
            .await?
            .unwrap_or_default();
        execution.recent_orders = self
            .repository
            .list_live_orders(self.options.recent_live_order_limit)
            .await
            .unwrap_or_default();
        execution.recent_fills = self
            .repository
            .list_live_fills(self.options.recent_live_fill_limit)
            .await
            .unwrap_or_default();
        execution.repair_recent_window_only = true;
        execution.mainnet_canary_enabled = self.options.enable_mainnet_canary_execution;
        execution.kill_switch_engaged = self.repository.load_live_kill_switch().await?.engaged;
        execution.active_order = execution
            .recent_orders
            .iter()
            .rev()
            .find(|order| order.status.is_open())
            .cloned();
        Ok(execution)
    }

    async fn current_paper_position_open(&self) -> bool {
        self.state.lock().await.engine.position.is_some()
    }

    async fn active_live_secret(
        &self,
    ) -> AppResult<(LiveCredentialSummary, LiveCredentialSecret, LiveEnvironment)> {
        let live_state = self.repository.load_live_state().await?;
        let credential = self
            .repository
            .active_live_credential(live_state.environment)
            .await?
            .ok_or_else(|| AppError::Conflict("no active live credential selected".to_string()))?;
        let secret = self.secret_store.read(&credential.id).await?;
        Ok((credential, secret, live_state.environment))
    }

    async fn publish_reconciliation(&self, reconciliation: LiveReconciliationStatus) {
        self.publisher
            .publish(OutboundEvent::LiveShadowStatusUpdated(Box::new(
                reconciliation.clone(),
            )));
        if let Some(shadow) = reconciliation.shadow.as_ref() {
            self.publisher
                .publish(OutboundEvent::LiveShadowAccountUpdated(Box::new(
                    shadow.clone(),
                )));
        }
    }

    async fn publish_preflight(&self, result: LiveOrderPreflightResult) {
        let recent_preflights = self
            .repository
            .list_live_preflights(self.options.recent_preflight_limit)
            .await
            .unwrap_or_default();
        {
            let mut state = self.state.lock().await;
            state.live_status.recent_preflights = recent_preflights;
            state.live_status.updated_at = now_ms();
        }
        self.publisher
            .publish(OutboundEvent::LivePreflightResultAppended(Box::new(result)));
    }

    async fn publish_order_and_execution(
        &self,
        order: LiveOrderRecord,
        submitted: bool,
    ) -> AppResult<()> {
        let snapshot = self.load_live_status_snapshot().await?;
        self.repository
            .save_live_execution(&snapshot.execution)
            .await?;
        {
            let mut state = self.state.lock().await;
            state.live_status = snapshot.clone();
        }
        self.publisher
            .publish(OutboundEvent::LiveStatusUpdated(Box::new(snapshot.clone())));
        self.publisher.publish(if submitted {
            OutboundEvent::LiveOrderSubmitted(Box::new(order))
        } else {
            OutboundEvent::LiveOrderUpdated(Box::new(order))
        });
        self.publisher
            .publish(OutboundEvent::LiveExecutionStateUpdated(Box::new(
                snapshot.execution,
            )));
        Ok(())
    }

    async fn publish_fill_and_execution(&self, fill: LiveFillRecord) -> AppResult<()> {
        let snapshot = self.load_live_status_snapshot().await?;
        self.repository
            .save_live_execution(&snapshot.execution)
            .await?;
        {
            let mut state = self.state.lock().await;
            state.live_status = snapshot.clone();
        }
        self.publisher
            .publish(OutboundEvent::LiveFillAppended(Box::new(fill)));
        self.publisher
            .publish(OutboundEvent::LiveExecutionStateUpdated(Box::new(
                snapshot.execution,
            )));
        Ok(())
    }

    async fn reconcile_order_trade_update(
        &self,
        environment: LiveEnvironment,
        order: LiveShadowOrder,
    ) -> AppResult<()> {
        let existing = if let Some(client_order_id) = order.client_order_id.as_deref() {
            self.repository.get_live_order(client_order_id).await?
        } else {
            self.repository.get_live_order(&order.order_id).await?
        };
        let now = now_ms();
        let mut record = LiveOrderRecord {
            id: existing
                .as_ref()
                .map(|record| record.id.clone())
                .or_else(|| order.client_order_id.clone())
                .unwrap_or_else(|| format!("order_{}", order.order_id)),
            credential_id: existing
                .as_ref()
                .and_then(|record| record.credential_id.clone()),
            environment,
            symbol: order.symbol,
            side: order.side,
            order_type: order.order_type,
            status: live_order_status_from_exchange_status(&order.status),
            client_order_id: order
                .client_order_id
                .clone()
                .unwrap_or_else(|| format!("order_{}", order.order_id)),
            exchange_order_id: Some(order.order_id.clone()),
            quantity: order.original_qty.clone(),
            price: order.price.clone(),
            executed_qty: order.executed_qty.clone(),
            avg_price: order.avg_price.clone(),
            reduce_only: order.reduce_only,
            time_in_force: order.time_in_force.clone(),
            intent_id: existing
                .as_ref()
                .and_then(|record| record.intent_id.clone()),
            intent_hash: existing
                .as_ref()
                .and_then(|record| record.intent_hash.clone()),
            source_signal_id: existing
                .as_ref()
                .and_then(|record| record.source_signal_id.clone()),
            source_open_time: existing.as_ref().and_then(|record| record.source_open_time),
            reason: existing
                .as_ref()
                .map(|record| record.reason.clone())
                .unwrap_or_else(|| "user_data_stream".to_string()),
            payload: existing
                .as_ref()
                .map(|record| record.payload.clone())
                .unwrap_or_default(),
            response_type: existing
                .as_ref()
                .and_then(|record| record.response_type.clone()),
            self_trade_prevention_mode: order.self_trade_prevention_mode.clone(),
            price_match: order.price_match.clone(),
            expire_reason: order.expire_reason.clone(),
            last_error: None,
            submitted_at: existing
                .as_ref()
                .map(|record| record.submitted_at)
                .unwrap_or(order.last_update_time),
            updated_at: order.last_update_time.max(now),
        };
        if record.status == LiveOrderStatus::Filled {
            record.executed_qty = order.executed_qty.clone();
        }
        self.repository.upsert_live_order(&record).await?;
        self.publish_order_and_execution(record.clone(), false)
            .await?;
        info!(
            event = "live_order_updated",
            client_order_id = %record.client_order_id,
            status = %record.status.as_str(),
            "user-data stream reconciled live order state"
        );

        if let Some(fill) = fill_from_shadow_order(&record, &order) {
            self.repository.append_live_fill(&fill).await?;
            self.publish_fill_and_execution(fill.clone()).await?;
            info!(
                event = "live_fill_recorded",
                fill_id = %fill.id,
                client_order_id = ?fill.client_order_id,
                quantity = %fill.quantity,
                price = %fill.price,
                "recorded authoritative user-data fill"
            );
        }
        Ok(())
    }

    async fn repair_live_shadow_from_rest(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
    ) -> AppResult<()> {
        let mut account = self
            .live_exchange
            .fetch_account_snapshot(environment, secret)
            .await?;
        let account_mode = self
            .live_exchange
            .fetch_account_mode(environment, secret)
            .await?;
        account.position_mode = account_mode.position_mode;
        account.multi_assets_margin = account_mode.multi_assets_margin;
        account.account_mode_checked_at = Some(account_mode.fetched_at);
        let now = now_ms();
        let mut shadow = account_snapshot_to_shadow(account, now);
        shadow.last_rest_sync_at = Some(now);
        shadow.ambiguous = false;
        shadow.divergence_reasons.clear();
        self.repository.save_live_shadow(&shadow).await?;
        let mut reconciliation = self.load_reconciliation_cache().await?;
        reconciliation.state = LiveRuntimeState::ShadowRunning;
        reconciliation.shadow = Some(shadow);
        reconciliation.stream.state = LiveShadowStreamState::Running;
        reconciliation.stream.last_rest_sync_at = Some(now);
        reconciliation.stream.stale = false;
        reconciliation.blocking_reasons.clear();
        reconciliation.updated_at = now;
        self.repository
            .save_live_reconciliation(&reconciliation)
            .await?;
        self.publish_reconciliation(reconciliation).await;
        Ok(())
    }

    async fn run_live_shadow_loop(
        self: Arc<Self>,
        mut stop_rx: oneshot::Receiver<()>,
        environment: LiveEnvironment,
        _credential_id: LiveCredentialId,
        secret: LiveCredentialSecret,
        mut listen_key: String,
        mut stream: crate::ports::LiveUserDataStream,
    ) {
        let mut keepalive_interval = interval(Duration::from_secs(30 * 60));
        let mut forced_reconnect = Box::pin(sleep(Duration::from_millis(
            self.options.live_user_stream_forced_reconnect_ms.max(1) as u64,
        )));
        loop {
            tokio::select! {
                _ = &mut stop_rx => {
                    let _ = self.live_exchange.close_listen_key(environment, &secret, &listen_key).await;
                    info!(event = "live_shadow_stopped", "shadow user-data loop stopped");
                    break;
                }
                _ = keepalive_interval.tick() => {
                    if let Err(error) = self.live_exchange.keepalive_listen_key(environment, &secret, &listen_key).await {
                        warn!(event = "listen_key_keepalive_failed", detail = %error, "listenKey keepalive failed");
                        self.degrade_live_shadow(format!("listenKey keepalive failed: {error}")).await;
                    }
                }
                _ = &mut forced_reconnect => {
                    info!(
                        event = "live_shadow_forced_reconnect_started",
                        environment = %environment,
                        "forcing user-data stream reconnect before 24-hour websocket limit"
                    );
                    if let Err(error) = self.repair_live_shadow_from_rest(environment, &secret).await {
                        warn!(
                            event = "live_shadow_degraded",
                            detail = %error,
                            "forced reconnect REST repair failed"
                        );
                        self.degrade_live_shadow(format!("forced user-data reconnect repair failed: {error}")).await;
                        break;
                    }
                    let _ = self.live_exchange.close_listen_key(environment, &secret, &listen_key).await;
                    match self.live_exchange.create_listen_key(environment, &secret).await {
                        Ok(new_listen_key) => {
                            match self.live_exchange.subscribe_user_data(environment, &new_listen_key).await {
                                Ok(new_stream) => {
                                    listen_key = new_listen_key;
                                    stream = new_stream;
                                    keepalive_interval = interval(Duration::from_secs(30 * 60));
                                    forced_reconnect = Box::pin(sleep(Duration::from_millis(
                                        self.options.live_user_stream_forced_reconnect_ms.max(1) as u64,
                                    )));
                                    let mut reconciliation = self.load_reconciliation_cache().await.unwrap_or_default();
                                    reconciliation.stream.listen_key_hint = Some(mask_listen_key(&listen_key));
                                    reconciliation.stream.state = LiveShadowStreamState::Running;
                                    reconciliation.stream.status_since = now_ms();
                                    reconciliation.stream.reconnect_attempts = reconciliation.stream.reconnect_attempts.saturating_add(1);
                                    reconciliation.stream.detail = Some("forced 24h user-data reconnect completed".to_string());
                                    reconciliation.updated_at = now_ms();
                                    let _ = self.repository.save_live_reconciliation(&reconciliation).await;
                                    self.publish_reconciliation(reconciliation).await;
                                    let _ = self.refresh_live_status_from_repository().await;
                                    info!(
                                        event = "live_shadow_reconnected",
                                        environment = %environment,
                                        "forced user-data stream reconnect and REST repair completed"
                                    );
                                }
                                Err(error) => {
                                    self.degrade_live_shadow(format!("forced user-data reconnect subscribe failed: {error}")).await;
                                    break;
                                }
                            }
                        }
                        Err(error) => {
                            self.degrade_live_shadow(format!("forced user-data reconnect listenKey create failed: {error}")).await;
                            break;
                        }
                    }
                }
                event = stream.next() => {
                    match event {
                        Some(Ok(event)) => {
                            if let Err(error) = self.apply_live_user_data_event(environment, event).await {
                                warn!(event = "live_shadow_degraded", detail = %error, "failed applying user-data event");
                                self.degrade_live_shadow(error.to_string()).await;
                            }
                        }
                        Some(Err(error)) => {
                            warn!(event = "live_shadow_degraded", detail = %error, "user-data stream error");
                            self.degrade_live_shadow(error.to_string()).await;
                            break;
                        }
                        None => {
                            warn!(event = "live_shadow_degraded", "user-data stream ended");
                            self.degrade_live_shadow("user-data stream ended".to_string()).await;
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn apply_live_user_data_event(
        &self,
        environment: LiveEnvironment,
        event: LiveUserDataEvent,
    ) -> AppResult<()> {
        let now = now_ms();
        match event {
            LiveUserDataEvent::AccountUpdate(mut shadow) => {
                shadow.environment = environment;
                shadow.ambiguous = false;
                shadow.updated_at = now;
                shadow.last_event_time = shadow.last_event_time.or(Some(now));
                self.repository.save_live_shadow(&shadow).await?;
                let mut reconciliation = self.load_reconciliation_cache().await?;
                reconciliation.state = LiveRuntimeState::ShadowRunning;
                reconciliation.stream.state = LiveShadowStreamState::Running;
                reconciliation.stream.last_event_time = shadow.last_event_time;
                reconciliation.stream.stale = false;
                reconciliation.stream.detail = Some("ACCOUNT_UPDATE applied".to_string());
                reconciliation.shadow = Some(shadow);
                reconciliation.blocking_reasons.clear();
                reconciliation.updated_at = now;
                self.repository
                    .save_live_reconciliation(&reconciliation)
                    .await?;
                self.publish_reconciliation(reconciliation).await;
            }
            LiveUserDataEvent::OrderTradeUpdate(order) => {
                let order = *order;
                let order_for_execution = order.clone();
                let mut shadow = self
                    .repository
                    .load_live_shadow()
                    .await?
                    .unwrap_or_default();
                shadow.environment = environment;
                shadow
                    .open_orders
                    .retain(|item| item.order_id != order.order_id);
                if !matches!(
                    order.status.as_str(),
                    "FILLED" | "CANCELED" | "REJECTED" | "EXPIRED" | "EXPIRED_IN_MATCH"
                ) {
                    shadow.open_orders.push(order);
                }
                shadow.last_event_time = Some(now);
                shadow.updated_at = now;
                shadow.ambiguous = false;
                self.repository.save_live_shadow(&shadow).await?;
                let mut reconciliation = self.load_reconciliation_cache().await?;
                reconciliation.state = LiveRuntimeState::ShadowRunning;
                reconciliation.stream.state = LiveShadowStreamState::Running;
                reconciliation.stream.last_event_time = Some(now);
                reconciliation.shadow = Some(shadow);
                reconciliation.updated_at = now;
                self.repository
                    .save_live_reconciliation(&reconciliation)
                    .await?;
                self.publish_reconciliation(reconciliation).await;
                self.reconcile_order_trade_update(environment, order_for_execution)
                    .await?;
            }
            LiveUserDataEvent::AccountConfigUpdate {
                event_time,
                position_mode,
                ..
            } => {
                let mut shadow = self
                    .repository
                    .load_live_shadow()
                    .await?
                    .unwrap_or_default();
                shadow.environment = environment;
                if let Some(position_mode) = position_mode {
                    shadow.position_mode = Some(position_mode);
                }
                shadow.last_event_time = Some(event_time);
                shadow.updated_at = now;
                self.repository.save_live_shadow(&shadow).await?;
            }
            LiveUserDataEvent::ListenKeyExpired { .. } => {
                self.degrade_live_shadow("listenKey expired".to_string())
                    .await;
            }
            LiveUserDataEvent::Unknown { .. } => {}
        }
        let _ = self.refresh_live_status_from_repository().await;
        Ok(())
    }

    async fn degrade_live_shadow(&self, detail: String) {
        let mut reconciliation = self.load_reconciliation_cache().await.unwrap_or_default();
        reconciliation.state = LiveRuntimeState::ShadowDegraded;
        reconciliation.stream.state = LiveShadowStreamState::Degraded;
        reconciliation.stream.status_since = now_ms();
        reconciliation.stream.detail = Some(detail);
        reconciliation.stream.stale = true;
        if !reconciliation
            .blocking_reasons
            .contains(&LiveBlockingReason::ShadowStateAmbiguous)
        {
            reconciliation
                .blocking_reasons
                .push(LiveBlockingReason::ShadowStateAmbiguous);
        }
        reconciliation.updated_at = now_ms();
        let _ = self
            .repository
            .save_live_reconciliation(&reconciliation)
            .await;
        self.publish_reconciliation(reconciliation).await;
        let _ = self.refresh_live_status_from_repository().await;
    }

    async fn evaluate_live_readiness(&self, fetch_remote: bool) -> AppResult<LiveStatusSnapshot> {
        let live_state = self.repository.load_live_state().await?;
        let active_credential = self
            .repository
            .active_live_credential(live_state.environment)
            .await?;
        let paper_position_open = self.current_paper_position_open().await;
        let kill_switch = self.repository.load_live_kill_switch().await?;
        let risk_profile = self.repository.load_live_risk_profile().await?;
        let auto_executor = self.repository.load_live_auto_executor().await?;
        let mut extra_blocking = Vec::new();
        let mut account_snapshot = None;
        let mut symbol_rules = None;

        let Some(credential) = active_credential.clone() else {
            return Ok(build_live_status(LiveStatusBuildInput {
                live_state,
                active_credential: None,
                account_snapshot: None,
                symbol_rules: None,
                reconciliation: self.load_reconciliation_cache().await?,
                intent_preview: None,
                recent_preflights: self
                    .repository
                    .list_live_preflights(self.options.recent_preflight_limit)
                    .await
                    .unwrap_or_default(),
                execution: self.load_execution_cache().await?,
                kill_switch: kill_switch.clone(),
                risk_profile: risk_profile.clone(),
                auto_executor: auto_executor.clone(),
                paper_position_open,
                extra_blocking: vec![LiveBlockingReason::NoActiveCredential],
                now_ms: now_ms(),
                options: &self.options,
            }));
        };

        if !credential.validation_status.is_valid() {
            extra_blocking.push(
                if credential.validation_status == LiveCredentialValidationStatus::Unknown {
                    LiveBlockingReason::ValidationMissing
                } else {
                    LiveBlockingReason::ValidationFailed
                },
            );
            return Ok(build_live_status(LiveStatusBuildInput {
                live_state,
                active_credential: Some(credential),
                account_snapshot: None,
                symbol_rules: None,
                reconciliation: self.load_reconciliation_cache().await?,
                intent_preview: None,
                recent_preflights: self
                    .repository
                    .list_live_preflights(self.options.recent_preflight_limit)
                    .await
                    .unwrap_or_default(),
                execution: self.load_execution_cache().await?,
                kill_switch: kill_switch.clone(),
                risk_profile: risk_profile.clone(),
                auto_executor: auto_executor.clone(),
                paper_position_open,
                extra_blocking,
                now_ms: now_ms(),
                options: &self.options,
            }));
        }

        if validation_is_stale(&credential, self.options.live_validation_ttl_ms, now_ms()) {
            extra_blocking.push(LiveBlockingReason::ValidationMissing);
        }

        if fetch_remote && extra_blocking.is_empty() {
            let secret = match self.secret_store.read(&credential.id).await {
                Ok(secret) => secret,
                Err(AppError::SecureStoreUnavailable(_error)) => {
                    extra_blocking.push(LiveBlockingReason::SecureStoreUnavailable);
                    return Ok(build_live_status(LiveStatusBuildInput {
                        live_state,
                        active_credential: Some(credential),
                        account_snapshot: None,
                        symbol_rules: None,
                        reconciliation: self.load_reconciliation_cache().await?,
                        intent_preview: None,
                        recent_preflights: self
                            .repository
                            .list_live_preflights(self.options.recent_preflight_limit)
                            .await
                            .unwrap_or_default(),
                        execution: self.load_execution_cache().await?,
                        kill_switch: kill_switch.clone(),
                        risk_profile: risk_profile.clone(),
                        auto_executor: auto_executor.clone(),
                        paper_position_open,
                        extra_blocking,
                        now_ms: now_ms(),
                        options: &self.options,
                    }));
                }
                Err(error) => return Err(error),
            };
            let active_symbol = self.state.lock().await.settings.active_symbol;
            match self
                .live_exchange
                .fetch_symbol_rules(live_state.environment, active_symbol)
                .await
            {
                Ok(rules) => symbol_rules = Some(rules),
                Err(error) => {
                    warn!(
                        event = "live_readiness_refreshed",
                        detail = %error,
                        "symbol rules fetch failed during live readiness refresh"
                    );
                    extra_blocking.push(LiveBlockingReason::SymbolRulesMissing);
                }
            }
            match self
                .live_exchange
                .fetch_account_snapshot(live_state.environment, &secret)
                .await
            {
                Ok(mut snapshot) => {
                    match self
                        .live_exchange
                        .fetch_account_mode(live_state.environment, &secret)
                        .await
                    {
                        Ok(mode) => {
                            snapshot.position_mode = mode.position_mode;
                            snapshot.multi_assets_margin = mode.multi_assets_margin;
                            snapshot.account_mode_checked_at = Some(mode.fetched_at);
                        }
                        Err(error) => {
                            warn!(
                                event = "live_readiness_refreshed",
                                detail = %error,
                                "dedicated account mode check failed during live readiness refresh"
                            );
                            extra_blocking.push(LiveBlockingReason::UnsupportedAccountMode);
                        }
                    }
                    account_snapshot = Some(snapshot);
                }
                Err(error) => {
                    warn!(
                        event = "live_readiness_refreshed",
                        detail = %error,
                        "account snapshot fetch failed during live readiness refresh"
                    );
                    extra_blocking.push(LiveBlockingReason::AccountSnapshotMissing);
                }
            }
        } else if !fetch_remote {
            let current = self.state.lock().await.live_status.clone();
            account_snapshot = current.account_snapshot;
            symbol_rules = current.symbol_rules;
        }

        if symbol_rules.is_none() {
            extra_blocking.push(LiveBlockingReason::SymbolRulesMissing);
        }
        if account_snapshot.is_none() {
            extra_blocking.push(LiveBlockingReason::AccountSnapshotMissing);
        }

        Ok(build_live_status(LiveStatusBuildInput {
            live_state,
            active_credential: Some(credential),
            account_snapshot,
            symbol_rules,
            reconciliation: self.load_reconciliation_cache().await?,
            intent_preview: None,
            recent_preflights: self
                .repository
                .list_live_preflights(self.options.recent_preflight_limit)
                .await
                .unwrap_or_default(),
            execution: self.load_execution_cache().await?,
            kill_switch,
            risk_profile,
            auto_executor,
            paper_position_open,
            extra_blocking,
            now_ms: now_ms(),
            options: &self.options,
        }))
    }

    async fn rebuild_state(&self, reason: &str) -> AppResult<BootstrapPayload> {
        self.rebuild_state_with_settings(reason, None).await
    }

    async fn rebuild_state_with_settings(
        &self,
        reason: &str,
        settings_override: Option<Settings>,
    ) -> AppResult<BootstrapPayload> {
        let mut settings = match settings_override {
            Some(settings) => settings,
            None => self.repository.load_settings().await?,
        };
        repair_settings(&mut settings);
        validate_settings(&settings)?;

        let logs = self
            .repository
            .recent_logs(self.options.recent_logs_limit)
            .await?;
        let live_status = self.load_live_status_snapshot().await?;
        let trade_history = self
            .repository
            .list_trades(self.options.recent_trades_limit)
            .await?;
        let wallet_rows = self.repository.load_wallets().await?;
        let wallets = if wallet_rows.is_empty() {
            reset_wallets(&settings.initial_wallet_balance_by_quote, now_ms())
        } else {
            wallet_rows
                .into_iter()
                .map(|wallet| (wallet.quote_asset, wallet))
                .collect::<BTreeMap<QuoteAsset, Wallet>>()
        };
        let position = self.repository.load_position().await?;
        let mut candles = self
            .repository
            .load_recent_klines(
                settings.active_symbol,
                settings.timeframe,
                self.options.history_limit.max(warmup_candles_required(
                    settings.aso_length,
                    settings.aso_mode,
                )),
            )
            .await?;

        let history_plan = build_history_load_plan(
            settings.active_symbol,
            settings.timeframe,
            settings.aso_length,
            settings.aso_mode,
            self.options.history_limit,
            now_ms(),
            &candles,
        );
        info!(
            event = "bootstrap_history_plan_built",
            reason,
            symbol = %history_plan.symbol,
            timeframe = %history_plan.timeframe,
            chart_seed_closed_candles = history_plan.chart_seed_closed_candles,
            warmup_closed_candles = history_plan.warmup_closed_candles,
            recompute_tail_closed_candles = history_plan.recompute_tail_closed_candles,
            requested_closed_candles = history_plan.requested_closed_candles,
            start_open_time = history_plan.window.start_open_time,
            end_open_time = history_plan.window.end_open_time,
            local_closed_candles = history_plan.local_closed_candles,
            local_contiguous = history_plan.local_contiguous,
            remote_backfill_required = history_plan.remote_backfill_required,
            "built history load plan"
        );

        let mut fetched_closed_candles = Vec::new();
        if history_plan.remote_backfill_required {
            info!(
                event = "bootstrap_history_fetch_started",
                reason,
                symbol = %history_plan.symbol,
                timeframe = %history_plan.timeframe,
                start_open_time = history_plan.window.start_open_time,
                end_open_time = history_plan.window.end_open_time,
                "starting explicit ranged history fetch"
            );
            let backfill = self
                .market_data
                .fetch_klines_range(history_plan.range_request())
                .await?;
            info!(
                event = "bootstrap_history_fetch_finished",
                reason,
                symbol = %history_plan.symbol,
                timeframe = %history_plan.timeframe,
                fetched_candles = backfill.len(),
                start_open_time = history_plan.window.start_open_time,
                end_open_time = history_plan.window.end_open_time,
                "finished explicit ranged history fetch"
            );
            fetched_closed_candles = backfill
                .iter()
                .filter(|candle| candle.closed)
                .cloned()
                .collect();
            candles = merge_candles(candles, backfill, history_plan.requested_closed_candles);
            info!(
                event = "bootstrap_history_merge_finished",
                reason,
                symbol = %history_plan.symbol,
                timeframe = %history_plan.timeframe,
                merged_candles = candles.len(),
                "finished merging fetched history window"
            );
        }

        let candles = select_closed_window(
            &candles,
            settings.active_symbol,
            settings.timeframe,
            history_plan.window,
        );
        validate_closed_window(settings.timeframe, history_plan.window, &candles).map_err(
            |error| {
                warn!(
                    event = "bootstrap_history_recompute_failed",
                    reason,
                    symbol = %history_plan.symbol,
                    timeframe = %history_plan.timeframe,
                    detail = %error,
                    "history rebuild failed deterministic contiguity validation"
                );
                AppError::History(error)
            },
        )?;
        info!(
            event = "bootstrap_history_recompute_finished",
            reason,
            symbol = %history_plan.symbol,
            timeframe = %history_plan.timeframe,
            contiguous_closed_candles = candles.len(),
            "validated deterministic history window for rebuild"
        );

        for candle in &fetched_closed_candles {
            self.repository.upsert_kline(candle).await?;
        }

        let aso_points = compute_aso_series(&candles, settings.aso_length, settings.aso_mode);
        let signals =
            derive_signal_history(settings.active_symbol, settings.timeframe, &aso_points);
        self.repository
            .sync_signals(settings.active_symbol, settings.timeframe, &signals)
            .await?;
        self.repository.save_settings(&settings).await?;

        let mut calculator = AsoCalculator::new(settings.aso_length, settings.aso_mode);
        for candle in candles.iter().filter(|candle| candle.closed).cloned() {
            let _ = calculator.push_closed(candle);
        }

        let mut engine = PaperEngine::with_state(wallets, position, trade_history);
        if let Some(last_price) = candles.last().map(|candle| candle.close) {
            mark_to_market(
                &mut engine.wallets,
                &mut engine.position,
                last_price,
                now_ms(),
            );
        }
        let performance = compute_performance(&engine.wallets, &engine.position, &engine.trades);
        let system_metrics = self.metrics.snapshot();

        let snapshot = {
            let mut state = self.state.lock().await;
            state.settings = settings.clone();
            state.runtime_status.active_symbol = settings.active_symbol;
            state.runtime_status.timeframe = settings.timeframe;
            state.runtime_status.activity = None;
            if state.connection_state.status_since.is_none() {
                state.connection_state.status_since = Some(now_ms());
            }
            state.connection_state.resync_required = false;
            state.connection_state.detail = Some(reason.to_string());
            state.candles = candles;
            state.aso_points = aso_points;
            state.signals = signals;
            state.engine = engine;
            state.performance = performance;
            state.live_status = live_status;
            state.system_metrics = system_metrics;
            state.logs = logs;
            state.calculator = calculator;
            state.initialized = true;
            state.resynced_live_events_remaining = 0;
            state.snapshot(&self.options)
        };

        self.publisher
            .publish(OutboundEvent::Snapshot(Box::new(snapshot.clone())));
        self.record_log("info", "bootstrap", format!("state rebuilt: {reason}"))
            .await?;
        Ok(snapshot)
    }

    async fn run_runtime_loop(self: Arc<Self>, mut stop_rx: oneshot::Receiver<()>) {
        let mut reconnect_attempts = 0_u64;
        loop {
            match stop_rx.try_recv() {
                Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => break,
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
            }

            let (symbol, timeframe) = {
                let state = self.state.lock().await;
                (state.settings.active_symbol, state.settings.timeframe)
            };
            let reconnecting_detail = if reconnect_attempts == 0 {
                "opening Binance kline stream".to_string()
            } else {
                format!("reconnecting Binance kline stream (attempt {reconnect_attempts})")
            };
            self.update_connection_state(
                ConnectionStatus::Reconnecting,
                reconnect_attempts,
                false,
                reconnecting_detail,
            )
            .await;

            let subscribe_result = self.market_data.subscribe_klines(symbol, timeframe).await;
            let mut stream = match subscribe_result {
                Ok(stream) => stream,
                Err(error) => {
                    reconnect_attempts += 1;
                    self.handle_disconnect(
                        symbol,
                        timeframe,
                        reconnect_attempts,
                        format!("subscribe failed: {error}"),
                    )
                    .await;
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            let first_item = tokio::select! {
                _ = &mut stop_rx => {
                    return;
                }
                next_item = stream.next() => next_item,
            };
            let first_event = match first_item {
                Some(Ok(event)) => event,
                Some(Err(error)) => {
                    reconnect_attempts += 1;
                    self.handle_disconnect(
                        symbol,
                        timeframe,
                        reconnect_attempts,
                        format!("stream open failed: {error}"),
                    )
                    .await;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                None => {
                    reconnect_attempts += 1;
                    self.handle_disconnect(
                        symbol,
                        timeframe,
                        reconnect_attempts,
                        "stream ended before first event".to_string(),
                    )
                    .await;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            let had_reconnect = reconnect_attempts > 0;
            match self
                .recover_after_reconnect(symbol, timeframe, reconnect_attempts, &first_event)
                .await
            {
                Ok(RecoveryDecision::NotNeeded) => {
                    reconnect_attempts = 0;
                    if had_reconnect {
                        self.update_connection_state(
                            ConnectionStatus::Connected,
                            reconnect_attempts,
                            false,
                            "reconnected without candle gap".to_string(),
                        )
                        .await;
                    }
                }
                Ok(RecoveryDecision::Recovered { recovered_closed }) => {
                    reconnect_attempts = 0;
                    self.update_connection_state(
                        ConnectionStatus::Resynced,
                        reconnect_attempts,
                        false,
                        format!("reconciled {recovered_closed} closed candle(s)"),
                    )
                    .await;
                    let mut state = self.state.lock().await;
                    state.resynced_live_events_remaining = 1;
                }
                Ok(RecoveryDecision::HardResync { reason }) => {
                    reconnect_attempts += 1;
                    self.trigger_hard_resync(symbol, timeframe, reconnect_attempts, reason)
                        .await;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                Err(error) => {
                    reconnect_attempts += 1;
                    self.trigger_hard_resync(
                        symbol,
                        timeframe,
                        reconnect_attempts,
                        format!("recovery execution failed: {error}"),
                    )
                    .await;
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }

            if let Err(error) = self
                .process_market_event(first_event, MarketEventOrigin::Live)
                .await
            {
                reconnect_attempts += 1;
                self.trigger_hard_resync(
                    symbol,
                    timeframe,
                    reconnect_attempts,
                    format!("failed to process live market event: {error}"),
                )
                .await;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            let mut metrics_tick = tokio::time::interval(Duration::from_secs(5));
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        return;
                    }
                    _ = metrics_tick.tick() => {
                        let metrics = self.metrics.snapshot();
                        let mut state = self.state.lock().await;
                        state.system_metrics = metrics.clone();
                        drop(state);
                        self.publisher.publish(OutboundEvent::SystemMetrics(metrics));
                    }
                    next_item = stream.next() => {
                        match next_item {
                            Some(Ok(event)) => {
                                if let Err(error) = self.process_market_event(event, MarketEventOrigin::Live).await {
                                    reconnect_attempts += 1;
                                    self.trigger_hard_resync(
                                        symbol,
                                        timeframe,
                                        reconnect_attempts,
                                        format!("failed to process live market event: {error}"),
                                    )
                                    .await;
                                    break;
                                }
                            }
                            Some(Err(error)) => {
                                reconnect_attempts += 1;
                                self.handle_disconnect(
                                    symbol,
                                    timeframe,
                                    reconnect_attempts,
                                    error.to_string(),
                                )
                                .await;
                                break;
                            }
                            None => {
                                reconnect_attempts += 1;
                                self.handle_disconnect(
                                    symbol,
                                    timeframe,
                                    reconnect_attempts,
                                    "market stream ended".to_string(),
                                )
                                .await;
                                break;
                            }
                        }
                    }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn build_recovery_plan(
        &self,
        symbol: relxen_domain::Symbol,
        timeframe: relxen_domain::Timeframe,
        reconnect_attempts: u64,
        first_event: &MarketStreamEvent,
    ) -> AppResult<Result<Option<RecoveryPlan>, String>> {
        if reconnect_attempts == 0 {
            return Ok(Ok(None));
        }

        let (last_seen, required_context_closed, available_context_closed) = {
            let state = self.state.lock().await;
            (
                state.candles.last().cloned(),
                warmup_candles_required(state.settings.aso_length, state.settings.aso_mode)
                    .saturating_sub(1),
                state.candles.iter().filter(|candle| candle.closed).count(),
            )
        };
        let last_persisted = self
            .repository
            .load_recent_klines(symbol, timeframe, 1)
            .await?
            .into_iter()
            .last()
            .or_else(|| last_seen.clone().filter(|candle| candle.closed));
        let Some(last_persisted_closed) = last_persisted else {
            let reason = "missing persisted candle anchor during reconnect".to_string();
            warn!(
                event = "recovery_failed",
                symbol = %symbol,
                timeframe = %timeframe,
                reconnect_attempts,
                detail = %reason,
                "reconnect recovery could not identify a durable closed-candle anchor"
            );
            return Ok(Err(reason));
        };

        let first_stream_open_time = timeframe.align_open_time(first_event.candle.open_time);
        let expected_live_open_time = match last_seen.as_ref() {
            Some(candle) if candle.closed => timeframe.next_open_time(candle.open_time),
            Some(candle) => timeframe.align_open_time(candle.open_time),
            None => timeframe.next_open_time(last_persisted_closed.open_time),
        };
        if first_stream_open_time <= expected_live_open_time {
            return Ok(Ok(None));
        }

        if available_context_closed < required_context_closed {
            let reason = format!(
                "insufficient indicator context before reconnect gap: have {}, need {}",
                available_context_closed, required_context_closed
            );
            warn!(
                event = "recovery_failed",
                symbol = %symbol,
                timeframe = %timeframe,
                reconnect_attempts,
                detail = %reason,
                "reconnect recovery could not prove indicator context"
            );
            return Ok(Err(reason));
        }

        let fetch_start_open_time = timeframe.next_open_time(last_persisted_closed.open_time);
        let fetch_end_open_time = first_stream_open_time - timeframe.duration_ms();
        let gap_closed_candles =
            timeframe.count_open_times_between(fetch_start_open_time, fetch_end_open_time);

        if gap_closed_candles > self.options.recovery_limit {
            let reason = format!(
                "gap requires {gap_closed_candles} closed candles but recovery limit is {}",
                self.options.recovery_limit
            );
            warn!(
                event = "recovery_failed",
                symbol = %symbol,
                timeframe = %timeframe,
                reconnect_attempts,
                detail = %reason,
                "reconnect recovery window exceeded configured bound"
            );
            return Ok(Err(reason));
        }

        let plan = RecoveryPlan {
            fetch_request: KlineRangeRequest {
                symbol,
                timeframe,
                start_open_time: fetch_start_open_time,
                end_open_time: fetch_end_open_time,
            },
            last_persisted_closed_open_time: last_persisted_closed.open_time,
            first_stream_open_time,
            gap_closed_candles,
            required_context_closed,
            available_context_closed,
        };
        info!(
            event = "recovery_plan_built",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            last_persisted_closed_open_time = plan.last_persisted_closed_open_time,
            first_stream_open_time = plan.first_stream_open_time,
            fetch_start_open_time = plan.fetch_request.start_open_time,
            fetch_end_open_time = plan.fetch_request.end_open_time,
            gap_closed_candles = plan.gap_closed_candles,
            required_context_closed = plan.required_context_closed,
            available_context_closed = plan.available_context_closed,
            "built deterministic reconnect recovery plan"
        );
        Ok(Ok(Some(plan)))
    }

    async fn recover_after_reconnect(
        &self,
        symbol: relxen_domain::Symbol,
        timeframe: relxen_domain::Timeframe,
        reconnect_attempts: u64,
        first_event: &MarketStreamEvent,
    ) -> AppResult<RecoveryDecision> {
        let plan = match self
            .build_recovery_plan(symbol, timeframe, reconnect_attempts, first_event)
            .await?
        {
            Ok(Some(plan)) => plan,
            Ok(None) => return Ok(RecoveryDecision::NotNeeded),
            Err(reason) => return Ok(RecoveryDecision::HardResync { reason }),
        };

        self.update_connection_state(
            ConnectionStatus::Stale,
            reconnect_attempts,
            false,
            format!(
                "recovering {} closed candle(s) through {}",
                plan.gap_closed_candles, plan.fetch_request.end_open_time
            ),
        )
        .await;
        info!(
            event = "recovery_fetch_started",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            start_open_time = plan.fetch_request.start_open_time,
            end_open_time = plan.fetch_request.end_open_time,
            gap_closed_candles = plan.gap_closed_candles,
            "starting explicit ranged reconnect recovery"
        );

        let recovered = match self
            .market_data
            .fetch_klines_range(plan.fetch_request)
            .await
        {
            Ok(candles) => candles,
            Err(error) => {
                let reason = format!("explicit recovery request failed: {error}");
                warn!(
                    event = "recovery_failed",
                    symbol = %symbol,
                    timeframe = %timeframe,
                    reconnect_attempts,
                    detail = %reason,
                    "explicit ranged reconnect recovery failed"
                );
                return Ok(RecoveryDecision::HardResync { reason });
            }
        };
        info!(
            event = "recovery_fetch_finished",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            fetched_candles = recovered.len(),
            start_open_time = plan.fetch_request.start_open_time,
            end_open_time = plan.fetch_request.end_open_time,
            "completed explicit ranged reconnect fetch"
        );

        let mut recovered_closed: Vec<Candle> = recovered
            .into_iter()
            .filter(|candle| {
                candle.closed
                    && candle.open_time >= plan.fetch_request.start_open_time
                    && candle.open_time <= plan.fetch_request.end_open_time
            })
            .collect();
        recovered_closed.sort_by_key(|candle| candle.open_time);
        recovered_closed.dedup_by_key(|candle| candle.open_time);

        if recovered_closed.len() != plan.gap_closed_candles {
            let reason = format!(
                "recovery window returned {} candles but {} were required",
                recovered_closed.len(),
                plan.gap_closed_candles
            );
            warn!(
                event = "recovery_failed",
                symbol = %symbol,
                timeframe = %timeframe,
                reconnect_attempts,
                detail = %reason,
                "explicit ranged reconnect recovery was incomplete"
            );
            return Ok(RecoveryDecision::HardResync { reason });
        }

        let mut expected_open_time = plan.fetch_request.start_open_time;
        for candle in &recovered_closed {
            if candle.open_time != expected_open_time {
                let reason = format!(
                    "recovery window does not bridge gap at open_time {expected_open_time}"
                );
                warn!(
                    event = "recovery_failed",
                    symbol = %symbol,
                    timeframe = %timeframe,
                    reconnect_attempts,
                    detail = %reason,
                    "explicit ranged reconnect recovery was not contiguous"
                );
                return Ok(RecoveryDecision::HardResync { reason });
            }
            expected_open_time = timeframe.next_open_time(expected_open_time);
        }

        for candle in recovered_closed.iter().cloned() {
            self.process_market_event(
                MarketStreamEvent {
                    candle,
                    closed: true,
                },
                MarketEventOrigin::Recovery,
            )
            .await?;
        }

        info!(
            event = "recovery_merge_finished",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            merged_candles = recovered_closed.len(),
            last_persisted_closed_open_time = plan.last_persisted_closed_open_time,
            "merged recovered candles into runtime state and persistence"
        );
        info!(
            event = "recovery_recompute_finished",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            replayed_candles = recovered_closed.len(),
            first_stream_open_time = plan.first_stream_open_time,
            "replayed recovered candles through ASO, signal, and paper engine"
        );
        let _ = self
            .record_log(
                "info",
                "recovery",
                format!(
                    "recovered {} closed candle(s) after reconnect",
                    recovered_closed.len()
                ),
            )
            .await;
        Ok(RecoveryDecision::Recovered {
            recovered_closed: recovered_closed.len(),
        })
    }

    async fn maybe_auto_execute_signal(
        &self,
        signal: SignalEvent,
        reference_price: f64,
    ) -> AppResult<()> {
        let mut auto = self.repository.load_live_auto_executor().await?;
        if auto.state != LiveAutoExecutorStateKind::Running {
            return Ok(());
        }
        let live_state = self.repository.load_live_state().await?;
        if live_state.environment != LiveEnvironment::Testnet {
            auto.state = LiveAutoExecutorStateKind::Blocked;
            auto.blocking_reasons = vec![LiveBlockingReason::MainnetAutoBlocked];
            auto.last_message = Some("Auto execution is TESTNET-only.".to_string());
            auto.updated_at = now_ms();
            self.repository.save_live_auto_executor(&auto).await?;
            self.publisher
                .publish(OutboundEvent::LiveAutoStateUpdated(auto));
            let _ = self.refresh_live_status_from_repository().await;
            return Ok(());
        }

        let settings = self.state.lock().await.settings.clone();
        let lock_key = format!(
            "{}:{}:{}:{}:{:?}",
            live_state.environment, signal.symbol, signal.timeframe, signal.open_time, signal.side
        );
        if self
            .repository
            .get_live_intent_lock(&lock_key)
            .await?
            .is_some()
        {
            auto.last_signal_id = Some(signal.id.clone());
            auto.last_signal_open_time = Some(signal.open_time);
            auto.last_message = Some("Duplicate closed-candle signal suppressed.".to_string());
            auto.blocking_reasons = vec![LiveBlockingReason::DuplicateSignalSuppressed];
            auto.updated_at = now_ms();
            self.repository.save_live_auto_executor(&auto).await?;
            self.publisher
                .publish(OutboundEvent::LiveAutoStateUpdated(auto));
            let _ = self.refresh_live_status_from_repository().await;
            info!(
                event = "auto_signal_blocked",
                signal_id = %signal.id,
                reason = %LiveBlockingReason::DuplicateSignalSuppressed.as_str(),
                "duplicate live auto signal suppressed"
            );
            return Ok(());
        }

        let now = now_ms();
        let mut lock = LiveIntentLock {
            key: lock_key,
            environment: live_state.environment,
            symbol: signal.symbol,
            timeframe: signal.timeframe,
            signal_id: signal.id.clone(),
            signal_open_time: signal.open_time,
            signal_side: signal.side,
            intent_hash: None,
            order_id: None,
            status: LiveIntentLockStatus::Created,
            block_reason: None,
            created_at: now,
            updated_at: now,
        };
        self.repository.upsert_live_intent_lock(&lock).await?;

        let status = self.refresh_live_status_from_repository().await?;
        let Some(rules) = status.symbol_rules.clone() else {
            lock.status = LiveIntentLockStatus::Blocked;
            lock.block_reason = Some(LiveBlockingReason::SymbolRulesMissing);
            lock.updated_at = now_ms();
            self.repository.upsert_live_intent_lock(&lock).await?;
            return Ok(());
        };
        let Some(shadow) = status.reconciliation.shadow.clone() else {
            lock.status = LiveIntentLockStatus::Blocked;
            lock.block_reason = Some(LiveBlockingReason::ShadowStateAmbiguous);
            lock.updated_at = now_ms();
            self.repository.upsert_live_intent_lock(&lock).await?;
            return Ok(());
        };
        let preview = build_live_order_preview(LiveIntentInput {
            environment: live_state.environment,
            symbol: settings.active_symbol,
            settings,
            rules,
            shadow,
            latest_signal: Some(signal.clone()),
            order_type: auto.order_type,
            reference_price: Decimal::from_str(&reference_price.to_string())
                .unwrap_or(Decimal::ZERO),
            limit_price: None,
            now_ms: now_ms(),
        });
        {
            let mut state = self.state.lock().await;
            state.live_status.intent_preview = Some(preview.clone());
            state.live_status.updated_at = now_ms();
        }
        self.publisher
            .publish(OutboundEvent::LiveIntentPreviewUpdated(Box::new(
                preview.clone(),
            )));
        let Some(intent) = preview.intent.as_ref() else {
            lock.status = LiveIntentLockStatus::Blocked;
            lock.block_reason = preview.blocking_reasons.first().copied();
            lock.updated_at = now_ms();
            self.repository.upsert_live_intent_lock(&lock).await?;
            auto.last_message = Some("Auto signal blocked during intent build.".to_string());
            auto.blocking_reasons = preview.blocking_reasons.clone();
            auto.updated_at = now_ms();
            self.repository.save_live_auto_executor(&auto).await?;
            self.publisher
                .publish(OutboundEvent::LiveAutoStateUpdated(auto));
            let _ = self.refresh_live_status_from_repository().await;
            return Ok(());
        };
        lock.intent_hash = Some(intent.intent_hash.clone());
        let result = self
            .execute_live_current_preview(LiveExecutionRequest {
                intent_id: Some(intent.id.clone()),
                confirm_testnet: true,
                confirm_mainnet_canary: false,
                confirmation_text: None,
            })
            .await?;
        if let Some(order) = result.order.as_ref() {
            lock.order_id = Some(order.id.clone());
        }
        lock.status = if result.accepted {
            LiveIntentLockStatus::Submitted
        } else {
            LiveIntentLockStatus::Blocked
        };
        lock.block_reason = result.blocking_reason;
        lock.updated_at = now_ms();
        self.repository.upsert_live_intent_lock(&lock).await?;
        auto.last_signal_id = Some(signal.id.clone());
        auto.last_signal_open_time = Some(signal.open_time);
        auto.last_intent_hash = Some(intent.intent_hash.clone());
        auto.last_order_id = result.order.as_ref().map(|order| order.id.clone());
        auto.last_message = Some(result.message);
        auto.blocking_reasons = result.blocking_reason.into_iter().collect();
        auto.updated_at = now_ms();
        self.repository.save_live_auto_executor(&auto).await?;
        self.publisher
            .publish(OutboundEvent::LiveAutoStateUpdated(auto));
        let _ = self.refresh_live_status_from_repository().await;
        info!(
            event = if result.accepted { "auto_signal_submitted" } else { "auto_signal_blocked" },
            signal_id = %signal.id,
            "closed-candle signal processed by TESTNET auto executor"
        );
        Ok(())
    }

    async fn process_market_event(
        &self,
        event: MarketStreamEvent,
        origin: MarketEventOrigin,
    ) -> AppResult<()> {
        let mut persist_candle = None;
        let mut persist_signal = None;
        let mut persist_wallets: Option<Vec<Wallet>> = None;
        let mut persist_position: Option<Option<Position>> = None;
        let mut new_trades: VecDeque<Trade> = VecDeque::new();
        let mut publish_events = Vec::new();

        {
            let mut state = self.state.lock().await;
            if matches!(origin, MarketEventOrigin::Live) {
                match state.connection_state.status {
                    ConnectionStatus::Resynced if state.resynced_live_events_remaining > 0 => {
                        state.resynced_live_events_remaining -= 1;
                    }
                    ConnectionStatus::Resynced => {
                        state.connection_state.status = ConnectionStatus::Connected;
                        state.connection_state.status_since = Some(now_ms());
                        state.connection_state.detail = Some("stream healthy".to_string());
                    }
                    _ => {
                        if state.connection_state.status != ConnectionStatus::Connected {
                            state.connection_state.status_since = Some(now_ms());
                        }
                        state.connection_state.status = ConnectionStatus::Connected;
                    }
                }
            }
            state.connection_state.last_message_time = Some(now_ms());
            state.connection_state.resync_required = false;

            let already_closed_processed = state
                .candles
                .iter()
                .any(|candle| candle.open_time == event.candle.open_time && candle.closed);

            upsert_recent_candle(
                &mut state.candles,
                event.candle.clone(),
                self.options.history_limit,
            );

            if event.closed {
                persist_candle = Some(event.candle.clone());
                if !already_closed_processed {
                    let point = state.calculator.push_closed(event.candle.clone());
                    push_recent(
                        &mut state.aso_points,
                        point.clone(),
                        self.options.history_limit,
                    );
                    publish_events.push(OutboundEvent::CandleClosed(event.candle.clone()));
                    publish_events.push(OutboundEvent::AsoUpdated(point.clone()));

                    if state.aso_points.len() >= 2 {
                        let previous = &state.aso_points[state.aso_points.len() - 2];
                        let current = &state.aso_points[state.aso_points.len() - 1];
                        if let Some(signal) = signal_from_points(
                            state.settings.active_symbol,
                            state.settings.timeframe,
                            previous,
                            current,
                        ) {
                            persist_signal = Some(signal.clone());
                            push_recent(
                                &mut state.signals,
                                signal.clone(),
                                self.options.recent_signals_limit * 4,
                            );
                            publish_events.push(OutboundEvent::SignalEmitted(signal.clone()));
                            let previous_trade_len = state.engine.trades.len();
                            let settings = state.settings.clone();
                            if let Err(error) = state.engine.apply_signal(
                                &settings,
                                &signal,
                                event.candle.close,
                                now_ms(),
                            ) {
                                warn!("paper engine rejected signal: {error}");
                            }
                            for trade in
                                state.engine.trades.iter().skip(previous_trade_len).cloned()
                            {
                                publish_events.push(OutboundEvent::TradeAppended(trade.clone()));
                                new_trades.push_back(trade);
                            }
                        }
                    }
                }
            } else if now_ms() - state.last_partial_publish_ms >= 250 {
                state.last_partial_publish_ms = now_ms();
                publish_events.push(OutboundEvent::CandlePartial(event.candle.clone()));
            }

            {
                let engine = &mut state.engine;
                mark_to_market(
                    &mut engine.wallets,
                    &mut engine.position,
                    event.candle.close,
                    now_ms(),
                );
            }
            state.performance = compute_performance(
                &state.engine.wallets,
                &state.engine.position,
                &state.engine.trades,
            );
            if event.closed || persist_signal.is_some() || !new_trades.is_empty() {
                persist_wallets = Some(state.engine.wallets.values().cloned().collect());
                persist_position = Some(state.engine.position.clone());
            }
            publish_events.push(OutboundEvent::PositionUpdated(
                state.engine.position.clone(),
            ));
            publish_events.push(OutboundEvent::WalletUpdated(
                state.engine.wallets.values().cloned().collect(),
            ));
            publish_events.push(OutboundEvent::PerformanceUpdated(state.performance.clone()));
            if matches!(origin, MarketEventOrigin::Live) {
                publish_events.push(OutboundEvent::ConnectionChanged(
                    state.connection_state.clone(),
                ));
            }
        }

        if let Some(candle) = persist_candle.as_ref() {
            self.repository.upsert_kline(candle).await?;
        }
        if let Some(signal) = persist_signal.as_ref() {
            self.repository.append_signal(signal).await?;
            let _ = self
                .maybe_auto_execute_signal(signal.clone(), event.candle.close)
                .await;
        }
        while let Some(trade) = new_trades.pop_front() {
            self.repository.append_trade(&trade).await?;
        }
        if let Some(wallets) = persist_wallets.as_ref() {
            self.repository.save_wallets(wallets).await?;
        }
        if let Some(position) = persist_position.as_ref() {
            self.repository.save_position(position.as_ref()).await?;
        }

        for event in publish_events {
            if let OutboundEvent::TradeAppended(trade) = &event {
                info!(
                    event = "trade_event_emitted",
                    trade_id = %trade.id,
                    symbol = %trade.symbol,
                    action = ?trade.action,
                    source = ?trade.source,
                    "publishing trade websocket event"
                );
            }
            self.publisher.publish(event);
        }
        Ok(())
    }

    async fn handle_disconnect(
        &self,
        symbol: relxen_domain::Symbol,
        timeframe: relxen_domain::Timeframe,
        reconnect_attempts: u64,
        message: String,
    ) {
        warn!(
            event = "disconnect_detected",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            detail = %message,
            "market stream interrupted"
        );
        self.update_connection_state(
            ConnectionStatus::Reconnecting,
            reconnect_attempts,
            false,
            format!("stream interrupted: {message}"),
        )
        .await;
        {
            let mut state = self.state.lock().await;
            state.runtime_status.last_error = Some(message.clone());
            state.resynced_live_events_remaining = 0;
        }
        let _ = self
            .record_log("warn", "stream", format!("disconnect: {message}"))
            .await;
    }

    async fn trigger_hard_resync(
        &self,
        symbol: relxen_domain::Symbol,
        timeframe: relxen_domain::Timeframe,
        reconnect_attempts: u64,
        reason: String,
    ) {
        warn!(
            event = "recovery_failed",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            detail = %reason,
            "deterministic reconnect recovery failed"
        );
        {
            let mut state = self.state.lock().await;
            state.connection_state.status = ConnectionStatus::Stale;
            state.connection_state.status_since = Some(now_ms());
            state.connection_state.reconnect_attempts = reconnect_attempts;
            state.connection_state.resync_required = true;
            state.connection_state.detail = Some(reason.clone());
            state.runtime_status.last_error = Some(reason.clone());
            state.resynced_live_events_remaining = 0;
            self.publisher.publish(OutboundEvent::ConnectionChanged(
                state.connection_state.clone(),
            ));
            self.publisher
                .publish(OutboundEvent::RuntimeChanged(state.runtime_status.clone()));
        }
        warn!(
            event = "resync_required_emitted",
            symbol = %symbol,
            timeframe = %timeframe,
            reconnect_attempts,
            detail = %reason,
            "hard resync required before live deltas can continue"
        );
        self.publisher.publish(OutboundEvent::ResyncRequired {
            reason: reason.clone(),
        });
        let _ = self
            .record_log("warn", "recovery", format!("resync required: {reason}"))
            .await;
        self.set_runtime_activity(Some(relxen_domain::RuntimeActivity::HistorySync))
            .await;
        let _ = self.rebuild_state("hard resync recovery").await;
    }

    async fn update_connection_state(
        &self,
        status: ConnectionStatus,
        reconnect_attempts: u64,
        resync_required: bool,
        detail: String,
    ) {
        let connection = {
            let mut state = self.state.lock().await;
            let status_changed = state.connection_state.status != status;
            state.connection_state.status = status;
            if status_changed || state.connection_state.status_since.is_none() {
                state.connection_state.status_since = Some(now_ms());
            }
            state.connection_state.reconnect_attempts = reconnect_attempts;
            state.connection_state.resync_required = resync_required;
            state.connection_state.detail = Some(detail);
            if !matches!(status, ConnectionStatus::Resynced) {
                state.resynced_live_events_remaining = 0;
            }
            state.connection_state.clone()
        };
        self.publisher
            .publish(OutboundEvent::ConnectionChanged(connection));
    }

    async fn set_runtime_activity(&self, activity: Option<relxen_domain::RuntimeActivity>) {
        let status = {
            let mut state = self.state.lock().await;
            if state.runtime_status.activity == activity {
                return;
            }
            state.runtime_status.activity = activity;
            state.runtime_status.clone()
        };
        self.publisher
            .publish(OutboundEvent::RuntimeChanged(status));
    }

    async fn record_log(&self, level: &str, target: &str, message: String) -> AppResult<()> {
        let event = LogEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: now_ms(),
            level: level.to_string(),
            target: target.to_string(),
            message,
        };

        {
            let mut state = self.state.lock().await;
            push_recent(
                &mut state.logs,
                event.clone(),
                self.options.recent_logs_limit * 4,
            );
        }

        self.repository.append_log(&event).await?;
        self.publisher.publish(OutboundEvent::LogAppended(event));
        Ok(())
    }
}

fn blocked_execution_result(
    reason: LiveBlockingReason,
    message: &str,
    created_at: i64,
) -> LiveExecutionResult {
    LiveExecutionResult {
        accepted: false,
        order: None,
        blocking_reason: Some(reason),
        message: message.to_string(),
        created_at,
    }
}

fn blocked_cancel_result(
    reason: LiveBlockingReason,
    message: &str,
    created_at: i64,
) -> LiveCancelResult {
    LiveCancelResult {
        accepted: false,
        order: None,
        blocking_reason: Some(reason),
        message: message.to_string(),
        created_at,
    }
}

fn blocked_flatten_result(
    reason: LiveBlockingReason,
    message: &str,
    created_at: i64,
) -> LiveFlattenResult {
    LiveFlattenResult {
        accepted: false,
        canceled_orders: Vec::new(),
        flatten_order: None,
        blocking_reason: Some(reason),
        message: message.to_string(),
        created_at,
    }
}

fn mainnet_confirmation_phrase(intent: &relxen_domain::LiveOrderIntent) -> String {
    match (intent.order_type, intent.price.as_deref()) {
        (LiveOrderType::Limit, Some(price)) => format!(
            "SUBMIT MAINNET {} LIMIT {} {} @ {}",
            intent.side.as_binance(),
            intent.symbol,
            intent.quantity,
            price
        ),
        _ => format!(
            "SUBMIT MAINNET {} MARKET {} {}",
            intent.side.as_binance(),
            intent.symbol,
            intent.quantity
        ),
    }
}

fn build_mainnet_canary_status(
    environment: LiveEnvironment,
    enabled_by_server: bool,
    risk_profile: &LiveRiskProfile,
    execution: &LiveExecutionSnapshot,
    intent_preview: Option<&LiveOrderPreview>,
    updated_at: i64,
) -> relxen_domain::LiveMainnetCanaryStatus {
    let mut blocking_reasons = Vec::new();
    if environment != LiveEnvironment::Mainnet {
        blocking_reasons.push(LiveBlockingReason::MainnetExecutionBlocked);
    }
    if !enabled_by_server {
        blocking_reasons.push(LiveBlockingReason::MainnetCanaryDisabled);
    }
    if !risk_profile.configured {
        blocking_reasons.push(LiveBlockingReason::MainnetCanaryRiskProfileMissing);
    }
    for reason in &execution.blocking_reasons {
        if !blocking_reasons.contains(reason) {
            blocking_reasons.push(*reason);
        }
    }
    let required_confirmation = intent_preview
        .and_then(|preview| preview.intent.as_ref())
        .filter(|intent| intent.environment == LiveEnvironment::Mainnet)
        .map(mainnet_confirmation_phrase);
    let canary_ready = environment == LiveEnvironment::Mainnet
        && enabled_by_server
        && risk_profile.configured
        && execution.can_submit
        && required_confirmation.is_some();
    relxen_domain::LiveMainnetCanaryStatus {
        enabled_by_server,
        risk_profile_configured: risk_profile.configured,
        canary_ready,
        manual_execution_enabled: canary_ready,
        required_confirmation,
        blocking_reasons,
        updated_at,
    }
}

fn intent_exceeds_risk_limits(
    intent: &relxen_domain::LiveOrderIntent,
    risk_profile: &LiveRiskProfile,
) -> bool {
    let notional = Decimal::from_str(&intent.sizing.estimated_notional).unwrap_or(Decimal::ZERO);
    let max_notional =
        Decimal::from_str(&risk_profile.limits.max_notional_per_order).unwrap_or(Decimal::ZERO);
    let leverage = Decimal::from_str(&intent.sizing.leverage).unwrap_or(Decimal::ZERO);
    let max_leverage =
        Decimal::from_str(&risk_profile.limits.max_leverage).unwrap_or(Decimal::ZERO);
    (max_notional > Decimal::ZERO && notional > max_notional)
        || (max_leverage > Decimal::ZERO && leverage > max_leverage)
}

fn client_order_id(prefix: &str) -> String {
    let raw = Uuid::new_v4().simple().to_string();
    format!("{prefix}_{}", &raw[..24])
}

fn merge_exchange_order(mut local: LiveOrderRecord, exchange: LiveOrderRecord) -> LiveOrderRecord {
    local.status = exchange.status;
    local.exchange_order_id = exchange.exchange_order_id.or(local.exchange_order_id);
    local.quantity = if exchange.quantity != "0" {
        exchange.quantity
    } else {
        local.quantity
    };
    local.price = exchange.price.or(local.price);
    local.executed_qty = exchange.executed_qty;
    local.avg_price = exchange.avg_price.or(local.avg_price);
    local.reduce_only = exchange.reduce_only || local.reduce_only;
    local.time_in_force = exchange.time_in_force.or(local.time_in_force);
    local.self_trade_prevention_mode = exchange.self_trade_prevention_mode;
    local.price_match = exchange.price_match;
    local.expire_reason = exchange.expire_reason;
    local.updated_at = exchange.updated_at.max(now_ms());
    local.last_error = None;
    local
}

fn merge_exchange_ack(mut local: LiveOrderRecord, exchange: LiveOrderRecord) -> LiveOrderRecord {
    local.status = LiveOrderStatus::Accepted;
    local.exchange_order_id = exchange.exchange_order_id.or(local.exchange_order_id);
    local.response_type = Some("ACK".to_string());
    local.self_trade_prevention_mode = exchange.self_trade_prevention_mode;
    local.price_match = exchange.price_match;
    local.expire_reason = exchange.expire_reason;
    local.updated_at = exchange.updated_at.max(now_ms());
    local.last_error = None;
    local
}

fn live_order_status_from_exchange_status(status: &str) -> LiveOrderStatus {
    match status {
        "NEW" => LiveOrderStatus::Working,
        "PARTIALLY_FILLED" => LiveOrderStatus::PartiallyFilled,
        "FILLED" => LiveOrderStatus::Filled,
        "CANCELED" => LiveOrderStatus::Canceled,
        "REJECTED" => LiveOrderStatus::Rejected,
        "EXPIRED" => LiveOrderStatus::Expired,
        "EXPIRED_IN_MATCH" => LiveOrderStatus::ExpiredInMatch,
        _ => LiveOrderStatus::UnknownNeedsRepair,
    }
}

fn fill_from_shadow_order(
    record: &LiveOrderRecord,
    order: &LiveShadowOrder,
) -> Option<LiveFillRecord> {
    let quantity = order.last_filled_qty.as_deref()?;
    let price = order.last_filled_price.as_deref()?;
    if Decimal::from_str(quantity).ok()? <= Decimal::ZERO {
        return None;
    }
    Some(LiveFillRecord {
        id: order
            .trade_id
            .as_ref()
            .map(|trade_id| format!("trade_{}_{}", order.order_id, trade_id))
            .unwrap_or_else(|| {
                format!(
                    "fill_{}_{}_{}",
                    record.client_order_id, order.last_update_time, quantity
                )
            }),
        order_id: Some(record.id.clone()),
        client_order_id: Some(record.client_order_id.clone()),
        exchange_order_id: Some(order.order_id.clone()),
        symbol: order.symbol,
        side: order.side,
        quantity: quantity.to_string(),
        price: price.to_string(),
        commission: order.commission.clone(),
        commission_asset: order.commission_asset.clone(),
        realized_pnl: None,
        trade_id: order.trade_id.clone(),
        event_time: order.last_update_time,
        created_at: now_ms(),
    })
}

fn decimal_to_exchange_string(value: Decimal) -> String {
    value
        .normalize()
        .to_string()
        .trim_end_matches(".0")
        .to_string()
}

fn repair_settings(settings: &mut Settings) {
    settings
        .available_symbols
        .retain(|symbol| ALLOWED_SYMBOLS.contains(symbol));
    if settings.available_symbols.is_empty() {
        settings.available_symbols = ALLOWED_SYMBOLS.to_vec();
    }
    if !settings.available_symbols.contains(&settings.active_symbol) {
        settings.active_symbol = settings.available_symbols[0];
    }
}

fn settings_requires_rebuild(current: &Settings, candidate: &Settings) -> bool {
    current.active_symbol != candidate.active_symbol
        || current.timeframe != candidate.timeframe
        || current.aso_length != candidate.aso_length
        || current.aso_mode != candidate.aso_mode
}

fn validate_secret_input(api_key: &str, api_secret: &str) -> AppResult<()> {
    if api_key.trim().is_empty() {
        return Err(AppError::Validation("api_key cannot be empty".to_string()));
    }
    if api_secret.trim().is_empty() {
        return Err(AppError::Validation(
            "api_secret cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn mask_api_key(api_key: &str) -> String {
    let trimmed = api_key.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let first: String = chars.iter().take(4).collect();
    let last: String = chars[chars.len() - 4..].iter().collect();
    format!("{first}…{last}")
}

fn validation_is_stale(
    credential: &LiveCredentialSummary,
    validation_ttl_ms: i64,
    now_ms: i64,
) -> bool {
    credential
        .last_validated_at
        .map(|validated_at| now_ms.saturating_sub(validated_at) > validation_ttl_ms)
        .unwrap_or(true)
}

fn live_execution_unavailable() -> LiveExecutionAvailability {
    LiveExecutionAvailability {
        can_execute_live: false,
        reason: LiveBlockingReason::MainnetExecutionBlocked,
        message: "MAINNET execution is blocked; TESTNET execution requires readiness gates."
            .to_string(),
    }
}

fn mask_listen_key(listen_key: &str) -> String {
    if listen_key.len() <= 10 {
        return "listenKey…".to_string();
    }
    format!(
        "{}…{}",
        &listen_key[..4],
        &listen_key[listen_key.len().saturating_sub(4)..]
    )
}

fn account_snapshot_to_shadow(snapshot: LiveAccountSnapshot, updated_at: i64) -> LiveAccountShadow {
    LiveAccountShadow {
        environment: snapshot.environment,
        balances: snapshot
            .assets
            .into_iter()
            .map(|asset| LiveShadowBalance {
                asset: asset.asset,
                wallet_balance: asset.wallet_balance.to_string(),
                cross_wallet_balance: Some(asset.available_balance.to_string()),
                balance_change: None,
                updated_at,
            })
            .collect(),
        positions: snapshot
            .positions
            .into_iter()
            .map(|position| LiveShadowPosition {
                symbol: position.symbol,
                position_side: position.position_side,
                position_amt: position.position_amt.to_string(),
                entry_price: position.entry_price.to_string(),
                unrealized_pnl: position.unrealized_pnl.to_string(),
                margin_type: None,
                isolated_wallet: None,
                updated_at,
            })
            .collect(),
        open_orders: Vec::new(),
        can_trade: snapshot.can_trade,
        multi_assets_margin: snapshot.multi_assets_margin,
        position_mode: snapshot
            .position_mode
            .or_else(|| Some("one_way".to_string())),
        last_event_time: None,
        last_rest_sync_at: Some(snapshot.fetched_at),
        updated_at,
        ambiguous: false,
        divergence_reasons: Vec::new(),
    }
}

fn refresh_account_snapshot_from_shadow(
    baseline: Option<LiveAccountSnapshot>,
    shadow: Option<&LiveAccountShadow>,
) -> Option<LiveAccountSnapshot> {
    let shadow = match shadow {
        Some(shadow) => shadow,
        None => return baseline,
    };
    let mut snapshot = baseline.unwrap_or(LiveAccountSnapshot {
        environment: shadow.environment,
        can_trade: shadow.can_trade,
        multi_assets_margin: shadow.multi_assets_margin,
        position_mode: shadow.position_mode.clone(),
        account_mode_checked_at: None,
        total_wallet_balance: 0.0,
        total_margin_balance: 0.0,
        available_balance: 0.0,
        assets: Vec::new(),
        positions: Vec::new(),
        fetched_at: shadow.updated_at,
    });
    snapshot.environment = shadow.environment;
    snapshot.can_trade = shadow.can_trade;
    snapshot.multi_assets_margin = shadow.multi_assets_margin;
    snapshot.position_mode = shadow.position_mode.clone();
    snapshot.fetched_at = shadow
        .last_rest_sync_at
        .or(shadow.last_event_time)
        .unwrap_or(shadow.updated_at);
    snapshot.assets = shadow
        .balances
        .iter()
        .map(|balance| LiveAssetBalance {
            asset: balance.asset.clone(),
            wallet_balance: parse_shadow_number(&balance.wallet_balance),
            available_balance: parse_shadow_number(
                balance
                    .cross_wallet_balance
                    .as_deref()
                    .unwrap_or(&balance.wallet_balance),
            ),
            unrealized_pnl: 0.0,
        })
        .collect();
    snapshot.positions = shadow
        .positions
        .iter()
        .map(|position| LivePositionSnapshot {
            symbol: position.symbol,
            position_side: position.position_side.clone(),
            position_amt: parse_shadow_number(&position.position_amt),
            entry_price: parse_shadow_number(&position.entry_price),
            mark_price: None,
            unrealized_pnl: parse_shadow_number(&position.unrealized_pnl),
            leverage: snapshot
                .positions
                .iter()
                .find(|existing| existing.symbol == position.symbol)
                .and_then(|existing| existing.leverage),
        })
        .collect();
    Some(snapshot)
}

fn parse_shadow_number(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or_default()
}

struct LiveStatusBuildInput<'a> {
    live_state: LiveStateRecord,
    active_credential: Option<LiveCredentialSummary>,
    account_snapshot: Option<LiveAccountSnapshot>,
    symbol_rules: Option<LiveSymbolRules>,
    reconciliation: LiveReconciliationStatus,
    intent_preview: Option<LiveOrderPreview>,
    recent_preflights: Vec<LiveOrderPreflightResult>,
    execution: LiveExecutionSnapshot,
    kill_switch: LiveKillSwitchState,
    risk_profile: LiveRiskProfile,
    auto_executor: LiveAutoExecutorStatus,
    paper_position_open: bool,
    extra_blocking: Vec<LiveBlockingReason>,
    now_ms: i64,
    options: &'a ServiceOptions,
}

fn build_live_status(input: LiveStatusBuildInput<'_>) -> LiveStatusSnapshot {
    let LiveStatusBuildInput {
        live_state,
        active_credential,
        account_snapshot,
        symbol_rules,
        mut reconciliation,
        intent_preview,
        recent_preflights,
        execution,
        kill_switch,
        risk_profile,
        auto_executor,
        paper_position_open,
        extra_blocking,
        now_ms,
        options,
    } = input;

    let mut checks = Vec::new();
    let mut blocking_reasons = Vec::<LiveBlockingReason>::new();
    let mut warnings = Vec::<LiveWarning>::new();

    if live_state.environment == LiveEnvironment::Testnet {
        warnings.push(LiveWarning::TestnetEnvironment);
    }

    push_check(
        &mut checks,
        &mut blocking_reasons,
        "active_credential",
        active_credential.is_some(),
        LiveBlockingReason::NoActiveCredential,
        "Active live credential selected.",
        "No active live credential is selected.",
    );

    if let Some(credential) = active_credential.as_ref() {
        let validation_valid = credential.validation_status.is_valid()
            && !validation_is_stale(credential, options.live_validation_ttl_ms, now_ms);
        if credential.validation_status.is_valid() && !validation_valid {
            warnings.push(LiveWarning::ValidationStale);
        }
        push_check(
            &mut checks,
            &mut blocking_reasons,
            "credential_validation",
            validation_valid,
            if credential.validation_status == LiveCredentialValidationStatus::Unknown {
                LiveBlockingReason::ValidationMissing
            } else {
                LiveBlockingReason::ValidationFailed
            },
            "Credential validation is current.",
            "Credential validation is missing, stale, or failed.",
        );
    }

    push_check(
        &mut checks,
        &mut blocking_reasons,
        "symbol_rules",
        symbol_rules.is_some(),
        LiveBlockingReason::SymbolRulesMissing,
        "Active symbol rules are available.",
        "Active symbol rules are not available.",
    );
    if symbol_rules
        .as_ref()
        .map(|rules| now_ms.saturating_sub(rules.fetched_at) > options.live_snapshot_stale_ms)
        .unwrap_or(false)
    {
        warnings.push(LiveWarning::RulesSnapshotStale);
    }

    push_check(
        &mut checks,
        &mut blocking_reasons,
        "account_snapshot",
        account_snapshot.is_some(),
        LiveBlockingReason::AccountSnapshotMissing,
        "Account snapshot is available.",
        "Account snapshot is not available.",
    );
    if account_snapshot
        .as_ref()
        .map(|snapshot| now_ms.saturating_sub(snapshot.fetched_at) > options.live_snapshot_stale_ms)
        .unwrap_or(false)
    {
        warnings.push(LiveWarning::AccountSnapshotStale);
    }
    if account_snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .positions
                .iter()
                .any(|position| position.position_amt.abs() > f64::EPSILON)
        })
        .unwrap_or(false)
    {
        warnings.push(LiveWarning::OpenExchangePositionDetected);
    }

    if reconciliation.stream.state == LiveShadowStreamState::Degraded {
        blocking_reasons.push(LiveBlockingReason::ShadowStateAmbiguous);
    }
    if reconciliation.stream.state == LiveShadowStreamState::Running
        && reconciliation
            .stream
            .last_event_time
            .map(|last| now_ms.saturating_sub(last) > options.live_shadow_stale_ms)
            .unwrap_or(false)
    {
        warnings.push(LiveWarning::ShadowStreamStale);
        blocking_reasons.push(LiveBlockingReason::ShadowStreamDown);
        reconciliation.stream.stale = true;
    }
    for reason in &reconciliation.blocking_reasons {
        if !blocking_reasons.contains(reason) {
            blocking_reasons.push(*reason);
        }
    }

    push_check(
        &mut checks,
        &mut blocking_reasons,
        "paper_position_flat",
        !paper_position_open,
        LiveBlockingReason::PaperPositionOpen,
        "No open paper position conflicts with live arming.",
        "An open paper position blocks live arming.",
    );

    for reason in extra_blocking {
        if !blocking_reasons.contains(&reason) {
            blocking_reasons.push(reason);
        }
    }
    if kill_switch.engaged && !blocking_reasons.contains(&LiveBlockingReason::KillSwitchEngaged) {
        blocking_reasons.push(LiveBlockingReason::KillSwitchEngaged);
    }
    blocking_reasons.sort_by_key(|reason| reason.as_str());
    blocking_reasons.dedup();
    warnings.dedup();

    let ready = blocking_reasons.is_empty();
    let mut state = if active_credential.is_none() {
        LiveRuntimeState::CredentialsMissing
    } else if blocking_reasons.contains(&LiveBlockingReason::SecureStoreUnavailable) {
        LiveRuntimeState::SecureStoreUnavailable
    } else if blocking_reasons.contains(&LiveBlockingReason::ValidationFailed)
        || blocking_reasons.contains(&LiveBlockingReason::ValidationMissing)
    {
        LiveRuntimeState::ValidationFailed
    } else if blocking_reasons.contains(&LiveBlockingReason::SymbolRulesMissing) {
        LiveRuntimeState::RulesUnavailable
    } else if blocking_reasons.contains(&LiveBlockingReason::AccountSnapshotMissing) {
        LiveRuntimeState::AccountSnapshotUnavailable
    } else if ready && live_state.armed {
        LiveRuntimeState::ArmedReadOnly
    } else if ready {
        LiveRuntimeState::ReadyReadOnly
    } else {
        LiveRuntimeState::NotReady
    };
    if state == LiveRuntimeState::ReadyReadOnly || state == LiveRuntimeState::ArmedReadOnly {
        state = match reconciliation.state {
            LiveRuntimeState::ShadowRunning | LiveRuntimeState::PreflightReady => {
                reconciliation.state
            }
            LiveRuntimeState::ShadowDegraded | LiveRuntimeState::PreflightBlocked => {
                reconciliation.state
            }
            _ => state,
        };
    }
    if let Some(preview) = intent_preview.as_ref() {
        let preview_blocked = preview.intent.is_none() || !preview.blocking_reasons.is_empty();
        if preview_blocked
            && matches!(
                state,
                LiveRuntimeState::ReadyReadOnly
                    | LiveRuntimeState::ArmedReadOnly
                    | LiveRuntimeState::ShadowRunning
                    | LiveRuntimeState::PreflightReady
            )
        {
            state = LiveRuntimeState::PreflightBlocked;
        } else if !preview_blocked
            && matches!(
                state,
                LiveRuntimeState::ReadyReadOnly
                    | LiveRuntimeState::ArmedReadOnly
                    | LiveRuntimeState::ShadowRunning
            )
        {
            state = LiveRuntimeState::PreflightReady;
        }
    }

    let readiness = LiveReadinessSnapshot {
        state,
        environment: live_state.environment,
        active_credential: active_credential.clone(),
        checks,
        blocking_reasons: blocking_reasons.clone(),
        warnings: warnings.clone(),
        account_snapshot: account_snapshot.clone(),
        symbol_rules: symbol_rules.clone(),
        can_arm: ready,
        can_execute_live: false,
        refreshed_at: now_ms,
    };

    let (execution, execution_availability) = build_execution_status(ExecutionStatusInput {
        live_state: &live_state,
        readiness_ready: ready,
        readiness_blocking: &blocking_reasons,
        readiness_warnings: &warnings,
        reconciliation: &reconciliation,
        intent_preview: intent_preview.as_ref(),
        execution,
        kill_switch: &kill_switch,
        risk_profile: &risk_profile,
        auto_executor: &auto_executor,
        now_ms,
        options,
    });
    if execution.state == LiveExecutionState::TestnetExecutionReady {
        state = LiveRuntimeState::TestnetExecutionReady;
    } else if execution.state == LiveExecutionState::MainnetExecutionBlocked {
        state = LiveRuntimeState::MainnetExecutionBlocked;
    } else if execution.state == LiveExecutionState::MainnetCanaryReady {
        state = LiveRuntimeState::MainnetCanaryReady;
    } else if execution.state == LiveExecutionState::MainnetManualExecutionEnabled {
        state = LiveRuntimeState::MainnetManualExecutionEnabled;
    } else if execution.state == LiveExecutionState::TestnetAutoRunning {
        state = LiveRuntimeState::TestnetAutoRunning;
    } else if execution.state == LiveExecutionState::KillSwitchEngaged {
        state = LiveRuntimeState::KillSwitchEngaged;
    } else if execution.state == LiveExecutionState::ExecutionDegraded {
        state = LiveRuntimeState::ExecutionDegraded;
    }

    let mainnet_canary = build_mainnet_canary_status(
        live_state.environment,
        options.enable_mainnet_canary_execution,
        &risk_profile,
        &execution,
        intent_preview.as_ref(),
        now_ms,
    );

    LiveStatusSnapshot {
        feature_visible: true,
        mode_preference: live_state.mode_preference,
        environment: live_state.environment,
        state,
        armed: live_state.armed && ready,
        active_credential,
        readiness,
        reconciliation,
        account_snapshot,
        symbol_rules,
        intent_preview,
        recent_preflights,
        execution,
        execution_availability,
        kill_switch,
        risk_profile,
        auto_executor,
        mainnet_canary,
        updated_at: now_ms,
    }
}

struct ExecutionStatusInput<'a> {
    live_state: &'a LiveStateRecord,
    readiness_ready: bool,
    readiness_blocking: &'a [LiveBlockingReason],
    readiness_warnings: &'a [LiveWarning],
    reconciliation: &'a LiveReconciliationStatus,
    intent_preview: Option<&'a LiveOrderPreview>,
    execution: LiveExecutionSnapshot,
    kill_switch: &'a LiveKillSwitchState,
    risk_profile: &'a LiveRiskProfile,
    auto_executor: &'a LiveAutoExecutorStatus,
    now_ms: i64,
    options: &'a ServiceOptions,
}

fn build_execution_status(
    input: ExecutionStatusInput<'_>,
) -> (LiveExecutionSnapshot, LiveExecutionAvailability) {
    let ExecutionStatusInput {
        live_state,
        readiness_ready,
        readiness_blocking,
        readiness_warnings,
        reconciliation,
        intent_preview,
        mut execution,
        kill_switch,
        risk_profile,
        auto_executor,
        now_ms,
        options,
    } = input;
    let mut blocking = readiness_blocking.to_vec();
    let mut warnings = readiness_warnings.to_vec();

    if kill_switch.engaged {
        blocking.push(LiveBlockingReason::KillSwitchEngaged);
    }
    if live_state.environment == LiveEnvironment::Mainnet
        && !options.enable_mainnet_canary_execution
    {
        blocking.push(LiveBlockingReason::MainnetCanaryDisabled);
    }
    if live_state.environment == LiveEnvironment::Mainnet && !risk_profile.configured {
        blocking.push(LiveBlockingReason::MainnetCanaryRiskProfileMissing);
    }
    if !live_state.armed {
        blocking.push(LiveBlockingReason::RuntimeBusy);
    }

    match reconciliation.stream.state {
        LiveShadowStreamState::Running => {}
        LiveShadowStreamState::Degraded | LiveShadowStreamState::Expired => {
            blocking.push(LiveBlockingReason::ShadowStateAmbiguous);
        }
        _ => blocking.push(LiveBlockingReason::ShadowStreamDown),
    }
    if reconciliation.stream.stale {
        blocking.push(LiveBlockingReason::StaleShadowState);
        warnings.push(LiveWarning::ShadowStreamStale);
    }
    if let Some(last_event) = reconciliation.stream.last_event_time {
        if now_ms.saturating_sub(last_event) > options.live_shadow_stale_ms {
            blocking.push(LiveBlockingReason::StaleShadowState);
            warnings.push(LiveWarning::ShadowStreamStale);
        }
    }

    match reconciliation.shadow.as_ref() {
        Some(shadow) => {
            if shadow.ambiguous {
                blocking.push(LiveBlockingReason::ShadowStateAmbiguous);
            }
            if shadow.multi_assets_margin.unwrap_or(false)
                || shadow
                    .positions
                    .iter()
                    .any(|position| position.position_side != "BOTH")
            {
                blocking.push(LiveBlockingReason::UnsupportedAccountMode);
            }
        }
        None => blocking.push(LiveBlockingReason::AccountSnapshotMissing),
    }

    let active_order = execution
        .recent_orders
        .iter()
        .rev()
        .find(|order| order.status.is_open())
        .cloned();
    if active_order.is_some() {
        blocking.push(LiveBlockingReason::RuntimeBusy);
    }

    match intent_preview {
        Some(preview) => {
            blocking.extend(preview.blocking_reasons.iter().copied());
            if let Some(intent) = preview.intent.as_ref() {
                blocking.extend(intent.blocking_reasons.iter().copied());
                if intent.environment != live_state.environment {
                    blocking.push(LiveBlockingReason::PreviewMismatch);
                }
                if now_ms.saturating_sub(intent.built_at) > options.live_intent_ttl_ms {
                    blocking.push(LiveBlockingReason::PreviewMismatch);
                }
                if risk_profile.configured && intent_exceeds_risk_limits(intent, risk_profile) {
                    blocking.push(LiveBlockingReason::RiskLimitExceeded);
                }
                if !intent.can_execute_now && live_state.environment == LiveEnvironment::Testnet {
                    blocking.push(LiveBlockingReason::ExecutionNotImplemented);
                }
            } else {
                blocking.push(LiveBlockingReason::IntentUnavailable);
            }
        }
        None => blocking.push(LiveBlockingReason::IntentUnavailable),
    }

    blocking.sort_by_key(|reason| reason.as_str());
    blocking.dedup();
    warnings.dedup();

    if live_state.environment == LiveEnvironment::Testnet
        && auto_executor.state == LiveAutoExecutorStateKind::Running
        && !blocking.contains(&LiveBlockingReason::RuntimeBusy)
    {
        // Auto mode shares manual execution gates; status text distinguishes the operator mode.
    }

    let can_submit = readiness_ready && blocking.is_empty();
    let state = active_order
        .as_ref()
        .map(|order| match order.status {
            LiveOrderStatus::SubmitPending => LiveExecutionState::TestnetSubmitPending,
            LiveOrderStatus::Working | LiveOrderStatus::Accepted => {
                LiveExecutionState::TestnetOrderOpen
            }
            LiveOrderStatus::PartiallyFilled => LiveExecutionState::TestnetPartiallyFilled,
            LiveOrderStatus::Filled => LiveExecutionState::TestnetFilled,
            LiveOrderStatus::CancelPending => LiveExecutionState::TestnetCancelPending,
            LiveOrderStatus::Canceled
            | LiveOrderStatus::Rejected
            | LiveOrderStatus::Expired
            | LiveOrderStatus::ExpiredInMatch
            | LiveOrderStatus::UnknownNeedsRepair
            | LiveOrderStatus::LocalCreated => LiveExecutionState::ExecutionDegraded,
        })
        .unwrap_or_else(|| {
            if live_state.environment == LiveEnvironment::Mainnet {
                if can_submit && options.enable_mainnet_canary_execution && risk_profile.configured
                {
                    LiveExecutionState::MainnetCanaryReady
                } else {
                    LiveExecutionState::MainnetExecutionBlocked
                }
            } else if kill_switch.engaged {
                LiveExecutionState::KillSwitchEngaged
            } else if can_submit && auto_executor.state == LiveAutoExecutorStateKind::Running {
                LiveExecutionState::TestnetAutoRunning
            } else if can_submit {
                LiveExecutionState::TestnetExecutionReady
            } else if blocking.contains(&LiveBlockingReason::ShadowStateAmbiguous)
                || blocking.contains(&LiveBlockingReason::StaleShadowState)
            {
                LiveExecutionState::ExecutionDegraded
            } else {
                LiveExecutionState::ExecutionBlocked
            }
        });

    execution.environment = live_state.environment;
    execution.state = state;
    execution.can_submit = can_submit;
    execution.blocking_reasons = blocking.clone();
    execution.warnings = warnings;
    execution.active_order = active_order;
    execution.kill_switch_engaged = kill_switch.engaged;
    execution.mainnet_canary_enabled = options.enable_mainnet_canary_execution;
    execution.repair_recent_window_only = true;
    execution.updated_at = now_ms;

    let first_reason = blocking
        .first()
        .copied()
        .unwrap_or(LiveBlockingReason::ExecutionNotImplemented);
    let availability = LiveExecutionAvailability {
        can_execute_live: can_submit,
        reason: first_reason,
        message: if can_submit {
            if live_state.environment == LiveEnvironment::Mainnet {
                "MAINNET canary execution ready for the displayed preview.".to_string()
            } else {
                "TESTNET execution ready for the displayed preview.".to_string()
            }
        } else if live_state.environment == LiveEnvironment::Mainnet
            && !options.enable_mainnet_canary_execution
        {
            "MAINNET canary execution is disabled by server policy.".to_string()
        } else if live_state.environment == LiveEnvironment::Mainnet {
            format!(
                "MAINNET canary execution blocked: {}",
                first_reason.as_str()
            )
        } else {
            format!("TESTNET execution blocked: {}", first_reason.as_str())
        },
    };
    (execution, availability)
}

fn push_check(
    checks: &mut Vec<LiveGateCheck>,
    blocking_reasons: &mut Vec<LiveBlockingReason>,
    code: &str,
    passed: bool,
    blocking_reason: LiveBlockingReason,
    passed_message: &str,
    failed_message: &str,
) {
    checks.push(LiveGateCheck {
        code: code.to_string(),
        passed,
        message: if passed {
            passed_message.to_string()
        } else {
            failed_message.to_string()
        },
    });
    if !passed {
        blocking_reasons.push(blocking_reason);
    }
}

fn upsert_recent_candle(candles: &mut Vec<Candle>, candle: Candle, limit: usize) {
    if let Some(existing) = candles
        .iter_mut()
        .find(|item| item.open_time == candle.open_time)
    {
        *existing = candle;
    } else {
        candles.push(candle);
        candles.sort_by_key(|item| item.open_time);
    }

    if candles.len() > limit {
        let excess = candles.len() - limit;
        candles.drain(0..excess);
    }
}

fn push_recent<T>(items: &mut Vec<T>, item: T, limit: usize) {
    items.push(item);
    if items.len() > limit {
        let excess = items.len() - limit;
        items.drain(0..excess);
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
