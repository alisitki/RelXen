use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use futures::StreamExt;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

use relxen_app::{
    now_ms, AppError, AppMetadata, AppResult, AppService, BootstrapPayload, KlineRangeRequest,
    LiveDependencies, LiveExchangePort, MarketDataPort, MarketStream, MarketStreamEvent,
    NoopPublisher, Repository, ServiceOptions,
};
use relxen_domain::{
    Candle, ConnectionStatus, LiveAccountModeStatus, LiveAccountShadow, LiveAccountSnapshot,
    LiveAssetBalance, LiveAutoExecutorStatus, LiveCredentialId, LiveCredentialMetadata,
    LiveCredentialSecret, LiveCredentialValidationResult, LiveCredentialValidationStatus,
    LiveEnvironment, LiveExecutionRequest, LiveExecutionSnapshot, LiveFillRecord, LiveIntentLock,
    LiveKillSwitchState, LiveOrderPreflightResult, LiveOrderRecord, LiveOrderSide, LiveOrderStatus,
    LiveOrderType, LiveReconciliationStatus, LiveRiskProfile, LiveStateRecord,
    LiveSymbolFilterSummary, LiveSymbolRules, LiveUserDataEvent, LogEvent,
    MainnetAutoDecisionEvent, MainnetAutoLessonReport, MainnetAutoRiskBudget, MainnetAutoStatus,
    MainnetAutoWatchdogEvent, Position, Settings, SignalEvent, Symbol, SystemMetrics, Timeframe,
    Trade, Wallet,
};
use relxen_infra::{EventBus, MemorySecretStore};
use relxen_server::{build_router, RouterState};

fn candle_with_bull_at_open_time(open_time: i64, bull: f64, closed: bool) -> Candle {
    let bull = bull.clamp(0.0, 100.0);
    let (open, close) = if bull <= 50.0 {
        (100.0, bull * 2.0)
    } else {
        (0.0, (bull - 50.0) * 2.0)
    };
    Candle {
        symbol: Symbol::BtcUsdt,
        timeframe: Timeframe::M1,
        open_time,
        close_time: Timeframe::M1.close_time_for_open(open_time),
        open,
        high: 100.0,
        low: 0.0,
        close,
        volume: 1.0,
        closed,
    }
}

fn stream_error(message: &str) -> Result<MarketStreamEvent, AppError> {
    Err(AppError::Other(anyhow::anyhow!(message.to_string())))
}

fn stream_event(candle: Candle, closed: bool) -> MarketStreamEvent {
    MarketStreamEvent { candle, closed }
}

fn latest_closed_open_time(timeframe: Timeframe) -> i64 {
    timeframe.align_open_time(now_ms() - timeframe.duration_ms())
}

fn anchored_candle(
    latest_closed_open_time: i64,
    offset_from_latest_closed: i64,
    bull: f64,
    closed: bool,
) -> Candle {
    candle_with_bull_at_open_time(
        latest_closed_open_time + offset_from_latest_closed * Timeframe::M1.duration_ms(),
        bull,
        closed,
    )
}

fn recent_closed_window(count: usize, bull: f64) -> Vec<Candle> {
    recent_closed_window_for(Symbol::BtcUsdt, Timeframe::M1, count, bull)
}

fn recent_closed_window_for(
    symbol: Symbol,
    timeframe: Timeframe,
    count: usize,
    bull: f64,
) -> Vec<Candle> {
    let end_open_time = latest_closed_open_time(timeframe);
    let start_open_time = end_open_time - (count as i64 - 1) * timeframe.duration_ms();

    (0..count)
        .map(|index| {
            let open_time = start_open_time + index as i64 * timeframe.duration_ms();
            let mut candle = candle_with_bull_at_open_time(open_time, bull, true);
            candle.symbol = symbol;
            candle.timeframe = timeframe;
            candle.close_time = timeframe.close_time_for_open(open_time);
            candle
        })
        .collect()
}

#[derive(Default)]
struct InMemoryRepository {
    settings: Mutex<Settings>,
    candles: Mutex<Vec<Candle>>,
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
    mainnet_auto_status: Mutex<Option<MainnetAutoStatus>>,
    mainnet_auto_risk_budget: Mutex<Option<MainnetAutoRiskBudget>>,
    mainnet_auto_decisions: Mutex<Vec<MainnetAutoDecisionEvent>>,
    mainnet_auto_watchdog_events: Mutex<Vec<MainnetAutoWatchdogEvent>>,
    mainnet_auto_lessons: Mutex<Vec<MainnetAutoLessonReport>>,
    live_orders: Mutex<Vec<LiveOrderRecord>>,
    live_fills: Mutex<Vec<LiveFillRecord>>,
    fail_clear_trades: bool,
}

