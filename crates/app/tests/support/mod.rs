#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use relxen_app::{
    AppError, AppResult, EventPublisher, KlineRangeRequest, LiveExchangePort, MarketDataPort,
    MarketStream, MarketStreamEvent, MetricsPort, OutboundEvent, Repository, SecretStore,
};
use relxen_domain::{
    Candle, ConnectionStatus, LiveAccountModeStatus, LiveAccountShadow, LiveAccountSnapshot,
    LiveAssetBalance, LiveAutoExecutorStatus, LiveCredentialId, LiveCredentialMetadata,
    LiveCredentialSecret, LiveCredentialValidationResult, LiveCredentialValidationStatus,
    LiveEnvironment, LiveExecutionSnapshot, LiveFillRecord, LiveIntentLock, LiveKillSwitchState,
    LiveOrderPreflightResult, LiveOrderRecord, LiveOrderSide, LiveOrderStatus, LiveOrderType,
    LiveReconciliationStatus, LiveRiskProfile, LiveStateRecord, LiveSymbolFilterSummary,
    LiveSymbolRules, LiveUserDataEvent, LogEvent, Position, Settings, SignalEvent, Symbol,
    SystemMetrics, Timeframe, Trade, Wallet,
};

pub fn candle(index: i64) -> Candle {
    candle_at(Timeframe::M1, index)
}

pub fn candle_at(timeframe: Timeframe, index: i64) -> Candle {
    let open_time = index * timeframe.duration_ms();
    Candle {
        symbol: Symbol::BtcUsdt,
        timeframe,
        open_time,
        close_time: timeframe.close_time_for_open(open_time),
        open: 100.0 + index as f64,
        high: 102.0 + index as f64,
        low: 99.0 + index as f64,
        close: 101.0 + index as f64,
        volume: 1.0,
        closed: true,
    }
}

pub fn candle_with_bull(index: i64, bull: f64, closed: bool) -> Candle {
    candle_with_bull_at(Timeframe::M1, index, bull, closed)
}

pub fn candle_with_bull_at(timeframe: Timeframe, index: i64, bull: f64, closed: bool) -> Candle {
    candle_with_bull_at_open_time(
        Symbol::BtcUsdt,
        timeframe,
        index * timeframe.duration_ms(),
        bull,
        closed,
    )
}

