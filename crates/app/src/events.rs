use serde::{Deserialize, Serialize};

use relxen_domain::{
    AsoPoint, Candle, ConnectionState, LiveAccountShadow, LiveAutoExecutorStatus,
    LiveExecutionSnapshot, LiveFillRecord, LiveKillSwitchState, LiveOrderPreflightResult,
    LiveOrderPreview, LiveOrderRecord, LiveReconciliationStatus, LiveStatusSnapshot, LogEvent,
    PerformanceStats, Position, Settings, SignalEvent, Symbol, SystemMetrics, Trade, Wallet,
    ALLOWED_SYMBOLS,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppMetadata {
    pub app_name: String,
    pub version: String,
    pub started_at: i64,
}

impl Default for AppMetadata {
    fn default() -> Self {
        Self {
            app_name: "RelXen".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: crate::service::now_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapPayload {
    pub metadata: AppMetadata,
    pub runtime_status: relxen_domain::RuntimeStatus,
    pub settings: Settings,
    pub allowed_symbols: Vec<Symbol>,
    pub active_symbol: Symbol,
    pub candles: Vec<Candle>,
    pub aso_points: Vec<AsoPoint>,
    pub recent_signals: Vec<SignalEvent>,
    pub recent_trades: Vec<Trade>,
    pub current_position: Option<Position>,
    pub wallets: Vec<Wallet>,
    pub performance: PerformanceStats,
    pub connection_state: ConnectionState,
    pub live_status: LiveStatusSnapshot,
    pub system_metrics: SystemMetrics,
    pub recent_logs: Vec<LogEvent>,
}

impl BootstrapPayload {
    pub fn allowed_symbols() -> Vec<Symbol> {
        ALLOWED_SYMBOLS.to_vec()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum OutboundEvent {
    Snapshot(Box<BootstrapPayload>),
    CandlePartial(Candle),
    CandleClosed(Candle),
    AsoUpdated(AsoPoint),
    SignalEmitted(SignalEvent),
    TradeAppended(Trade),
    TradeHistoryReset,
    PositionUpdated(Option<Position>),
    WalletUpdated(Vec<Wallet>),
    PerformanceUpdated(PerformanceStats),
    ConnectionChanged(ConnectionState),
    RuntimeChanged(relxen_domain::RuntimeStatus),
    LiveStatusUpdated(Box<LiveStatusSnapshot>),
    LiveShadowStatusUpdated(Box<LiveReconciliationStatus>),
    LiveShadowAccountUpdated(Box<LiveAccountShadow>),
    LiveIntentPreviewUpdated(Box<LiveOrderPreview>),
    LivePreflightResultAppended(Box<LiveOrderPreflightResult>),
    LiveExecutionStateUpdated(Box<LiveExecutionSnapshot>),
    LiveExecutionBlocked { reason: String },
    LiveKillSwitchUpdated(LiveKillSwitchState),
    LiveAutoStateUpdated(LiveAutoExecutorStatus),
    LiveExecutionDegraded { reason: String },
    LiveExecutionResynced,
    LiveMainnetGateUpdated { enabled: bool },
    LiveOrderSubmitted(Box<LiveOrderRecord>),
    LiveOrderUpdated(Box<LiveOrderRecord>),
    LiveFillAppended(Box<LiveFillRecord>),
    LiveFlattenStarted { symbol: Symbol },
    LiveFlattenFinished { message: String },
    LiveShadowDegraded { reason: String },
    LiveShadowResynced,
    LogAppended(LogEvent),
    SystemMetrics(SystemMetrics),
    ResyncRequired { reason: String },
}