#[async_trait]
impl Repository for InMemoryRepository {
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
            .candles
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
        let mut candles = self.candles.lock().await;
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
        _: Symbol,
        _: Timeframe,
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
        if self.fail_clear_trades {
            return Err(AppError::Other(anyhow::anyhow!(
                "controlled clear_trades failure"
            )));
        }
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
        let mut results = self.live_preflights.lock().await.clone();
        if results.len() > limit {
            results = results.split_off(results.len() - limit);
        }
        Ok(results)
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

    async fn load_mainnet_auto_status(&self) -> AppResult<MainnetAutoStatus> {
        Ok(self
            .mainnet_auto_status
            .lock()
            .await
            .clone()
            .unwrap_or_default())
    }

    async fn save_mainnet_auto_status(&self, status: &MainnetAutoStatus) -> AppResult<()> {
        *self.mainnet_auto_status.lock().await = Some(status.clone());
        Ok(())
    }

    async fn load_mainnet_auto_risk_budget(&self) -> AppResult<MainnetAutoRiskBudget> {
        Ok(self
            .mainnet_auto_risk_budget
            .lock()
            .await
            .clone()
            .unwrap_or_default())
    }

    async fn save_mainnet_auto_risk_budget(&self, budget: &MainnetAutoRiskBudget) -> AppResult<()> {
        *self.mainnet_auto_risk_budget.lock().await = Some(budget.clone());
        Ok(())
    }

    async fn append_mainnet_auto_decision(
        &self,
        decision: &MainnetAutoDecisionEvent,
    ) -> AppResult<()> {
        self.mainnet_auto_decisions
            .lock()
            .await
            .push(decision.clone());
        Ok(())
    }

    async fn list_mainnet_auto_decisions(
        &self,
        limit: usize,
    ) -> AppResult<Vec<MainnetAutoDecisionEvent>> {
        let mut decisions = self.mainnet_auto_decisions.lock().await.clone();
        decisions.sort_by_key(|decision| decision.created_at);
        if decisions.len() > limit {
            decisions = decisions.split_off(decisions.len() - limit);
        }
        Ok(decisions)
    }

    async fn append_mainnet_auto_watchdog_event(
        &self,
        event: &MainnetAutoWatchdogEvent,
    ) -> AppResult<()> {
        self.mainnet_auto_watchdog_events
            .lock()
            .await
            .push(event.clone());
        Ok(())
    }

    async fn list_mainnet_auto_watchdog_events(
        &self,
        limit: usize,
    ) -> AppResult<Vec<MainnetAutoWatchdogEvent>> {
        let mut events = self.mainnet_auto_watchdog_events.lock().await.clone();
        events.sort_by_key(|event| event.created_at);
        if events.len() > limit {
            events = events.split_off(events.len() - limit);
        }
        Ok(events)
    }

    async fn save_mainnet_auto_lesson_report(
        &self,
        report: &MainnetAutoLessonReport,
    ) -> AppResult<()> {
        self.mainnet_auto_lessons.lock().await.push(report.clone());
        Ok(())
    }