pub fn candle_with_bull_at_open_time(
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

pub fn stream_event(candle: Candle, closed: bool) -> MarketStreamEvent {
    MarketStreamEvent { candle, closed }
}

pub fn latest_closed_open_time(timeframe: Timeframe) -> i64 {
    timeframe.align_open_time(relxen_app::now_ms() - timeframe.duration_ms())
}

pub fn recent_open_time(timeframe: Timeframe, offset_from_latest_closed: i64) -> i64 {
    latest_closed_open_time(timeframe) + offset_from_latest_closed * timeframe.duration_ms()
}

#[derive(Default)]
pub struct MockRepository {
    settings: Mutex<Settings>,
    klines: Mutex<Vec<Candle>>,
    signals: Mutex<Vec<SignalEvent>>,
    trades: Mutex<Vec<Trade>>,
    wallets: Mutex<Vec<Wallet>>,
    position: Mutex<Option<Position>>,
    logs: Mutex<Vec<LogEvent>>,
    live_credentials: Mutex<Vec<LiveCredentialMetadata>>,
    live_state: Mutex<Option<LiveStateRecord>>,
    live_reconciliation: Mutex<Option<LiveReconciliationStatus>>,
    live_shadow: Mutex<Option<LiveAccountShadow>>,
    live_preflights: Mutex<Vec<LiveOrderPreflightResult>>,
    live_execution: Mutex<Option<LiveExecutionSnapshot>>,
    live_kill_switch: Mutex<Option<LiveKillSwitchState>>,
    live_risk_profile: Mutex<Option<LiveRiskProfile>>,
    live_auto_executor: Mutex<Option<LiveAutoExecutorStatus>>,
    live_intent_locks: Mutex<Vec<LiveIntentLock>>,
    live_orders: Mutex<Vec<LiveOrderRecord>>,
    live_fills: Mutex<Vec<LiveFillRecord>>,
}

impl MockRepository {
    pub async fn seed_candles(&self, candles: &[Candle]) {
        for candle in candles {
            self.upsert_kline(candle).await.unwrap();
        }
    }

    pub async fn all_klines(&self) -> Vec<Candle> {
        let mut candles = self.klines.lock().await.clone();
        candles.sort_by_key(|candle| candle.open_time);
        candles
    }

    pub async fn klines_for(&self, symbol: Symbol, timeframe: Timeframe) -> Vec<Candle> {
        let mut candles: Vec<Candle> = self
            .klines
            .lock()
            .await
            .iter()
            .filter(|candle| candle.symbol == symbol && candle.timeframe == timeframe)
            .cloned()
            .collect();
        candles.sort_by_key(|candle| candle.open_time);
        candles
    }
}

#[async_trait]
impl Repository for MockRepository {
    async fn load_settings(&self) -> AppResult<Settings> {
        Ok(self.settings.lock().await.clone())
    }

    async fn save_settings(&self, settings: &Settings) -> AppResult<()> {
        *self.settings.lock().await = settings.clone();
        Ok(())
    }

    async fn load_recent_klines(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
        limit: usize,
    ) -> AppResult<Vec<Candle>> {
        let mut candles: Vec<Candle> = self
            .klines
            .lock()
            .await
            .iter()
            .filter(|candle| candle.symbol == symbol && candle.timeframe == timeframe)
            .cloned()
            .collect();
        candles.sort_by_key(|candle| candle.open_time);
        if candles.len() > limit {
            candles = candles.split_off(candles.len() - limit);
        }
        Ok(candles)
    }

    async fn upsert_kline(&self, candle: &Candle) -> AppResult<()> {
        let mut candles = self.klines.lock().await;
        if let Some(existing) = candles.iter_mut().find(|item| {
            item.symbol == candle.symbol
                && item.timeframe == candle.timeframe
                && item.open_time == candle.open_time
        }) {
            *existing = candle.clone();
        } else {
            candles.push(candle.clone());
        }
        Ok(())
    }

    async fn list_signals(&self, limit: usize) -> AppResult<Vec<SignalEvent>> {
        let mut signals = self.signals.lock().await.clone();
        if signals.len() > limit {
            signals = signals.split_off(signals.len() - limit);
        }
        Ok(signals)
    }

    async fn sync_signals(
        &self,
        _symbol: Symbol,
        _timeframe: Timeframe,
        signals: &[SignalEvent],
    ) -> AppResult<()> {
        *self.signals.lock().await = signals.to_vec();
        Ok(())
    }

    async fn append_signal(&self, signal: &SignalEvent) -> AppResult<()> {
        self.signals.lock().await.push(signal.clone());
        Ok(())
    }

    async fn list_trades(&self, limit: usize) -> AppResult<Vec<Trade>> {
        let mut trades = self.trades.lock().await.clone();
        if trades.len() > limit {
            trades = trades.split_off(trades.len() - limit);
        }
        Ok(trades)
    }

    async fn append_trade(&self, trade: &Trade) -> AppResult<()> {
        self.trades.lock().await.push(trade.clone());
        Ok(())
    }

    async fn clear_trades(&self) -> AppResult<()> {
        self.trades.lock().await.clear();
        Ok(())
    }

    async fn load_wallets(&self) -> AppResult<Vec<Wallet>> {
        Ok(self.wallets.lock().await.clone())
    }

    async fn save_wallets(&self, wallets: &[Wallet]) -> AppResult<()> {
        *self.wallets.lock().await = wallets.to_vec();
        Ok(())
    }

    async fn load_position(&self) -> AppResult<Option<Position>> {
        Ok(self.position.lock().await.clone())
    }

    async fn save_position(&self, position: Option<&Position>) -> AppResult<()> {
        *self.position.lock().await = position.cloned();
        Ok(())
    }

    async fn recent_logs(&self, limit: usize) -> AppResult<Vec<LogEvent>> {
        let mut logs = self.logs.lock().await.clone();
        if logs.len() > limit {
            logs = logs.split_off(logs.len() - limit);
        }
        Ok(logs)
    }

    async fn append_log(&self, log: &LogEvent) -> AppResult<()> {
        self.logs.lock().await.push(log.clone());
        Ok(())
    }

    async fn list_live_credentials(&self) -> AppResult<Vec<LiveCredentialMetadata>> {
        Ok(self.live_credentials.lock().await.clone())
    }

    async fn get_live_credential(
        &self,
        id: &LiveCredentialId,
    ) -> AppResult<Option<LiveCredentialMetadata>> {
        Ok(self
            .live_credentials
            .lock()
            .await
            .iter()
            .find(|credential| credential.id == *id)
            .cloned())
    }

    async fn active_live_credential(
        &self,
        environment: LiveEnvironment,
    ) -> AppResult<Option<LiveCredentialMetadata>> {
        Ok(self
            .live_credentials
            .lock()
            .await
            .iter()
            .find(|credential| credential.environment == environment && credential.is_active)
            .cloned())
    }

    async fn upsert_live_credential(&self, credential: &LiveCredentialMetadata) -> AppResult<()> {
        let mut credentials = self.live_credentials.lock().await;
        if let Some(existing) = credentials.iter_mut().find(|item| item.id == credential.id) {
            *existing = credential.clone();
        } else {
            credentials.push(credential.clone());
        }
        Ok(())
    }

    async fn delete_live_credential(&self, id: &LiveCredentialId) -> AppResult<()> {
        self.live_credentials
            .lock()
            .await
            .retain(|credential| credential.id != *id);
        Ok(())
    }

    async fn select_live_credential(
        &self,
        id: &LiveCredentialId,
        environment: LiveEnvironment,
    ) -> AppResult<()> {
        for credential in self.live_credentials.lock().await.iter_mut() {
            if credential.environment == environment {
                credential.is_active = credential.id == *id;
            }
        }
        Ok(())
    }

    async fn load_live_state(&self) -> AppResult<LiveStateRecord> {
        Ok(self.live_state.lock().await.clone().unwrap_or_default())
    }

    async fn save_live_state(&self, state: &LiveStateRecord) -> AppResult<()> {
        *self.live_state.lock().await = Some(state.clone());
        Ok(())
    }

    async fn load_live_reconciliation(&self) -> AppResult<Option<LiveReconciliationStatus>> {
        Ok(self.live_reconciliation.lock().await.clone())
    }

    async fn save_live_reconciliation(&self, status: &LiveReconciliationStatus) -> AppResult<()> {
        *self.live_reconciliation.lock().await = Some(status.clone());
        Ok(())
    }

    async fn load_live_shadow(&self) -> AppResult<Option<LiveAccountShadow>> {
        Ok(self.live_shadow.lock().await.clone())
    }

    async fn save_live_shadow(&self, shadow: &LiveAccountShadow) -> AppResult<()> {
        *self.live_shadow.lock().await = Some(shadow.clone());
        Ok(())
    }

    async fn list_live_preflights(&self, limit: usize) -> AppResult<Vec<LiveOrderPreflightResult>> {
        let mut preflights = self.live_preflights.lock().await.clone();
        if preflights.len() > limit {
            preflights = preflights.split_off(preflights.len() - limit);
        }
        Ok(preflights)
    }

    async fn append_live_preflight(&self, result: &LiveOrderPreflightResult) -> AppResult<()> {
        self.live_preflights.lock().await.push(result.clone());
        Ok(())
    }

    async fn load_live_execution(&self) -> AppResult<Option<LiveExecutionSnapshot>> {
        Ok(self.live_execution.lock().await.clone())
    }

    async fn save_live_execution(&self, execution: &LiveExecutionSnapshot) -> AppResult<()> {
        *self.live_execution.lock().await = Some(execution.clone());
        Ok(())
    }

    async fn load_live_kill_switch(&self) -> AppResult<LiveKillSwitchState> {
        Ok(self
            .live_kill_switch
            .lock()
            .await
            .clone()
            .unwrap_or_default())
    }

    async fn save_live_kill_switch(&self, state: &LiveKillSwitchState) -> AppResult<()> {
        *self.live_kill_switch.lock().await = Some(state.clone());
        Ok(())
    }

    async fn load_live_risk_profile(&self) -> AppResult<LiveRiskProfile> {
        Ok(self
            .live_risk_profile
            .lock()
            .await
            .clone()
            .unwrap_or_default())
    }

    async fn save_live_risk_profile(&self, profile: &LiveRiskProfile) -> AppResult<()> {
        *self.live_risk_profile.lock().await = Some(profile.clone());
        Ok(())
    }

    async fn load_live_auto_executor(&self) -> AppResult<LiveAutoExecutorStatus> {
        Ok(self
            .live_auto_executor
            .lock()
            .await
            .clone()
            .unwrap_or_default())
    }

    async fn save_live_auto_executor(&self, status: &LiveAutoExecutorStatus) -> AppResult<()> {
        *self.live_auto_executor.lock().await = Some(status.clone());
        Ok(())
    }

    async fn get_live_intent_lock(&self, key: &str) -> AppResult<Option<LiveIntentLock>> {
        Ok(self
            .live_intent_locks
            .lock()
            .await
            .iter()
            .find(|lock| lock.key == key)
            .cloned())
    }

    async fn upsert_live_intent_lock(&self, lock: &LiveIntentLock) -> AppResult<()> {
        let mut locks = self.live_intent_locks.lock().await;
        if let Some(existing) = locks.iter_mut().find(|item| item.key == lock.key) {
            *existing = lock.clone();
        } else {
            locks.push(lock.clone());
        }
        Ok(())
    }

    async fn list_live_orders(&self, limit: usize) -> AppResult<Vec<LiveOrderRecord>> {
        let mut orders = self.live_orders.lock().await.clone();
        orders.sort_by_key(|order| order.updated_at);
        if orders.len() > limit {
            orders = orders.split_off(orders.len() - limit);
        }
        Ok(orders)
    }

    async fn get_live_order(&self, order_ref: &str) -> AppResult<Option<LiveOrderRecord>> {
        Ok(self
            .live_orders
            .lock()
            .await
            .iter()
            .find(|order| {
                order.id == order_ref
                    || order.client_order_id == order_ref
                    || order.exchange_order_id.as_deref() == Some(order_ref)
            })
            .cloned())
    }

    async fn upsert_live_order(&self, order: &LiveOrderRecord) -> AppResult<()> {
        let mut orders = self.live_orders.lock().await;
        if let Some(existing) = orders.iter_mut().find(|item| item.id == order.id) {
            *existing = order.clone();
        } else {
            orders.push(order.clone());
        }
        Ok(())
    }

    async fn list_live_fills(&self, limit: usize) -> AppResult<Vec<LiveFillRecord>> {
        let mut fills = self.live_fills.lock().await.clone();
        fills.sort_by_key(|fill| fill.created_at);
        if fills.len() > limit {
            fills = fills.split_off(fills.len() - limit);
        }
        Ok(fills)
    }

    async fn append_live_fill(&self, fill: &LiveFillRecord) -> AppResult<()> {
        let mut fills = self.live_fills.lock().await;
        if let Some(existing) = fills.iter_mut().find(|item| item.id == fill.id) {
            *existing = fill.clone();
        } else {
            fills.push(fill.clone());
        }
        Ok(())
    }
}

pub struct FakeLiveExchange {
    pub validation_status: LiveCredentialValidationStatus,
    pub account: Option<LiveAccountSnapshot>,
    pub rules: Option<LiveSymbolRules>,
    pub user_events: Mutex<VecDeque<Result<LiveUserDataEvent, AppError>>>,
    pub user_trades: Mutex<Vec<LiveFillRecord>>,
    pub preflight_accept: bool,
    pub submitted_orders: Mutex<Vec<LiveOrderRecord>>,
    pub fail_submit: bool,
    pub fail_cancel: bool,
    pub listen_key_creates: AtomicUsize,
    pub listen_key_closes: AtomicUsize,
    pub stream_subscriptions: AtomicUsize,
}

#[derive(Default)]
pub struct TestSecretStore {
    secrets: Mutex<BTreeMap<String, LiveCredentialSecret>>,
    unavailable: bool,
}

impl TestSecretStore {
    pub fn unavailable() -> Self {
        Self {
            secrets: Mutex::new(BTreeMap::new()),
            unavailable: true,
        }
    }
}

#[async_trait]
impl SecretStore for TestSecretStore {
    async fn store(&self, id: &LiveCredentialId, secret: &LiveCredentialSecret) -> AppResult<()> {
        self.ensure_available().await?;
        self.secrets
            .lock()
            .await
            .insert(id.as_str().to_string(), secret.clone());
        Ok(())
    }

    async fn read(&self, id: &LiveCredentialId) -> AppResult<LiveCredentialSecret> {
        self.ensure_available().await?;
        self.secrets
            .lock()
            .await
            .get(id.as_str())
            .cloned()
            .ok_or_else(|| AppError::NotFound("test secret missing".to_string()))
    }

    async fn delete(&self, id: &LiveCredentialId) -> AppResult<()> {
        self.ensure_available().await?;
        self.secrets.lock().await.remove(id.as_str());
        Ok(())
    }

    async fn ensure_available(&self) -> AppResult<()> {
        if self.unavailable {
            Err(AppError::SecureStoreUnavailable(
                "test secret store unavailable".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

impl Default for FakeLiveExchange {
    fn default() -> Self {
        Self {
            validation_status: LiveCredentialValidationStatus::Valid,
            account: Some(fake_account_snapshot(LiveEnvironment::Testnet)),
            rules: Some(fake_symbol_rules(LiveEnvironment::Testnet, Symbol::BtcUsdt)),
            user_events: Mutex::new(VecDeque::new()),
            user_trades: Mutex::new(Vec::new()),
            preflight_accept: true,
            submitted_orders: Mutex::new(Vec::new()),
            fail_submit: false,
            fail_cancel: false,
            listen_key_creates: AtomicUsize::new(0),
            listen_key_closes: AtomicUsize::new(0),
            stream_subscriptions: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LiveExchangePort for FakeLiveExchange {
    async fn validate_credentials(
        &self,
        environment: LiveEnvironment,
        credential_id: &LiveCredentialId,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveCredentialValidationResult> {
        Ok(LiveCredentialValidationResult {
            credential_id: credential_id.clone(),
            environment,
            status: self.validation_status,
            validated_at: relxen_app::now_ms(),
            message: if self.validation_status.is_valid() {
                None
            } else {
                Some("fake validation failure".to_string())
            },
        })
    }

    async fn fetch_account_snapshot(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountSnapshot> {
        self.account
            .clone()
            .map(|mut account| {
                account.environment = environment;
                account
            })
            .ok_or_else(|| AppError::Exchange("account snapshot unavailable".to_string()))
    }

    async fn fetch_account_mode(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountModeStatus> {
        let account = self
            .account
            .clone()
            .ok_or_else(|| AppError::Exchange("account snapshot unavailable".to_string()))?;
        Ok(LiveAccountModeStatus {
            environment,
            position_mode: account.position_mode,
            multi_assets_margin: account.multi_assets_margin,
            fetched_at: relxen_app::now_ms(),
        })
    }

    async fn fetch_symbol_rules(
        &self,
        environment: LiveEnvironment,
        symbol: Symbol,
    ) -> AppResult<LiveSymbolRules> {
        self.rules
            .clone()
            .map(|mut rules| {
                rules.environment = environment;
                rules.symbol = symbol;
                rules
            })
            .ok_or_else(|| AppError::Exchange("symbol rules unavailable".to_string()))
    }

    async fn create_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<String> {
        let count = self.listen_key_creates.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(format!("fake-listen-key-{count}"))
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
        self.listen_key_closes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn subscribe_user_data(
        &self,
        _environment: LiveEnvironment,
        _listen_key: &str,
    ) -> AppResult<relxen_app::LiveUserDataStream> {
        self.stream_subscriptions.fetch_add(1, Ordering::SeqCst);
        let events: Vec<_> = self.user_events.lock().await.drain(..).collect();
        Ok(Box::pin(stream::iter(events).chain(stream::pending())))
    }

    async fn preflight_order_test(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _payload: &BTreeMap<String, String>,
    ) -> AppResult<()> {
        if self.preflight_accept {
            Ok(())
        } else {
            Err(AppError::Exchange("exchange_error: rejected".to_string()))
        }
    }

    async fn submit_order(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        payload: &BTreeMap<String, String>,
    ) -> AppResult<LiveOrderRecord> {
        if self.fail_submit {
            return Err(AppError::Exchange(
                "network_error: submit failed".to_string(),
            ));
        }
        let now = relxen_app::now_ms();
        let symbol = payload
            .get("symbol")
            .and_then(|value| value.parse().ok())
            .unwrap_or(Symbol::BtcUsdt);
        let side = if payload.get("side").map(String::as_str) == Some("SELL") {
            LiveOrderSide::Sell
        } else {
            LiveOrderSide::Buy
        };
        let order_type = if payload.get("type").map(String::as_str) == Some("LIMIT") {
            LiveOrderType::Limit
        } else {
            LiveOrderType::Market
        };
        let client_order_id = payload
            .get("newClientOrderId")
            .cloned()
            .unwrap_or_else(|| "fake-client-order".to_string());
        let order = LiveOrderRecord {
            id: client_order_id.clone(),
            credential_id: None,
            environment,
            symbol,
            side,
            order_type,
            status: LiveOrderStatus::Working,
            client_order_id,
            exchange_order_id: Some("42".to_string()),
            quantity: payload
                .get("quantity")
                .cloned()
                .unwrap_or_else(|| "0.001".to_string()),
            price: payload.get("price").cloned(),
            executed_qty: "0".to_string(),
            avg_price: None,
            reduce_only: payload
                .get("reduceOnly")
                .map(|value| value == "true")
                .unwrap_or(false),
            time_in_force: payload.get("timeInForce").cloned(),
            intent_id: None,
            intent_hash: None,
            source_signal_id: None,
            source_open_time: None,
            reason: "fake_exchange".to_string(),
            payload: payload.clone(),
            response_type: Some("ACK".to_string()),
            self_trade_prevention_mode: None,
            price_match: None,
            expire_reason: None,
            last_error: None,
            submitted_at: now,
            updated_at: now,
        };
        self.submitted_orders.lock().await.push(order.clone());
        Ok(order)
    }

    async fn cancel_order(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        symbol: Symbol,
        orig_client_order_id: Option<&str>,
        _order_id: Option<&str>,
    ) -> AppResult<LiveOrderRecord> {
        if self.fail_cancel {
            return Err(AppError::Exchange("cancel_failed".to_string()));
        }
        let mut orders = self.submitted_orders.lock().await;
        let index = orders
            .iter()
            .position(|order| orig_client_order_id == Some(order.client_order_id.as_str()))
            .unwrap_or(0);
        let mut order = orders
            .get(index)
            .cloned()
            .unwrap_or_else(|| LiveOrderRecord {
                id: orig_client_order_id.unwrap_or("fake-cancel").to_string(),
                credential_id: None,
                environment,
                symbol,
                side: LiveOrderSide::Buy,
                order_type: LiveOrderType::Market,
                status: LiveOrderStatus::Working,
                client_order_id: orig_client_order_id.unwrap_or("fake-cancel").to_string(),
                exchange_order_id: Some("42".to_string()),
                quantity: "0.001".to_string(),
                price: None,
                executed_qty: "0".to_string(),
                avg_price: None,
                reduce_only: false,
                time_in_force: None,
                intent_id: None,
                intent_hash: None,
                source_signal_id: None,
                source_open_time: None,
                reason: "fake_exchange".to_string(),
                payload: BTreeMap::new(),
                response_type: Some("ACK".to_string()),
                self_trade_prevention_mode: None,
                price_match: None,
                expire_reason: None,
                last_error: None,
                submitted_at: relxen_app::now_ms(),
                updated_at: relxen_app::now_ms(),
            });
        order.status = LiveOrderStatus::Canceled;
        order.updated_at = relxen_app::now_ms();
        if let Some(existing) = orders.get_mut(index) {
            *existing = order.clone();
        }
        Ok(order)
    }

    async fn query_order(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        symbol: Symbol,
        orig_client_order_id: Option<&str>,
        _order_id: Option<&str>,
    ) -> AppResult<Option<LiveOrderRecord>> {
        Ok(self
            .submitted_orders
            .lock()
            .await
            .iter()
            .find(|order| {
                order.environment == environment
                    && order.symbol == symbol
                    && orig_client_order_id == Some(order.client_order_id.as_str())
            })
            .cloned())
    }

    async fn list_open_orders(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        symbol: Symbol,
    ) -> AppResult<Vec<LiveOrderRecord>> {
        Ok(self
            .submitted_orders
            .lock()
            .await
            .iter()
            .filter(|order| order.symbol == symbol && order.status.is_open())
            .cloned()
            .collect())
    }

    async fn list_user_trades(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _symbol: Symbol,
        limit: usize,
    ) -> AppResult<Vec<LiveFillRecord>> {
        let mut fills = self.user_trades.lock().await.clone();
        fills.sort_by_key(|fill| fill.created_at);
        if fills.len() > limit {
            fills = fills.split_off(fills.len() - limit);
        }
        Ok(fills)
    }
}

pub fn fake_account_snapshot(environment: LiveEnvironment) -> LiveAccountSnapshot {
    LiveAccountSnapshot {
        environment,
        can_trade: true,
        multi_assets_margin: Some(false),
        position_mode: Some("one_way".to_string()),
        account_mode_checked_at: Some(relxen_app::now_ms()),
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
    }
}

pub fn fake_symbol_rules(environment: LiveEnvironment, symbol: Symbol) -> LiveSymbolRules {
    LiveSymbolRules {
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
    }
}

pub struct SequenceMarket {
    pub range_called: AtomicUsize,
    pub subscribe_called: AtomicUsize,
    range_fetches: Mutex<VecDeque<Vec<Candle>>>,
    range_requests: Mutex<Vec<KlineRangeRequest>>,
    subscriptions: Mutex<VecDeque<Vec<Result<MarketStreamEvent, AppError>>>>,
}

impl SequenceMarket {
    pub fn new(
        subscriptions: Vec<Vec<Result<MarketStreamEvent, AppError>>>,
        range_fetches: Vec<Vec<Candle>>,
    ) -> Self {
        Self {
            range_called: AtomicUsize::new(0),
            subscribe_called: AtomicUsize::new(0),
            range_fetches: Mutex::new(range_fetches.into()),
            range_requests: Mutex::new(Vec::new()),
            subscriptions: Mutex::new(subscriptions.into()),
        }
    }

    pub async fn range_requests(&self) -> Vec<KlineRangeRequest> {
        self.range_requests.lock().await.clone()
    }
}

#[async_trait]
impl MarketDataPort for SequenceMarket {
    async fn fetch_klines_range(&self, request: KlineRangeRequest) -> AppResult<Vec<Candle>> {
        self.range_called.fetch_add(1, Ordering::SeqCst);
        self.range_requests.lock().await.push(request);
        Ok(self
            .range_fetches
            .lock()
            .await
            .pop_front()
            .unwrap_or_default())
    }

    async fn subscribe_klines(
        &self,
        _symbol: Symbol,
        _timeframe: Timeframe,
    ) -> AppResult<MarketStream> {
        self.subscribe_called.fetch_add(1, Ordering::SeqCst);
        let events = self
            .subscriptions
            .lock()
            .await
            .pop_front()
            .unwrap_or_default();
        let stream = stream::iter(events).chain(stream::pending());
        Ok(Box::pin(stream))
    }
}

pub struct StaticMetrics;

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

#[derive(Default)]
pub struct CapturingPublisher {
    events: StdMutex<Vec<OutboundEvent>>,
}

impl CapturingPublisher {
    pub fn events(&self) -> Vec<OutboundEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn has_resync_required(&self) -> bool {
        self.events()
            .iter()
            .any(|event| matches!(event, OutboundEvent::ResyncRequired { .. }))
    }

    pub fn connection_statuses(&self) -> Vec<ConnectionStatus> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                OutboundEvent::ConnectionChanged(state) => Some(state.status),
                _ => None,
            })
            .collect()
    }
}

impl EventPublisher for CapturingPublisher {
    fn publish(&self, event: OutboundEvent) {
        self.events.lock().unwrap().push(event);
    }
}

pub async fn wait_until(label: &str, duration: Duration, mut predicate: impl FnMut() -> bool) {
    timeout(duration, async move {
        loop {
            if predicate() {
                return;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out while waiting for {label}"));
}

pub fn stream_error(message: &str) -> Result<MarketStreamEvent, AppError> {
    Err(AppError::Other(anyhow!(message.to_string())))
}

pub fn assert_contains_status(statuses: &[ConnectionStatus], expected: ConnectionStatus) {
    assert!(
        statuses.contains(&expected),
        "missing connection status {expected:?} in {statuses:?}"
    );
}

pub fn arc<T>(value: T) -> Arc<T> {
    Arc::new(value)
}
