use std::collections::BTreeMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use relxen_domain::{
    Candle, LiveAccountShadow, LiveAccountSnapshot, LiveCredentialId, LiveCredentialMetadata,
    LiveCredentialSecret, LiveCredentialValidationResult, LiveEnvironment, LiveExecutionSnapshot,
    LiveFillRecord, LiveOrderPreflightResult, LiveOrderRecord, LiveReconciliationStatus,
    LiveStateRecord, LiveSymbolRules, LiveUserDataEvent, LogEvent, Position, Settings, SignalEvent,
    Symbol, SystemMetrics, Timeframe, Trade, Wallet,
};

use crate::{AppError, AppResult, OutboundEvent};

#[derive(Debug, Clone)]
pub struct MarketStreamEvent {
    pub candle: Candle,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KlineRangeRequest {
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub start_open_time: i64,
    pub end_open_time: i64,
}

pub type MarketStream = Pin<Box<dyn Stream<Item = Result<MarketStreamEvent, AppError>> + Send>>;
pub type LiveUserDataStream =
    Pin<Box<dyn Stream<Item = Result<LiveUserDataEvent, AppError>> + Send>>;

#[async_trait]
pub trait Repository: Send + Sync {
    async fn load_settings(&self) -> AppResult<Settings>;
    async fn save_settings(&self, settings: &Settings) -> AppResult<()>;
    async fn load_recent_klines(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
        limit: usize,
    ) -> AppResult<Vec<Candle>>;
    async fn upsert_kline(&self, candle: &Candle) -> AppResult<()>;
    async fn list_signals(&self, limit: usize) -> AppResult<Vec<SignalEvent>>;
    async fn sync_signals(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
        signals: &[SignalEvent],
    ) -> AppResult<()>;
    async fn append_signal(&self, signal: &SignalEvent) -> AppResult<()>;
    async fn list_trades(&self, limit: usize) -> AppResult<Vec<Trade>>;
    async fn append_trade(&self, trade: &Trade) -> AppResult<()>;
    async fn clear_trades(&self) -> AppResult<()>;
    async fn load_wallets(&self) -> AppResult<Vec<Wallet>>;
    async fn save_wallets(&self, wallets: &[Wallet]) -> AppResult<()>;
    async fn load_position(&self) -> AppResult<Option<Position>>;
    async fn save_position(&self, position: Option<&Position>) -> AppResult<()>;
    async fn recent_logs(&self, limit: usize) -> AppResult<Vec<LogEvent>>;
    async fn append_log(&self, log: &LogEvent) -> AppResult<()>;
    async fn list_live_credentials(&self) -> AppResult<Vec<LiveCredentialMetadata>>;
    async fn get_live_credential(
        &self,
        id: &LiveCredentialId,
    ) -> AppResult<Option<LiveCredentialMetadata>>;
    async fn active_live_credential(
        &self,
        environment: LiveEnvironment,
    ) -> AppResult<Option<LiveCredentialMetadata>>;
    async fn upsert_live_credential(&self, credential: &LiveCredentialMetadata) -> AppResult<()>;
    async fn delete_live_credential(&self, id: &LiveCredentialId) -> AppResult<()>;
    async fn select_live_credential(
        &self,
        id: &LiveCredentialId,
        environment: LiveEnvironment,
    ) -> AppResult<()>;
    async fn load_live_state(&self) -> AppResult<LiveStateRecord>;
    async fn save_live_state(&self, state: &LiveStateRecord) -> AppResult<()>;
    async fn load_live_reconciliation(&self) -> AppResult<Option<LiveReconciliationStatus>>;
    async fn save_live_reconciliation(&self, status: &LiveReconciliationStatus) -> AppResult<()>;
    async fn load_live_shadow(&self) -> AppResult<Option<LiveAccountShadow>>;
    async fn save_live_shadow(&self, shadow: &LiveAccountShadow) -> AppResult<()>;
    async fn list_live_preflights(&self, limit: usize) -> AppResult<Vec<LiveOrderPreflightResult>>;
    async fn append_live_preflight(&self, result: &LiveOrderPreflightResult) -> AppResult<()>;
    async fn load_live_execution(&self) -> AppResult<Option<LiveExecutionSnapshot>>;
    async fn save_live_execution(&self, execution: &LiveExecutionSnapshot) -> AppResult<()>;
    async fn list_live_orders(&self, limit: usize) -> AppResult<Vec<LiveOrderRecord>>;
    async fn get_live_order(&self, order_ref: &str) -> AppResult<Option<LiveOrderRecord>>;
    async fn upsert_live_order(&self, order: &LiveOrderRecord) -> AppResult<()>;
    async fn list_live_fills(&self, limit: usize) -> AppResult<Vec<LiveFillRecord>>;
    async fn append_live_fill(&self, fill: &LiveFillRecord) -> AppResult<()>;
}

#[async_trait]
pub trait MarketDataPort: Send + Sync {
    async fn fetch_klines_range(&self, request: KlineRangeRequest) -> AppResult<Vec<Candle>>;
    async fn subscribe_klines(
        &self,
        symbol: Symbol,
        timeframe: Timeframe,
    ) -> AppResult<MarketStream>;
}

pub trait MetricsPort: Send + Sync {
    fn snapshot(&self) -> SystemMetrics;
}

pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: OutboundEvent);
}

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn store(&self, id: &LiveCredentialId, secret: &LiveCredentialSecret) -> AppResult<()>;
    async fn read(&self, id: &LiveCredentialId) -> AppResult<LiveCredentialSecret>;
    async fn delete(&self, id: &LiveCredentialId) -> AppResult<()>;
    async fn ensure_available(&self) -> AppResult<()>;
}