    async fn latest_mainnet_auto_lesson_report(
        &self,
    ) -> AppResult<Option<MainnetAutoLessonReport>> {
        Ok(self
            .mainnet_auto_lessons
            .lock()
            .await
            .iter()
            .max_by_key(|report| report.created_at)
            .cloned())
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

#[derive(Default)]
struct ServerFakeLiveExchange {
    submitted_orders: Mutex<Vec<LiveOrderRecord>>,
}

#[async_trait]
impl LiveExchangePort for ServerFakeLiveExchange {
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
            validated_at: now_ms(),
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
            position_mode: Some("one_way".to_string()),
            account_mode_checked_at: Some(now_ms()),
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
            fetched_at: now_ms(),
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
            fetched_at: now_ms(),
        })
    }

    async fn fetch_account_mode(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountModeStatus> {
        Ok(LiveAccountModeStatus {
            environment,
            position_mode: Some("one_way".to_string()),
            multi_assets_margin: Some(false),
            fetched_at: now_ms(),
        })
    }

    async fn create_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<String> {
        Ok("server-listen-key".to_string())
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
        Ok(Box::pin(
            futures::stream::iter(Vec::<Result<LiveUserDataEvent, AppError>>::new())
                .chain(futures::stream::pending()),
        ))
    }

    async fn preflight_order_test(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _payload: &BTreeMap<String, String>,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn submit_order(
        &self,
        environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        payload: &BTreeMap<String, String>,
    ) -> AppResult<LiveOrderRecord> {
        let now = now_ms();
        let client_order_id = payload
            .get("newClientOrderId")
            .cloned()
            .unwrap_or_else(|| "server-client-order".to_string());
        let order = LiveOrderRecord {
            id: client_order_id.clone(),
            credential_id: None,
            environment,
            symbol: Symbol::BtcUsdt,
            side: LiveOrderSide::Buy,
            order_type: LiveOrderType::Market,
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
            reason: "server_fake".to_string(),
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
        let now = now_ms();
        let client_order_id = orig_client_order_id
            .unwrap_or("server-client-order")
            .to_string();
        Ok(LiveOrderRecord {
            id: client_order_id.clone(),
            credential_id: None,
            environment,
            symbol,
            side: LiveOrderSide::Buy,
            order_type: LiveOrderType::Market,
            status: LiveOrderStatus::Canceled,
            client_order_id,
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
            reason: "server_fake".to_string(),
            payload: BTreeMap::new(),
            response_type: Some("ACK".to_string()),
            self_trade_prevention_mode: None,
            price_match: None,
            expire_reason: None,
            last_error: None,
            submitted_at: now,
            updated_at: now,
        })
    }
}

struct SequenceMarket {
    range_fetches: Mutex<VecDeque<Vec<Candle>>>,
    subscriptions: Mutex<VecDeque<Vec<Result<MarketStreamEvent, AppError>>>>,
}

impl SequenceMarket {
    fn new(
        subscriptions: Vec<Vec<Result<MarketStreamEvent, AppError>>>,
        range_fetches: Vec<Vec<Candle>>,
    ) -> Self {
        Self {
            range_fetches: Mutex::new(range_fetches.into()),
            subscriptions: Mutex::new(subscriptions.into()),
        }
    }
}

#[async_trait]
impl MarketDataPort for SequenceMarket {
    async fn fetch_klines_range(&self, _request: KlineRangeRequest) -> AppResult<Vec<Candle>> {
        Ok(self
            .range_fetches
            .lock()
            .await
            .pop_front()
            .unwrap_or_default())
    }

    async fn subscribe_klines(&self, _: Symbol, _: Timeframe) -> AppResult<MarketStream> {
        let events = self
            .subscriptions
            .lock()
            .await
            .pop_front()
            .unwrap_or_default();
        Ok(Box::pin(
            futures::stream::iter(events).chain(futures::stream::pending()),
        ))
    }
}

struct StaticMetrics;

impl relxen_app::MetricsPort for StaticMetrics {
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

struct TestServer {
    base_ws: String,
    shutdown_tx: oneshot::Sender<()>,
    task: JoinHandle<std::io::Result<()>>,
}

impl TestServer {
    async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.task.await;
    }
}

async fn spawn_server(service: Arc<AppService>, event_bus: EventBus) -> TestServer {
    let router = build_router(
        RouterState { service, event_bus },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    TestServer {
        base_ws: format!("ws://{address}"),
        shutdown_tx,
        task,
    }
}

async fn recv_ws_event(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> relxen_app::OutboundEvent {
    timeout(Duration::from_secs(5), async {
        loop {
            match socket.next().await {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(&text).expect("valid websocket event payload");
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => panic!("websocket read failed: {error}"),
                None => panic!("websocket closed before delivering an event"),
            }
        }
    })
    .await
    .expect("timed out waiting for websocket event")
}

async fn collect_ws_events_until(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    label: &str,
    mut predicate: impl FnMut(&[relxen_app::OutboundEvent]) -> bool,
) -> Vec<relxen_app::OutboundEvent> {
    timeout(Duration::from_secs(5), async {
        let mut events = Vec::new();
        loop {
            events.push(recv_ws_event(socket).await);
            if predicate(&events) {
                return events;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out while waiting for {label}"))
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn mainnet_auto_test_router() -> (axum::Router, Arc<ServerFakeLiveExchange>) {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            auto_restart_on_apply: false,
            paper_enabled: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let exchange = Arc::new(ServerFakeLiveExchange::default());
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(39, 25.0)],
        )),
        LiveDependencies::new(Arc::new(MemorySecretStore::default()), exchange.clone()),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            auto_start: false,
            history_limit: 39,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    (router, exchange)
}

fn seeded_live_order(environment: LiveEnvironment, order_ref: &str) -> LiveOrderRecord {
    let now = now_ms();
    LiveOrderRecord {
        id: order_ref.to_string(),
        credential_id: None,
        environment,
        symbol: Symbol::BtcUsdt,
        side: LiveOrderSide::Buy,
        order_type: LiveOrderType::Limit,
        status: LiveOrderStatus::Working,
        client_order_id: order_ref.to_string(),
        exchange_order_id: Some("42".to_string()),
        quantity: "0.001".to_string(),
        price: Some("77800".to_string()),
        executed_qty: "0".to_string(),
        avg_price: None,
        reduce_only: false,
        time_in_force: Some("GTC".to_string()),
        intent_id: None,
        intent_hash: None,
        source_signal_id: None,
        source_open_time: None,
        reason: "server_cancel_test".to_string(),
        payload: BTreeMap::new(),
        response_type: Some("ACK".to_string()),
        self_trade_prevention_mode: None,
        price_match: None,
        expire_reason: None,
        last_error: None,
        submitted_at: now,
        updated_at: now,
    }
}

async fn router_with_seeded_cancel_order(
    environment: LiveEnvironment,
    enable_mainnet_canary_execution: bool,
) -> (axum::Router, String) {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            auto_restart_on_apply: false,
            paper_enabled: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository.clone(),
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(3, 80.0)],
        )),
        LiveDependencies::new(
            Arc::new(MemorySecretStore::new()),
            Arc::new(ServerFakeLiveExchange::default()),
        ),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 3,
            auto_start: false,
            enable_mainnet_canary_execution,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    let credential = service
        .create_live_credential(relxen_domain::CreateLiveCredentialRequest {
            alias: format!("{environment}-cancel"),
            environment,
            api_key: "abcd1234efgh5678".to_string(),
            api_secret: "secret".to_string(),
        })
        .await
        .unwrap();
    service
        .validate_live_credential(credential.id)
        .await
        .unwrap();
    let order_ref = "rx_exec_cancel_route_test".to_string();
    repository
        .upsert_live_order(&seeded_live_order(environment, &order_ref))
        .await
        .unwrap();
    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    (router, order_ref)
}

#[tokio::test]
async fn bootstrap_endpoint_returns_snapshot() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            auto_restart_on_apply: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let event_bus = EventBus::new(16);
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(8, 0.0)],
        )),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 8,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let router = build_router(
        RouterState { service, event_bus },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/bootstrap")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn execute_endpoint_submits_testnet_order_and_status_exposes_execution_state() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            auto_restart_on_apply: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(3, 80.0)],
        )),
        LiveDependencies::new(
            Arc::new(MemorySecretStore::new()),
            Arc::new(ServerFakeLiveExchange::default()),
        ),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 3,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    let credential = service
        .create_live_credential(relxen_domain::CreateLiveCredentialRequest {
            alias: "testnet".to_string(),
            environment: LiveEnvironment::Testnet,
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
    let preview = service
        .build_live_intent_preview(LiveOrderType::Market, None)
        .await
        .unwrap();
    let intent_id = preview.intent.unwrap().id;

    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/execute")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&LiveExecutionRequest {
                        intent_id: Some(intent_id),
                        confirm_testnet: true,
                        confirm_mainnet_canary: false,
                        confirmation_text: None,
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["order"]["status"], "accepted");
    assert_eq!(body["order"]["response_type"], "ACK");

    let status_response = router
        .oneshot(
            Request::builder()
                .uri("/api/live/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status = response_json(status_response).await;
    assert!(!status["execution"]["recent_orders"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn cancel_endpoint_uses_path_order_ref_without_requiring_body_order_ref() {
    let (router, order_ref) =
        router_with_seeded_cancel_order(LiveEnvironment::Testnet, false).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/live/orders/{order_ref}/cancel"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "confirm_testnet": true
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["order"]["client_order_id"], order_ref);
    assert_eq!(body["order"]["status"], "canceled");
}

#[tokio::test]
async fn cancel_endpoint_accepts_matching_body_order_ref_for_compatibility() {
    let (router, order_ref) =
        router_with_seeded_cancel_order(LiveEnvironment::Testnet, false).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/live/orders/{order_ref}/cancel"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "order_ref": order_ref,
                        "confirm_testnet": true
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["order"]["status"], "canceled");
}

#[tokio::test]
async fn cancel_endpoint_rejects_mismatched_body_order_ref() {
    let (router, order_ref) =
        router_with_seeded_cancel_order(LiveEnvironment::Testnet, false).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/live/orders/{order_ref}/cancel"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "order_ref": "different-order",
                        "confirm_testnet": true
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["kind"], "validation");
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("must match the route path"));
}

#[tokio::test]
async fn cancel_endpoint_keeps_mainnet_confirmation_required() {
    let (router, order_ref) = router_with_seeded_cancel_order(LiveEnvironment::Mainnet, true).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/live/orders/{order_ref}/cancel"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "confirm_mainnet_canary": true,
                        "confirmation_text": "CANCEL MAINNET BTCUSDT wrong-order"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["accepted"], false);
    assert_eq!(body["blocking_reason"], "mainnet_confirmation_missing");
}

#[tokio::test]
async fn live_kill_switch_risk_profile_and_auto_endpoints_update_status() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            auto_restart_on_apply: false,
            paper_enabled: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(3, 80.0)],
        )),
        LiveDependencies::new(
            Arc::new(MemorySecretStore::new()),
            Arc::new(ServerFakeLiveExchange::default()),
        ),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 3,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    let credential = service
        .create_live_credential(relxen_domain::CreateLiveCredentialRequest {
            alias: "testnet".to_string(),
            environment: LiveEnvironment::Testnet,
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

    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let engage = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/kill-switch/engage")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"api_test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(engage.status(), StatusCode::OK);
    let body = response_json(engage).await;
    assert_eq!(body["kill_switch"]["engaged"], true);
    assert_eq!(body["execution"]["kill_switch_engaged"], true);

    let release = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/kill-switch/release")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"api_test_release"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(release.status(), StatusCode::OK);

    let risk = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/live/risk-profile")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"configured":true,"profile_name":"api-test","limits":{"max_notional_per_order":"1000","max_open_notional_active_symbol":"1000","max_leverage":"10","max_orders_per_session":10,"max_fills_per_session":20,"max_consecutive_rejections":3,"max_daily_realized_loss":"250"},"updated_at":0}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(risk.status(), StatusCode::OK);
    let risk_body = response_json(risk).await;
    assert_eq!(risk_body["risk_profile"]["configured"], true);

    let auto = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/auto/start")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"confirm_testnet_auto":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(auto.status(), StatusCode::OK);
    let auto_body = response_json(auto).await;
    assert_eq!(auto_body["auto_executor"]["state"], "running");
}

#[tokio::test]
async fn mainnet_auto_status_and_live_start_are_blocked_by_default() {
    let (router, exchange) = mainnet_auto_test_router().await;

    let status_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/live/mainnet-auto/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let status = response_json(status_response).await;
    assert_eq!(status["state"], "disabled");
    assert_eq!(status["config"]["enable_live_execution"], false);

    let start_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/mainnet-auto/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start_response.status(), StatusCode::OK);
    let body = response_json(start_response).await;
    assert_eq!(body["state"], "config_blocked");
    assert!(body["current_blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason == "mainnet_auto_config_disabled"));
    assert!(exchange.submitted_orders.lock().await.is_empty());
}

#[tokio::test]
async fn mainnet_auto_dry_run_endpoints_record_decisions_without_orders() {
    let (router, exchange) = mainnet_auto_test_router().await;

    let start_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/mainnet-auto/dry-run/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start_response.status(), StatusCode::OK);
    let body = response_json(start_response).await;
    assert_eq!(body["state"], "dry_run_running");
    assert_eq!(body["live_orders_submitted"], 0);

    let decisions_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/live/mainnet-auto/decisions?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(decisions_response.status(), StatusCode::OK);
    let decisions = response_json(decisions_response).await;
    assert_eq!(decisions.as_array().unwrap().len(), 1);
    assert_eq!(decisions[0]["mode"], "dry_run");

    let lessons_response = router
        .oneshot(
            Request::builder()
                .uri("/api/live/mainnet-auto/lessons/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(lessons_response.status(), StatusCode::OK);
    let lessons = response_json(lessons_response).await;
    assert_eq!(lessons["live_order_submitted"], false);
    assert!(exchange.submitted_orders.lock().await.is_empty());
}

#[tokio::test]
async fn websocket_endpoint_streams_recovered_events_after_reconnect() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let anchor = latest_closed_open_time(Timeframe::M1);
    repository
        .upsert_kline(&anchored_candle(anchor, -1, 0.0, true))
        .await
        .unwrap();
    repository
        .upsert_kline(&anchored_candle(anchor, 0, 0.0, true))
        .await
        .unwrap();

    let recovered = anchored_candle(anchor, 1, 0.0, true);
    let live_partial = anchored_candle(anchor, 2, 0.0, false);
    let live_closed = anchored_candle(anchor, 2, 0.0, true);
    let market = Arc::new(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![
                Ok(stream_event(live_partial.clone(), false)),
                Ok(stream_event(live_closed, true)),
            ],
        ],
        vec![vec![recovered.clone()]],
    ));
    let event_bus = EventBus::new(128);
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market,
        Arc::new(StaticMetrics),
        Arc::new(event_bus.clone()),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let server = spawn_server(service.clone(), event_bus).await;
    let (mut socket, _) = tokio_tungstenite::connect_async(format!("{}/api/ws", server.base_ws))
        .await
        .unwrap();

    let initial = recv_ws_event(&mut socket).await;
    assert!(matches!(initial, relxen_app::OutboundEvent::Snapshot(_)));

    service.start_runtime().await.unwrap();

    let events = collect_ws_events_until(&mut socket, "websocket recovery events", |events| {
        let saw_connected = events.iter().any(|event| {
            matches!(
                event,
                relxen_app::OutboundEvent::ConnectionChanged(state)
                    if state.status == ConnectionStatus::Connected
            )
        });
        let saw_recovered_candle = events.iter().any(|event| {
            matches!(
                event,
                relxen_app::OutboundEvent::CandleClosed(candle)
                    if candle.open_time == recovered.open_time
            )
        });
        saw_connected && saw_recovered_candle
    })
    .await;

    service.stop_runtime().await.unwrap();
    server.shutdown().await;

    assert!(!events
        .iter()
        .any(|event| matches!(event, relxen_app::OutboundEvent::ResyncRequired { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        relxen_app::OutboundEvent::ConnectionChanged(state)
            if state.status == ConnectionStatus::Reconnecting
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        relxen_app::OutboundEvent::ConnectionChanged(state)
            if state.status == ConnectionStatus::Stale
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        relxen_app::OutboundEvent::ConnectionChanged(state)
            if state.status == ConnectionStatus::Resynced
    )));
}

#[tokio::test]
async fn websocket_endpoint_emits_resync_required_for_irrecoverable_gaps() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let anchor = latest_closed_open_time(Timeframe::M1);
    repository
        .upsert_kline(&anchored_candle(anchor, -1, 0.0, true))
        .await
        .unwrap();
    repository
        .upsert_kline(&anchored_candle(anchor, 0, 0.0, true))
        .await
        .unwrap();

    let market = Arc::new(SequenceMarket::new(
        vec![
            vec![stream_error("socket dropped")],
            vec![Ok(stream_event(
                anchored_candle(anchor, 4, 0.0, false),
                false,
            ))],
        ],
        vec![vec![
            anchored_candle(anchor, 2, 0.0, true),
            anchored_candle(anchor, 3, 0.0, true),
        ]],
    ));
    let event_bus = EventBus::new(128);
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market,
        Arc::new(StaticMetrics),
        Arc::new(event_bus.clone()),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let server = spawn_server(service.clone(), event_bus).await;
    let (mut socket, _) = tokio_tungstenite::connect_async(format!("{}/api/ws", server.base_ws))
        .await
        .unwrap();

    let initial = recv_ws_event(&mut socket).await;
    assert!(matches!(initial, relxen_app::OutboundEvent::Snapshot(_)));

    service.start_runtime().await.unwrap();

    let events = collect_ws_events_until(&mut socket, "websocket resync_required", |events| {
        events
            .iter()
            .any(|event| matches!(event, relxen_app::OutboundEvent::ResyncRequired { .. }))
    })
    .await;

    service.stop_runtime().await.unwrap();
    server.shutdown().await;

    assert!(events.iter().any(|event| matches!(
        event,
        relxen_app::OutboundEvent::ResyncRequired { reason }
            if reason.contains("returned 2 candles but 3 were required")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        relxen_app::OutboundEvent::ConnectionChanged(state)
            if state.status == ConnectionStatus::Stale
    )));
}