#[async_trait]
pub trait LiveExchangePort: Send + Sync {
    async fn validate_credentials(
        &self,
        environment: LiveEnvironment,
        credential_id: &LiveCredentialId,
        secret: &LiveCredentialSecret,
    ) -> AppResult<LiveCredentialValidationResult>;

    async fn fetch_account_snapshot(
        &self,
        environment: LiveEnvironment,
        secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountSnapshot>;

    async fn fetch_symbol_rules(
        &self,
        environment: LiveEnvironment,
        symbol: Symbol,
    ) -> AppResult<LiveSymbolRules>;

    async fn create_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<String> {
        Err(AppError::Exchange(
            "live user-data stream adapter is not configured".to_string(),
        ))
    }

    async fn keepalive_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _listen_key: &str,
    ) -> AppResult<()> {
        Err(AppError::Exchange(
            "live user-data stream adapter is not configured".to_string(),
        ))
    }

    async fn close_listen_key(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _listen_key: &str,
    ) -> AppResult<()> {
        Err(AppError::Exchange(
            "live user-data stream adapter is not configured".to_string(),
        ))
    }

    async fn subscribe_user_data(
        &self,
        _environment: LiveEnvironment,
        _listen_key: &str,
    ) -> AppResult<LiveUserDataStream> {
        Err(AppError::Exchange(
            "live user-data websocket adapter is not configured".to_string(),
        ))
    }

    async fn preflight_order_test(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _payload: &BTreeMap<String, String>,
    ) -> AppResult<()> {
        Err(AppError::Exchange(
            "live order-test adapter is not configured".to_string(),
        ))
    }

    async fn submit_order(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _payload: &BTreeMap<String, String>,
    ) -> AppResult<LiveOrderRecord> {
        Err(AppError::Exchange(
            "live order submission adapter is not configured".to_string(),
        ))
    }

    async fn cancel_order(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _symbol: Symbol,
        _orig_client_order_id: Option<&str>,
        _order_id: Option<&str>,
    ) -> AppResult<LiveOrderRecord> {
        Err(AppError::Exchange(
            "live order cancel adapter is not configured".to_string(),
        ))
    }

    async fn query_order(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _symbol: Symbol,
        _orig_client_order_id: Option<&str>,
        _order_id: Option<&str>,
    ) -> AppResult<Option<LiveOrderRecord>> {
        Err(AppError::Exchange(
            "live order query adapter is not configured".to_string(),
        ))
    }

    async fn list_open_orders(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _symbol: Symbol,
    ) -> AppResult<Vec<LiveOrderRecord>> {
        Err(AppError::Exchange(
            "live open-order query adapter is not configured".to_string(),
        ))
    }

    async fn list_user_trades(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
        _symbol: Symbol,
        _limit: usize,
    ) -> AppResult<Vec<LiveFillRecord>> {
        Err(AppError::Exchange(
            "live trade query adapter is not configured".to_string(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct UnavailableSecretStore;

#[async_trait]
impl SecretStore for UnavailableSecretStore {
    async fn store(&self, _id: &LiveCredentialId, _secret: &LiveCredentialSecret) -> AppResult<()> {
        Err(AppError::SecureStoreUnavailable(
            "secure storage adapter is not configured".to_string(),
        ))
    }

    async fn read(&self, _id: &LiveCredentialId) -> AppResult<LiveCredentialSecret> {
        Err(AppError::SecureStoreUnavailable(
            "secure storage adapter is not configured".to_string(),
        ))
    }

    async fn delete(&self, _id: &LiveCredentialId) -> AppResult<()> {
        Err(AppError::SecureStoreUnavailable(
            "secure storage adapter is not configured".to_string(),
        ))
    }

    async fn ensure_available(&self) -> AppResult<()> {
        Err(AppError::SecureStoreUnavailable(
            "secure storage adapter is not configured".to_string(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct UnavailableLiveExchange;

#[async_trait]
impl LiveExchangePort for UnavailableLiveExchange {
    async fn validate_credentials(
        &self,
        environment: LiveEnvironment,
        credential_id: &LiveCredentialId,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveCredentialValidationResult> {
        Ok(LiveCredentialValidationResult {
            credential_id: credential_id.clone(),
            environment,
            status: relxen_domain::LiveCredentialValidationStatus::NetworkError,
            validated_at: 0,
            message: Some("live exchange adapter is not configured".to_string()),
        })
    }

    async fn fetch_account_snapshot(
        &self,
        _environment: LiveEnvironment,
        _secret: &LiveCredentialSecret,
    ) -> AppResult<LiveAccountSnapshot> {
        Err(AppError::Exchange(
            "live exchange adapter is not configured".to_string(),
        ))
    }

    async fn fetch_symbol_rules(
        &self,
        _environment: LiveEnvironment,
        _symbol: Symbol,
    ) -> AppResult<LiveSymbolRules> {
        Err(AppError::Exchange(
            "live exchange adapter is not configured".to_string(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct NoopPublisher;

impl EventPublisher for NoopPublisher {
    fn publish(&self, _event: OutboundEvent) {}
}