#[tokio::test]
async fn websocket_endpoint_streams_trade_events() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let anchor = latest_closed_open_time(Timeframe::M1);
    repository
        .upsert_kline(&anchored_candle(anchor, -1, 0.0, true))
        .await
        .unwrap();
    repository
        .upsert_kline(&anchored_candle(anchor, 0, 40.0, true))
        .await
        .unwrap();

    let market = Arc::new(SequenceMarket::new(
        vec![vec![
            Ok(stream_event(anchored_candle(anchor, 1, 100.0, true), true)),
            Ok(stream_event(anchored_candle(anchor, 2, 20.0, true), true)),
            Ok(stream_event(anchored_candle(anchor, 3, 20.0, true), true)),
        ]],
        Vec::new(),
    ));
    let event_bus = EventBus::new(128);
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market,
        Arc::new(StaticMetrics),
        Arc::new(event_bus.clone()),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let server = spawn_server(service.clone(), event_bus).await;
    let (mut socket, _) = tokio_tungstenite::connect_async(format!("{}/api/ws", server.base_ws))
        .await
        .unwrap();
    let initial = recv_ws_event(&mut socket).await;
    assert!(matches!(initial, relxen_app::OutboundEvent::Snapshot(_)));

    service.start_runtime().await.unwrap();

    let events = collect_ws_events_until(&mut socket, "trade websocket events", |events| {
        events
            .iter()
            .filter(|event| matches!(event, relxen_app::OutboundEvent::TradeAppended(_)))
            .count()
            >= 3
    })
    .await;

    service.stop_runtime().await.unwrap();
    server.shutdown().await;

    let trades: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            relxen_app::OutboundEvent::TradeAppended(trade) => Some(trade),
            _ => None,
        })
        .collect();
    assert_eq!(trades.len(), 3);
    assert_eq!(trades[0].action, relxen_domain::TradeAction::Open);
    assert_eq!(trades[1].action, relxen_domain::TradeAction::Reverse);
    assert_eq!(trades[2].action, relxen_domain::TradeAction::Open);
}

#[tokio::test]
async fn put_settings_rebuilds_state_coherently() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            auto_restart_on_apply: false,
            ..Settings::default()
        })
        .await
        .unwrap();
    let anchor = latest_closed_open_time(Timeframe::M1);
    repository
        .upsert_kline(&anchored_candle(anchor, -1, 0.0, true))
        .await
        .unwrap();
    repository
        .upsert_kline(&anchored_candle(anchor, 0, 0.0, true))
        .await
        .unwrap();

    let m5_end_open_time = latest_closed_open_time(Timeframe::M5);
    let rebuilt_m5 = vec![
        Candle {
            symbol: Symbol::BtcUsdt,
            timeframe: Timeframe::M5,
            open_time: m5_end_open_time - Timeframe::M5.duration_ms(),
            close_time: Timeframe::M5
                .close_time_for_open(m5_end_open_time - Timeframe::M5.duration_ms()),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1.0,
            closed: true,
        },
        Candle {
            symbol: Symbol::BtcUsdt,
            timeframe: Timeframe::M5,
            open_time: m5_end_open_time,
            close_time: Timeframe::M5.close_time_for_open(m5_end_open_time),
            open: 101.0,
            high: 102.0,
            low: 100.0,
            close: 101.5,
            volume: 1.0,
            closed: true,
        },
    ];
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(Vec::new(), vec![rebuilt_m5.clone()])),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&Settings {
                        timeframe: Timeframe::M5,
                        aso_length: 2,
                        aso_mode: relxen_domain::AsoMode::Intrabar,
                        auto_restart_on_apply: false,
                        ..Settings::default()
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let snapshot: BootstrapPayload =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(snapshot.runtime_status.timeframe, Timeframe::M5);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M5));
}

#[tokio::test]
async fn put_settings_rebuild_failure_returns_typed_history_error_and_keeps_old_snapshot() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            auto_restart_on_apply: true,
            ..Settings::default()
        })
        .await
        .unwrap();
    let market = Arc::new(SequenceMarket::new(
        Vec::new(),
        vec![
            recent_closed_window_for(Symbol::BtcUsdt, Timeframe::M1, 2, 20.0),
            recent_closed_window_for(Symbol::BtcUsdt, Timeframe::M5, 2, 80.0)
                .into_iter()
                .take(1)
                .collect(),
        ],
    ));
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        market,
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&Settings {
                        timeframe: Timeframe::M5,
                        aso_length: 2,
                        aso_mode: relxen_domain::AsoMode::Intrabar,
                        auto_restart_on_apply: true,
                        ..Settings::default()
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response_json(response).await;
    assert_eq!(body["kind"], "history");
    assert_eq!(body["status"], 422);

    let bootstrap_response = router
        .oneshot(
            Request::builder()
                .uri("/api/bootstrap")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bootstrap_response.status(), StatusCode::OK);
    let snapshot: BootstrapPayload =
        serde_json::from_value(response_json(bootstrap_response).await).unwrap();
    assert_eq!(snapshot.runtime_status.timeframe, Timeframe::M1);
    assert_eq!(snapshot.settings.timeframe, Timeframe::M1);
    assert!(snapshot
        .candles
        .iter()
        .all(|candle| candle.timeframe == Timeframe::M1));
}

#[tokio::test]
async fn paper_reset_failure_returns_structured_internal_error() {
    let repository = Arc::new(InMemoryRepository {
        fail_clear_trades: true,
        ..InMemoryRepository::default()
    });
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(2, 20.0)],
        )),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/paper/reset")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_json(response).await;
    assert_eq!(body["kind"], "internal");
    assert_eq!(body["status"], 500);
}

#[tokio::test]
async fn runtime_start_stop_endpoints_remain_coherent() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(2, 20.0)],
        )),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();

    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );
    let start_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runtime/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start_response.status(), StatusCode::OK);
    assert_eq!(response_json(start_response).await["running"], true);

    let stop_response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runtime/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stop_response.status(), StatusCode::OK);
    assert_eq!(response_json(stop_response).await["running"], false);
}

#[tokio::test]
async fn live_credential_api_crud_validate_readiness_and_start_blocked() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(2, 20.0)],
        )),
        LiveDependencies::new(
            Arc::new(MemorySecretStore::new()),
            Arc::new(ServerFakeLiveExchange::default()),
        ),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );

    let create_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/credentials")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "alias": "Testnet",
                        "environment": "testnet",
                        "api_key": "abcd1234efgh5678",
                        "api_secret": "super-secret"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let created = response_json(create_response).await;
    let credential_id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["api_key_hint"], "abcd…5678");
    assert!(!created.to_string().contains("super-secret"));

    let validate_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/live/credentials/{credential_id}/validate"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(validate_response.status(), StatusCode::OK);
    assert_eq!(response_json(validate_response).await["status"], "valid");

    let refresh_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/readiness/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let readiness = response_json(refresh_response).await;
    assert_eq!(readiness["state"], "ready_read_only");
    assert_eq!(readiness["readiness"]["can_arm"], true);

    let arm_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/arm")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(arm_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(arm_response).await["state"],
        "armed_read_only"
    );

    let start_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/start-check")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start_response.status(), StatusCode::OK);
    let start = response_json(start_response).await;
    assert_eq!(start["allowed"], false);
    assert!(start["blocking_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason == "execution_not_implemented"));

    let shadow_start_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/shadow/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(shadow_start_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(shadow_start_response).await["reconciliation"]["stream"]["state"],
        "running"
    );

    let preview_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/live/intent/preview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(preview_response).await["intent"]["can_execute_now"],
        true
    );

    let preflight_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/preflight")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preflight_response.status(), StatusCode::OK);
    let preflight = response_json(preflight_response).await;
    assert_eq!(preflight["accepted"], true);
    assert_eq!(
        preflight["message"],
        "PREFLIGHT PASSED. No order was placed."
    );

    let shadow_stop_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/shadow/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(shadow_stop_response.status(), StatusCode::OK);

    let list_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/live/credentials")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    assert!(!response_json(list_response)
        .await
        .to_string()
        .contains("super-secret"));

    let delete_response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/live/credentials/{credential_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn live_secure_store_unavailable_returns_typed_error() {
    let repository = Arc::new(InMemoryRepository::default());
    repository
        .save_settings(&Settings {
            aso_length: 2,
            aso_mode: relxen_domain::AsoMode::Intrabar,
            ..Settings::default()
        })
        .await
        .unwrap();
    let service = AppService::new_with_live(
        AppMetadata::default(),
        repository,
        Arc::new(SequenceMarket::new(
            Vec::new(),
            vec![recent_closed_window(2, 20.0)],
        )),
        LiveDependencies::new(
            Arc::new(MemorySecretStore::unavailable()),
            Arc::new(ServerFakeLiveExchange::default()),
        ),
        Arc::new(StaticMetrics),
        Arc::new(NoopPublisher),
        ServiceOptions {
            history_limit: 2,
            auto_start: false,
            ..ServiceOptions::default()
        },
    );
    service.initialize().await.unwrap();
    let router = build_router(
        RouterState {
            service,
            event_bus: EventBus::new(16),
        },
        std::path::PathBuf::from("/Users/stk/Desktop/RelXen/web/dist"),
    );

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/live/credentials")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "alias": "Testnet",
                        "environment": "testnet",
                        "api_key": "abcd1234efgh5678",
                        "api_secret": "super-secret"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(response).await;
    assert_eq!(body["kind"], "secure_store_unavailable");
}
