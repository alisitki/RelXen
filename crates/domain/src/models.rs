use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum QuoteAsset {
    #[serde(rename = "USDT")]
    Usdt,
    #[serde(rename = "USDC")]
    Usdc,
}

impl QuoteAsset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Usdt => "USDT",
            Self::Usdc => "USDC",
        }
    }
}

impl Display for QuoteAsset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for QuoteAsset {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "USDT" => Ok(Self::Usdt),
            "USDC" => Ok(Self::Usdc),
            _ => Err(format!("unsupported quote asset: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Symbol {
    #[serde(rename = "BTCUSDT")]
    BtcUsdt,
    #[serde(rename = "BTCUSDC")]
    BtcUsdc,
}

pub const ALLOWED_SYMBOLS: [Symbol; 2] = [Symbol::BtcUsdt, Symbol::BtcUsdc];

impl Symbol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BtcUsdt => "BTCUSDT",
            Self::BtcUsdc => "BTCUSDC",
        }
    }

    pub const fn quote_asset(self) -> QuoteAsset {
        match self {
            Self::BtcUsdt => QuoteAsset::Usdt,
            Self::BtcUsdc => QuoteAsset::Usdc,
        }
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Symbol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "BTCUSDT" => Ok(Self::BtcUsdt),
            "BTCUSDC" => Ok(Self::BtcUsdc),
            _ => Err(format!("unsupported symbol: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timeframe {
    #[serde(rename = "1m")]
    M1,
    #[serde(rename = "5m")]
    M5,
    #[serde(rename = "15m")]
    M15,
    #[serde(rename = "1h")]
    H1,
}

impl Timeframe {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
        }
    }

    pub const fn duration_ms(self) -> i64 {
        match self {
            Self::M1 => 60_000,
            Self::M5 => 300_000,
            Self::M15 => 900_000,
            Self::H1 => 3_600_000,
        }
    }

    pub fn align_open_time(self, timestamp: i64) -> i64 {
        let duration = self.duration_ms();
        timestamp - timestamp.rem_euclid(duration)
    }

    pub fn close_time_for_open(self, open_time: i64) -> i64 {
        open_time + self.duration_ms() - 1
    }

    pub fn next_open_time(self, open_time: i64) -> i64 {
        open_time + self.duration_ms()
    }

    pub fn count_open_times_between(self, start_open_time: i64, end_open_time: i64) -> usize {
        if end_open_time < start_open_time {
            return 0;
        }

        ((end_open_time - start_open_time) / self.duration_ms() + 1) as usize
    }
}

impl Display for Timeframe {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Timeframe {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "1m" => Ok(Self::M1),
            "5m" => Ok(Self::M5),
            "15m" => Ok(Self::M15),
            "1h" => Ok(Self::H1),
            _ => Err(format!("unsupported timeframe: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsoMode {
    Intrabar,
    Group,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAction {
    Open,
    Close,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeSource {
    Signal,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizingMode {
    FixedNotional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Paper,
    LiveLocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveModePreference {
    Paper,
    LiveReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveEnvironment {
    Testnet,
    Mainnet,
}

impl LiveEnvironment {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
        }
    }
}

impl Display for LiveEnvironment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LiveEnvironment {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "testnet" => Ok(Self::Testnet),
            "mainnet" => Ok(Self::Mainnet),
            _ => Err(format!("unsupported live environment: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveMarginType {
    Cross,
    Isolated,
    #[default]
    Unknown,
}

impl LiveMarginType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cross => "cross",
            Self::Isolated => "isolated",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_exchange_str(value: Option<&str>) -> Self {
        match value.map(|value| value.trim().to_ascii_lowercase()) {
            Some(value) if value == "cross" || value == "crossed" => Self::Cross,
            Some(value) if value == "isolated" => Self::Isolated,
            _ => Self::Unknown,
        }
    }
}

impl Display for LiveMarginType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LiveMarginType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cross" | "crossed" => Ok(Self::Cross),
            "isolated" => Ok(Self::Isolated),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("unsupported live margin type: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LiveCredentialId(pub String);

impl LiveCredentialId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for LiveCredentialId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveCredentialValidationStatus {
    Unknown,
    Valid,
    InvalidApiKey,
    InvalidSignature,
    PermissionDenied,
    TimestampSkew,
    EnvironmentMismatch,
    NetworkError,
    ExchangeError,
    ResponseDecodeError,
    SecureStoreUnavailable,
}

impl LiveCredentialValidationStatus {
    pub const fn is_valid(self) -> bool {
        matches!(self, Self::Valid)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Valid => "valid",
            Self::InvalidApiKey => "invalid_api_key",
            Self::InvalidSignature => "invalid_signature",
            Self::PermissionDenied => "permission_denied",
            Self::TimestampSkew => "timestamp_skew",
            Self::EnvironmentMismatch => "environment_mismatch",
            Self::NetworkError => "network_error",
            Self::ExchangeError => "exchange_error",
            Self::ResponseDecodeError => "response_decode_error",
            Self::SecureStoreUnavailable => "secure_store_unavailable",
        }
    }
}

impl FromStr for LiveCredentialValidationStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "unknown" => Ok(Self::Unknown),
            "valid" => Ok(Self::Valid),
            "invalid_api_key" => Ok(Self::InvalidApiKey),
            "invalid_signature" => Ok(Self::InvalidSignature),
            "permission_denied" => Ok(Self::PermissionDenied),
            "timestamp_skew" => Ok(Self::TimestampSkew),
            "environment_mismatch" => Ok(Self::EnvironmentMismatch),
            "network_error" => Ok(Self::NetworkError),
            "exchange_error" => Ok(Self::ExchangeError),
            "response_decode_error" => Ok(Self::ResponseDecodeError),
            "secure_store_unavailable" => Ok(Self::SecureStoreUnavailable),
            _ => Err(format!("unsupported validation status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveCredentialSource {
    SecureStore,
    Env,
}

impl LiveCredentialSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SecureStore => "secure_store",
            Self::Env => "env",
        }
    }
}

impl FromStr for LiveCredentialSource {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "secure_store" => Ok(Self::SecureStore),
            "env" => Ok(Self::Env),
            _ => Err(format!("unsupported credential source: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveCredentialSummary {
    pub id: LiveCredentialId,
    pub alias: String,
    pub environment: LiveEnvironment,
    pub source: LiveCredentialSource,
    pub api_key_hint: String,
    pub validation_status: LiveCredentialValidationStatus,
    pub last_validated_at: Option<i64>,
    pub last_validation_error: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

pub type LiveCredentialMetadata = LiveCredentialSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveCredentialValidationResult {
    pub credential_id: LiveCredentialId,
    pub environment: LiveEnvironment,
    pub status: LiveCredentialValidationStatus,
    pub validated_at: i64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveRuntimeState {
    Disabled,
    CredentialsMissing,
    SecureStoreUnavailable,
    ValidationMissing,
    ValidationPending,
    ValidationFailed,
    RulesUnavailable,
    AccountSnapshotUnavailable,
    NotReady,
    ReadyReadOnly,
    ArmedReadOnly,
    ShadowStarting,
    ShadowSyncing,
    ShadowRunning,
    ShadowDegraded,
    PreflightReady,
    PreflightBlocked,
    TestnetExecutionReady,
    TestnetAutoReady,
    TestnetAutoRunning,
    TestnetSubmitPending,
    TestnetOrderOpen,
    TestnetPartiallyFilled,
    TestnetFilled,
    TestnetCancelPending,
    TestnetFlattenPending,
    ExecutionDegraded,
    ExecutionBlocked,
    MainnetExecutionBlocked,
    MainnetCanaryReady,
    MainnetManualExecutionEnabled,
    KillSwitchEngaged,
    StartBlocked,
    ExecutionNotImplemented,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoState {
    Disabled,
    ConfigBlocked,
    CredentialsMissing,
    ValidationMissing,
    ReadinessBlocked,
    RiskProfileMissing,
    DryRunReady,
    DryRunRunning,
    Armed,
    LiveReady,
    LiveRunning,
    Stopping,
    Stopped,
    WatchdogStopped,
    KillSwitchEngaged,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoRunMode {
    DryRun,
    Live,
}

impl MainnetAutoRunMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry_run",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoStopReason {
    KillSwitchEngaged,
    MarketDataStale,
    ShadowStale,
    UserDataStreamDown,
    AccountSnapshotStale,
    ReferencePriceStale,
    RulesStale,
    OrderReconciliationAmbiguous,
    UnexpectedOpenOrder,
    UnexpectedPosition,
    MaxRuntimeReached,
    MaxOrdersReached,
    MaxFillsReached,
    MaxLossReached,
    MaxRejectionsReached,
    DuplicateSignalDetected,
    LiveStartBlocked,
    LeverageAboveMax,
    UnsupportedAccountMode,
    UnsupportedMarginMode,
    EvidenceLoggingFailed,
    LessonReportFailed,
    MainnetAutoConfigDisabled,
    OperatorStop,
}

impl MainnetAutoStopReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KillSwitchEngaged => "kill_switch_engaged",
            Self::MarketDataStale => "market_data_stale",
            Self::ShadowStale => "shadow_stale",
            Self::UserDataStreamDown => "user_data_stream_down",
            Self::AccountSnapshotStale => "account_snapshot_stale",
            Self::ReferencePriceStale => "reference_price_stale",
            Self::RulesStale => "rules_stale",
            Self::OrderReconciliationAmbiguous => "order_reconciliation_ambiguous",
            Self::UnexpectedOpenOrder => "unexpected_open_order",
            Self::UnexpectedPosition => "unexpected_position",
            Self::MaxRuntimeReached => "max_runtime_reached",
            Self::MaxOrdersReached => "max_orders_reached",
            Self::MaxFillsReached => "max_fills_reached",
            Self::MaxLossReached => "max_loss_reached",
            Self::MaxRejectionsReached => "max_rejections_reached",
            Self::DuplicateSignalDetected => "duplicate_signal_detected",
            Self::LiveStartBlocked => "live_start_blocked",
            Self::LeverageAboveMax => "leverage_above_max",
            Self::UnsupportedAccountMode => "unsupported_account_mode",
            Self::UnsupportedMarginMode => "unsupported_margin_mode",
            Self::EvidenceLoggingFailed => "evidence_logging_failed",
            Self::LessonReportFailed => "lesson_report_failed",
            Self::MainnetAutoConfigDisabled => "mainnet_auto_config_disabled",
            Self::OperatorStop => "operator_stop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoDecisionOutcome {
    SignalSeen,
    SkippedUnfinishedCandle,
    SkippedDuplicate,
    SkippedStaleMarketData,
    SkippedStalePreview,
    SkippedStaleReferencePrice,
    SkippedStaleShadow,
    SkippedRiskBudget,
    SkippedOpenOrder,
    SkippedOpenPosition,
    SkippedKillSwitch,
    SkippedAutoDisabled,
    SkippedConfigBlocked,
    DryRunWouldSubmit,
    LiveSubmitRequested,
    LiveSubmitBlocked,
    WatchdogStopped,
}

impl MainnetAutoDecisionOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SignalSeen => "signal_seen",
            Self::SkippedUnfinishedCandle => "skipped_unfinished_candle",
            Self::SkippedDuplicate => "skipped_duplicate",
            Self::SkippedStaleMarketData => "skipped_stale_market_data",
            Self::SkippedStalePreview => "skipped_stale_preview",
            Self::SkippedStaleReferencePrice => "skipped_stale_reference_price",
            Self::SkippedStaleShadow => "skipped_stale_shadow",
            Self::SkippedRiskBudget => "skipped_risk_budget",
            Self::SkippedOpenOrder => "skipped_open_order",
            Self::SkippedOpenPosition => "skipped_open_position",
            Self::SkippedKillSwitch => "skipped_kill_switch",
            Self::SkippedAutoDisabled => "skipped_auto_disabled",
            Self::SkippedConfigBlocked => "skipped_config_blocked",
            Self::DryRunWouldSubmit => "dry_run_would_submit",
            Self::LiveSubmitRequested => "live_submit_requested",
            Self::LiveSubmitBlocked => "live_submit_blocked",
            Self::WatchdogStopped => "watchdog_stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveBlockingReason {
    NoActiveCredential,
    EnvCredentialsMissing,
    EnvCredentialPartial,
    SecureStoreUnavailable,
    ValidationFailed,
    ValidationMissing,
    ValidationStale,
    AccountSnapshotMissing,
    SymbolRulesMissing,
    RulesMissing,
    UnsupportedSymbol,
    UnsupportedTimeframe,
    UnsupportedAccountMode,
    PaperPositionOpen,
    RuntimeBusy,
    ShadowStreamDown,
    ShadowStateAmbiguous,
    PreflightNotSupportedOnMainnet,
    IntentUnavailable,
    MinNotional,
    PrecisionInvalid,
    MainnetExecutionBlocked,
    MainnetCanaryDisabled,
    MainnetCanaryRiskProfileMissing,
    MainnetCanaryLimitRequired,
    MainnetCanaryLimitMarketable,
    MainnetConfirmationMissing,
    MainnetAutoBlocked,
    ReferencePriceUnavailable,
    ReferencePriceStale,
    ReferencePriceSourceFailed,
    StaleShadowState,
    PreviewMismatch,
    ExecutionStatusUnknown,
    DuplicateClientOrderId,
    OrderRejected,
    OrderNotFound,
    CancelFailed,
    FlattenFailed,
    KillSwitchEngaged,
    RiskLimitExceeded,
    AutoExecutorStopped,
    DuplicateSignalSuppressed,
    RecentWindowRepairOnly,
    ExecutionNotImplemented,
}

impl LiveBlockingReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveCredential => "no_active_credential",
            Self::EnvCredentialsMissing => "env_credentials_missing",
            Self::EnvCredentialPartial => "env_credential_partial",
            Self::SecureStoreUnavailable => "secure_store_unavailable",
            Self::ValidationFailed => "validation_failed",
            Self::ValidationMissing => "validation_missing",
            Self::ValidationStale => "validation_stale",
            Self::AccountSnapshotMissing => "account_snapshot_missing",
            Self::SymbolRulesMissing => "symbol_rules_missing",
            Self::RulesMissing => "rules_missing",
            Self::UnsupportedSymbol => "unsupported_symbol",
            Self::UnsupportedTimeframe => "unsupported_timeframe",
            Self::UnsupportedAccountMode => "unsupported_account_mode",
            Self::PaperPositionOpen => "paper_position_open",
            Self::RuntimeBusy => "runtime_busy",
            Self::ShadowStreamDown => "shadow_stream_down",
            Self::ShadowStateAmbiguous => "shadow_state_ambiguous",
            Self::PreflightNotSupportedOnMainnet => "preflight_not_supported_on_mainnet",
            Self::IntentUnavailable => "intent_unavailable",
            Self::MinNotional => "min_notional",
            Self::PrecisionInvalid => "precision_invalid",
            Self::MainnetExecutionBlocked => "mainnet_execution_blocked",
            Self::MainnetCanaryDisabled => "mainnet_canary_disabled",
            Self::MainnetCanaryRiskProfileMissing => "mainnet_canary_risk_profile_missing",
            Self::MainnetCanaryLimitRequired => "mainnet_canary_limit_required",
            Self::MainnetCanaryLimitMarketable => "mainnet_canary_limit_marketable",
            Self::MainnetConfirmationMissing => "mainnet_confirmation_missing",
            Self::MainnetAutoBlocked => "mainnet_auto_blocked",
            Self::ReferencePriceUnavailable => "reference_price_unavailable",
            Self::ReferencePriceStale => "reference_price_stale",
            Self::ReferencePriceSourceFailed => "reference_price_source_failed",
            Self::StaleShadowState => "stale_shadow_state",
            Self::PreviewMismatch => "preview_mismatch",
            Self::ExecutionStatusUnknown => "execution_status_unknown",
            Self::DuplicateClientOrderId => "duplicate_client_order_id",
            Self::OrderRejected => "order_rejected",
            Self::OrderNotFound => "order_not_found",
            Self::CancelFailed => "cancel_failed",
            Self::FlattenFailed => "flatten_failed",
            Self::KillSwitchEngaged => "kill_switch_engaged",
            Self::RiskLimitExceeded => "risk_limit_exceeded",
            Self::AutoExecutorStopped => "auto_executor_stopped",
            Self::DuplicateSignalSuppressed => "duplicate_signal_suppressed",
            Self::RecentWindowRepairOnly => "recent_window_repair_only",
            Self::ExecutionNotImplemented => "execution_not_implemented",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveWarning {
    ValidationStale,
    RulesSnapshotStale,
    AccountSnapshotStale,
    ShadowSnapshotStale,
    ShadowStreamStale,
    TestnetEnvironment,
    OpenExchangePositionDetected,
    UnsupportedExchangeMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveGateCheck {
    pub code: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutionAvailability {
    pub can_execute_live: bool,
    pub reason: LiveBlockingReason,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAssetBalance {
    pub asset: String,
    pub wallet_balance: f64,
    pub available_balance: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LivePositionSnapshot {
    pub symbol: Symbol,
    pub position_side: String,
    pub position_amt: f64,
    pub entry_price: f64,
    pub mark_price: Option<f64>,
    pub unrealized_pnl: f64,
    pub leverage: Option<f64>,
    #[serde(default)]
    pub margin_type: LiveMarginType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAccountSnapshot {
    pub environment: LiveEnvironment,
    pub can_trade: bool,
    pub multi_assets_margin: Option<bool>,
    #[serde(default)]
    pub position_mode: Option<String>,
    #[serde(default)]
    pub account_mode_checked_at: Option<i64>,
    pub total_wallet_balance: f64,
    pub total_margin_balance: f64,
    pub available_balance: f64,
    pub assets: Vec<LiveAssetBalance>,
    pub positions: Vec<LivePositionSnapshot>,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAccountModeStatus {
    pub environment: LiveEnvironment,
    pub position_mode: Option<String>,
    pub multi_assets_margin: Option<bool>,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveSymbolFilterSummary {
    pub tick_size: Option<f64>,
    pub step_size: Option<f64>,
    pub min_qty: Option<f64>,
    pub min_notional: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveSymbolRules {
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub status: String,
    pub base_asset: String,
    pub quote_asset: QuoteAsset,
    pub price_precision: i64,
    pub quantity_precision: i64,
    pub filters: LiveSymbolFilterSummary,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveShadowStreamState {
    Stopped,
    Starting,
    Connecting,
    Syncing,
    Running,
    Reconnecting,
    Degraded,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveShadowStreamStatus {
    pub state: LiveShadowStreamState,
    pub environment: LiveEnvironment,
    pub listen_key_hint: Option<String>,
    pub status_since: i64,
    pub started_at: Option<i64>,
    pub last_event_time: Option<i64>,
    pub last_rest_sync_at: Option<i64>,
    pub reconnect_attempts: u64,
    pub stale: bool,
    pub detail: Option<String>,
}

impl Default for LiveShadowStreamStatus {
    fn default() -> Self {
        Self {
            state: LiveShadowStreamState::Stopped,
            environment: LiveEnvironment::Testnet,
            listen_key_hint: None,
            status_since: 0,
            started_at: None,
            last_event_time: None,
            last_rest_sync_at: None,
            reconnect_attempts: 0,
            stale: false,
            detail: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveShadowBalance {
    pub asset: String,
    pub wallet_balance: String,
    pub cross_wallet_balance: Option<String>,
    pub balance_change: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveShadowPosition {
    pub symbol: Symbol,
    pub position_side: String,
    pub position_amt: String,
    pub entry_price: String,
    pub unrealized_pnl: String,
    pub margin_type: Option<String>,
    pub isolated_wallet: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveShadowOrder {
    pub order_id: String,
    pub client_order_id: Option<String>,
    pub symbol: Symbol,
    pub side: LiveOrderSide,
    pub order_type: LiveOrderType,
    pub time_in_force: Option<String>,
    pub original_qty: String,
    pub executed_qty: String,
    pub price: Option<String>,
    pub avg_price: Option<String>,
    pub status: String,
    pub execution_type: Option<String>,
    pub reduce_only: bool,
    pub position_side: Option<String>,
    #[serde(default)]
    pub last_filled_qty: Option<String>,
    #[serde(default)]
    pub last_filled_price: Option<String>,
    #[serde(default)]
    pub commission: Option<String>,
    #[serde(default)]
    pub commission_asset: Option<String>,
    #[serde(default)]
    pub trade_id: Option<String>,
    #[serde(default)]
    pub self_trade_prevention_mode: Option<String>,
    #[serde(default)]
    pub price_match: Option<String>,
    #[serde(default)]
    pub expire_reason: Option<String>,
    pub last_update_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAccountShadow {
    pub environment: LiveEnvironment,
    pub balances: Vec<LiveShadowBalance>,
    pub positions: Vec<LiveShadowPosition>,
    pub open_orders: Vec<LiveShadowOrder>,
    pub can_trade: bool,
    pub multi_assets_margin: Option<bool>,
    pub position_mode: Option<String>,
    pub last_event_time: Option<i64>,
    pub last_rest_sync_at: Option<i64>,
    pub updated_at: i64,
    pub ambiguous: bool,
    pub divergence_reasons: Vec<LiveBlockingReason>,
}

impl Default for LiveAccountShadow {
    fn default() -> Self {
        Self {
            environment: LiveEnvironment::Testnet,
            balances: Vec::new(),
            positions: Vec::new(),
            open_orders: Vec::new(),
            can_trade: false,
            multi_assets_margin: None,
            position_mode: None,
            last_event_time: None,
            last_rest_sync_at: None,
            updated_at: 0,
            ambiguous: true,
            divergence_reasons: vec![LiveBlockingReason::AccountSnapshotMissing],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveReconciliationStatus {
    pub state: LiveRuntimeState,
    pub stream: LiveShadowStreamStatus,
    pub shadow: Option<LiveAccountShadow>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub warnings: Vec<LiveWarning>,
    pub updated_at: i64,
}

impl Default for LiveReconciliationStatus {
    fn default() -> Self {
        Self {
            state: LiveRuntimeState::CredentialsMissing,
            stream: LiveShadowStreamStatus::default(),
            shadow: None,
            blocking_reasons: Vec::new(),
            warnings: Vec::new(),
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum LiveUserDataEvent {
    AccountUpdate(LiveAccountShadow),
    OrderTradeUpdate(Box<LiveShadowOrder>),
    AccountConfigUpdate {
        event_time: i64,
        position_mode: Option<String>,
        leverage_symbol: Option<Symbol>,
        leverage: Option<i64>,
    },
    ListenKeyExpired {
        event_time: i64,
    },
    Unknown {
        event_type: String,
        event_time: Option<i64>,
    },
}

impl LiveUserDataEvent {
    pub fn event_time(&self) -> Option<i64> {
        match self {
            Self::AccountUpdate(shadow) => shadow.last_event_time,
            Self::OrderTradeUpdate(order) => Some(order.last_update_time),
            Self::AccountConfigUpdate { event_time, .. } => Some(*event_time),
            Self::ListenKeyExpired { event_time } => Some(*event_time),
            Self::Unknown { event_time, .. } => *event_time,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveOrderSide {
    Buy,
    Sell,
}

impl LiveOrderSide {
    pub const fn as_binance(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveOrderType {
    Market,
    Limit,
}

impl LiveOrderType {
    pub const fn as_binance(self) -> &'static str {
        match self {
            Self::Market => "MARKET",
            Self::Limit => "LIMIT",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveOrderSizingBreakdown {
    pub requested_notional: String,
    pub available_balance: String,
    pub leverage: String,
    pub required_margin: String,
    pub raw_quantity: String,
    pub rounded_quantity: String,
    pub estimated_notional: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveOrderIntent {
    pub id: String,
    #[serde(default)]
    pub intent_hash: String,
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub side: LiveOrderSide,
    pub order_type: LiveOrderType,
    pub quantity: String,
    pub price: Option<String>,
    pub reduce_only: bool,
    pub time_in_force: Option<String>,
    pub source_signal_id: Option<String>,
    pub source_open_time: Option<i64>,
    pub reason: String,
    pub exchange_payload: BTreeMap<String, String>,
    pub sizing: LiveOrderSizingBreakdown,
    pub validation_notes: Vec<String>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub can_preflight: bool,
    pub can_execute_now: bool,
    pub built_at: i64,
}

pub type ExecutionEnvironment = LiveEnvironment;
pub type LiveExecutionMode = LiveModePreference;
pub type LiveExecutionBlockingReason = LiveBlockingReason;
pub type LiveExecutionWarning = LiveWarning;
pub type LiveIntentHash = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveExecutionState {
    Disabled,
    CredentialsMissing,
    ValidationMissing,
    ValidationFailed,
    ShadowOnly,
    PreflightReady,
    TestnetExecutionReady,
    TestnetAutoReady,
    TestnetAutoRunning,
    TestnetSubmitPending,
    TestnetOrderOpen,
    TestnetPartiallyFilled,
    TestnetFilled,
    TestnetCancelPending,
    TestnetFlattenPending,
    ExecutionDegraded,
    ExecutionBlocked,
    MainnetExecutionBlocked,
    MainnetCanaryReady,
    MainnetManualExecutionEnabled,
    KillSwitchEngaged,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveOrderStatus {
    LocalCreated,
    SubmitPending,
    Accepted,
    Working,
    PartiallyFilled,
    Filled,
    CancelPending,
    Canceled,
    Rejected,
    Expired,
    ExpiredInMatch,
    UnknownNeedsRepair,
}

impl LiveOrderStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalCreated => "local_created",
            Self::SubmitPending => "submit_pending",
            Self::Accepted => "accepted",
            Self::Working => "working",
            Self::PartiallyFilled => "partially_filled",
            Self::Filled => "filled",
            Self::CancelPending => "cancel_pending",
            Self::Canceled => "canceled",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::ExpiredInMatch => "expired_in_match",
            Self::UnknownNeedsRepair => "unknown_needs_repair",
        }
    }

    pub const fn is_open(self) -> bool {
        matches!(
            self,
            Self::LocalCreated
                | Self::SubmitPending
                | Self::Accepted
                | Self::Working
                | Self::PartiallyFilled
                | Self::CancelPending
                | Self::UnknownNeedsRepair
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveOrderRecord {
    pub id: String,
    pub credential_id: Option<LiveCredentialId>,
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub side: LiveOrderSide,
    pub order_type: LiveOrderType,
    pub status: LiveOrderStatus,
    pub client_order_id: String,
    pub exchange_order_id: Option<String>,
    pub quantity: String,
    pub price: Option<String>,
    pub executed_qty: String,
    pub avg_price: Option<String>,
    pub reduce_only: bool,
    pub time_in_force: Option<String>,
    pub intent_id: Option<String>,
    pub intent_hash: Option<String>,
    pub source_signal_id: Option<String>,
    #[serde(default)]
    pub source_open_time: Option<i64>,
    pub reason: String,
    pub payload: BTreeMap<String, String>,
    #[serde(default)]
    pub response_type: Option<String>,
    #[serde(default)]
    pub self_trade_prevention_mode: Option<String>,
    #[serde(default)]
    pub price_match: Option<String>,
    #[serde(default)]
    pub expire_reason: Option<String>,
    pub last_error: Option<String>,
    pub submitted_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveFillRecord {
    pub id: String,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
    pub exchange_order_id: Option<String>,
    pub symbol: Symbol,
    pub side: LiveOrderSide,
    pub quantity: String,
    pub price: String,
    pub commission: Option<String>,
    pub commission_asset: Option<String>,
    pub realized_pnl: Option<String>,
    pub trade_id: Option<String>,
    pub event_time: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutionSnapshot {
    pub state: LiveExecutionState,
    pub environment: LiveEnvironment,
    pub can_submit: bool,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub warnings: Vec<LiveWarning>,
    pub active_order: Option<LiveOrderRecord>,
    pub recent_orders: Vec<LiveOrderRecord>,
    pub recent_fills: Vec<LiveFillRecord>,
    pub kill_switch_engaged: bool,
    #[serde(default)]
    pub repair_recent_window_only: bool,
    #[serde(default)]
    pub mainnet_canary_enabled: bool,
    pub updated_at: i64,
}

impl Default for LiveExecutionSnapshot {
    fn default() -> Self {
        Self {
            state: LiveExecutionState::CredentialsMissing,
            environment: LiveEnvironment::Testnet,
            can_submit: false,
            blocking_reasons: vec![LiveBlockingReason::NoActiveCredential],
            warnings: Vec::new(),
            active_order: None,
            recent_orders: Vec::new(),
            recent_fills: Vec::new(),
            kill_switch_engaged: false,
            repair_recent_window_only: true,
            mainnet_canary_enabled: false,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutionRequest {
    pub intent_id: Option<String>,
    pub confirm_testnet: bool,
    #[serde(default)]
    pub confirm_mainnet_canary: bool,
    #[serde(default)]
    pub confirmation_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutionResult {
    pub accepted: bool,
    pub order: Option<LiveOrderRecord>,
    pub blocking_reason: Option<LiveBlockingReason>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveCancelRequest {
    pub order_ref: String,
    pub confirm_testnet: bool,
    #[serde(default)]
    pub confirm_mainnet_canary: bool,
    #[serde(default)]
    pub confirmation_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveCancelAllRequest {
    pub confirm_testnet: bool,
    #[serde(default)]
    pub confirm_mainnet_canary: bool,
    #[serde(default)]
    pub confirmation_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveCancelResult {
    pub accepted: bool,
    pub order: Option<LiveOrderRecord>,
    pub blocking_reason: Option<LiveBlockingReason>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveFlattenRequest {
    pub confirm_testnet: bool,
    #[serde(default)]
    pub confirm_mainnet_canary: bool,
    #[serde(default)]
    pub confirmation_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveFlattenResult {
    pub accepted: bool,
    pub canceled_orders: Vec<LiveOrderRecord>,
    pub flatten_order: Option<LiveOrderRecord>,
    pub blocking_reason: Option<LiveBlockingReason>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutableIntent {
    pub intent: LiveOrderIntent,
    pub intent_hash: LiveIntentHash,
    pub payload: BTreeMap<String, String>,
    pub can_execute: bool,
    pub blocking_reasons: Vec<LiveBlockingReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveReferencePriceSnapshot {
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub price: Option<String>,
    pub source: Option<String>,
    pub observed_at: Option<i64>,
    pub fetched_at: Option<i64>,
    pub age_ms: Option<i64>,
    pub stale: bool,
    pub failure_reason: Option<String>,
    pub blocking_reason: Option<LiveBlockingReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveMarketabilityCheck {
    pub reference_price: Option<String>,
    pub reference_price_source: Option<String>,
    pub reference_price_age_ms: Option<i64>,
    pub rounded_order_price: Option<String>,
    pub marketable_after_rounding: Option<bool>,
    pub checked_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveIntentExecutionReadiness {
    pub can_execute: bool,
    pub intent_hash: Option<LiveIntentHash>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub staleness_reasons: Vec<LiveIntentStalenessReason>,
    pub checked_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveIntentStalenessReason {
    PreviewMissing,
    PreviewTooOld,
    IntentIdMismatch,
    LiveStateChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutionPreviewBridgeResult {
    pub readiness: LiveIntentExecutionReadiness,
    pub executable: Option<LiveExecutableIntent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveExecutionAuditEvent {
    pub id: String,
    pub event_type: String,
    pub order_id: Option<String>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveOrderPreview {
    pub built_at: i64,
    pub intent: Option<LiveOrderIntent>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub validation_errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price: Option<LiveReferencePriceSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketability_check: Option<LiveMarketabilityCheck>,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LiveKillSwitchState {
    pub engaged: bool,
    pub reason: Option<String>,
    pub engaged_at: Option<i64>,
    pub released_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveRiskLimits {
    pub max_notional_per_order: String,
    pub max_open_notional_active_symbol: String,
    pub max_leverage: String,
    pub max_orders_per_session: u64,
    pub max_fills_per_session: u64,
    pub max_consecutive_rejections: u64,
    pub max_daily_realized_loss: String,
}

impl Default for LiveRiskLimits {
    fn default() -> Self {
        Self {
            max_notional_per_order: "50".to_string(),
            max_open_notional_active_symbol: "50".to_string(),
            max_leverage: "3".to_string(),
            max_orders_per_session: 5,
            max_fills_per_session: 10,
            max_consecutive_rejections: 2,
            max_daily_realized_loss: "25".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LiveRiskProfile {
    pub configured: bool,
    pub profile_name: Option<String>,
    pub limits: LiveRiskLimits,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveAutoExecutorStateKind {
    Stopped,
    Ready,
    Running,
    Blocked,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAutoExecutorStatus {
    pub state: LiveAutoExecutorStateKind,
    pub environment: LiveEnvironment,
    pub order_type: LiveOrderType,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub last_signal_id: Option<String>,
    pub last_signal_open_time: Option<i64>,
    pub last_intent_hash: Option<String>,
    pub last_order_id: Option<String>,
    pub last_message: Option<String>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub updated_at: i64,
}

impl Default for LiveAutoExecutorStatus {
    fn default() -> Self {
        Self {
            state: LiveAutoExecutorStateKind::Stopped,
            environment: LiveEnvironment::Testnet,
            order_type: LiveOrderType::Market,
            started_at: None,
            stopped_at: None,
            last_signal_id: None,
            last_signal_open_time: None,
            last_intent_hash: None,
            last_order_id: None,
            last_message: None,
            blocking_reasons: vec![LiveBlockingReason::AutoExecutorStopped],
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveIntentLockStatus {
    Created,
    Submitted,
    Blocked,
    Repaired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveIntentLock {
    pub key: String,
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub signal_id: String,
    pub signal_open_time: i64,
    pub signal_side: SignalSide,
    pub intent_hash: Option<String>,
    pub order_id: Option<String>,
    pub status: LiveIntentLockStatus,
    pub block_reason: Option<LiveBlockingReason>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveMainnetCanaryStatus {
    pub enabled_by_server: bool,
    pub risk_profile_configured: bool,
    pub canary_ready: bool,
    pub manual_execution_enabled: bool,
    pub required_confirmation: Option<String>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub updated_at: i64,
}

impl Default for LiveMainnetCanaryStatus {
    fn default() -> Self {
        Self {
            enabled_by_server: false,
            risk_profile_configured: false,
            canary_ready: false,
            manual_execution_enabled: false,
            required_confirmation: None,
            blocking_reasons: vec![LiveBlockingReason::MainnetCanaryDisabled],
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveKillSwitchRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAutoExecutorRequest {
    pub confirm_testnet_auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MainnetAutoConfig {
    pub enable_live_execution: bool,
    pub mode: MainnetAutoRunMode,
    pub max_runtime_minutes: u64,
    pub max_orders: u64,
    pub max_fills: u64,
    pub max_notional: String,
    pub max_daily_loss: String,
    pub require_flat_start: bool,
    pub require_flat_stop: bool,
    pub require_manual_canary_evidence: bool,
    pub evidence_required: bool,
    pub lesson_report_required: bool,
    pub allowed_margin_type: MainnetAutoAllowedMarginType,
    pub position_policy: AsoPositionPolicy,
    pub aso_delta_threshold: String,
    pub aso_zone_threshold: String,
}

impl Default for MainnetAutoConfig {
    fn default() -> Self {
        Self {
            enable_live_execution: false,
            mode: MainnetAutoRunMode::DryRun,
            max_runtime_minutes: 15,
            max_orders: 1,
            max_fills: 1,
            max_notional: "80".to_string(),
            max_daily_loss: "5".to_string(),
            require_flat_start: true,
            require_flat_stop: true,
            require_manual_canary_evidence: true,
            evidence_required: true,
            lesson_report_required: true,
            allowed_margin_type: MainnetAutoAllowedMarginType::Isolated,
            position_policy: AsoPositionPolicy::CrossoverOnly,
            aso_delta_threshold: "5".to_string(),
            aso_zone_threshold: "55".to_string(),
        }
    }
}

pub const MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_15M: &str = "START MAINNET AUTO LIVE BTCUSDT 15M";
pub const MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_60M: &str = "START MAINNET AUTO LIVE BTCUSDT 60M";
pub const MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_OPERATOR_STOP: &str =
    "START MAINNET AUTO LIVE BTCUSDT OPERATOR STOP";
pub const MAINNET_AUTO_LIVE_CONFIRMATION_TEXT: &str = MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_15M;
pub const MAINNET_AUTO_OPERATOR_STOP_RUNTIME_MINUTES: u64 = 0;
pub const MAINNET_AUTO_ALLOWED_RUNTIME_MINUTES: [u64; 3] =
    [MAINNET_AUTO_OPERATOR_STOP_RUNTIME_MINUTES, 15, 60];

pub fn mainnet_auto_live_runtime_allowed(minutes: u64) -> bool {
    MAINNET_AUTO_ALLOWED_RUNTIME_MINUTES.contains(&minutes)
}

pub fn mainnet_auto_live_confirmation_text(minutes: u64) -> Option<&'static str> {
    match minutes {
        MAINNET_AUTO_OPERATOR_STOP_RUNTIME_MINUTES => {
            Some(MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_OPERATOR_STOP)
        }
        15 => Some(MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_15M),
        60 => Some(MAINNET_AUTO_LIVE_CONFIRMATION_TEXT_60M),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoAllowedMarginType {
    #[default]
    Isolated,
    Cross,
    Any,
}

impl MainnetAutoAllowedMarginType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::Cross => "cross",
            Self::Any => "any",
        }
    }

    pub const fn allows(self, actual: LiveMarginType) -> bool {
        match (self, actual) {
            (_, LiveMarginType::Unknown) => false,
            (Self::Any, LiveMarginType::Cross | LiveMarginType::Isolated) => true,
            (Self::Isolated, LiveMarginType::Isolated) => true,
            (Self::Cross, LiveMarginType::Cross) => true,
            _ => false,
        }
    }
}

impl Display for MainnetAutoAllowedMarginType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MainnetAutoAllowedMarginType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "isolated" => Ok(Self::Isolated),
            "cross" => Ok(Self::Cross),
            "any" => Ok(Self::Any),
            _ => Err(format!("unsupported mainnet auto margin policy: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsoPositionPolicy {
    #[default]
    CrossoverOnly,
    AlwaysInMarket,
    FlatAllowed,
}

impl AsoPositionPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CrossoverOnly => "crossover_only",
            Self::AlwaysInMarket => "always_in_market",
            Self::FlatAllowed => "flat_allowed",
        }
    }
}

impl Display for AsoPositionPolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AsoPositionPolicy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "crossover_only" | "crossover-only" => Ok(Self::CrossoverOnly),
            "always_in_market" | "always-in-market" => Ok(Self::AlwaysInMarket),
            "flat_allowed" | "flat-allowed" => Ok(Self::FlatAllowed),
            _ => Err(format!("unsupported ASO position policy: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoDesiredSide {
    Long,
    Short,
    #[default]
    None,
}

impl MainnetAutoDesiredSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
            Self::None => "none",
        }
    }

    pub const fn from_signal_side(side: SignalSide) -> Self {
        match side {
            SignalSide::Buy => Self::Long,
            SignalSide::Sell => Self::Short,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAutoPolicyAction {
    EnterLong,
    EnterShort,
    Hold,
    Close,
    Reverse,
    #[default]
    NoTrade,
    Blocked,
}

impl MainnetAutoPolicyAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EnterLong => "enter_long",
            Self::EnterShort => "enter_short",
            Self::Hold => "hold",
            Self::Close => "close",
            Self::Reverse => "reverse",
            Self::NoTrade => "no_trade",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MainnetAutoLiveStartRequest {
    pub symbol: Symbol,
    pub duration_minutes: u64,
    pub order_type: LiveOrderType,
    pub confirmation_text: String,
    #[serde(default)]
    pub allowed_margin_type: MainnetAutoAllowedMarginType,
    #[serde(default)]
    pub position_policy: AsoPositionPolicy,
    #[serde(default = "default_mainnet_auto_aso_delta_threshold")]
    pub aso_delta_threshold: String,
    #[serde(default = "default_mainnet_auto_aso_zone_threshold")]
    pub aso_zone_threshold: String,
}

pub fn default_mainnet_auto_aso_delta_threshold() -> String {
    "5".to_string()
}

pub fn default_mainnet_auto_aso_zone_threshold() -> String {
    "55".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MainnetAutoRiskBudget {
    pub configured: bool,
    pub budget_id: String,
    pub max_notional_per_order: String,
    pub max_total_session_notional: String,
    pub max_open_notional: String,
    pub max_orders_per_session: u64,
    pub max_fills_per_session: u64,
    pub max_consecutive_losses: u64,
    pub max_consecutive_rejections: u64,
    pub max_daily_realized_loss: String,
    pub max_position_age_seconds: u64,
    pub max_runtime_minutes: u64,
    pub max_leverage: String,
    pub require_flat_start: bool,
    pub require_flat_stop: bool,
    pub allowed_symbols: Vec<Symbol>,
    pub allowed_order_types: Vec<LiveOrderType>,
    pub require_fresh_reference_price: bool,
    pub require_fresh_shadow: bool,
    pub require_fresh_user_data_stream: bool,
    pub require_evidence_logging: bool,
    pub require_lessons_report: bool,
    pub updated_at: i64,
}

impl Default for MainnetAutoRiskBudget {
    fn default() -> Self {
        Self {
            configured: true,
            budget_id: "mainnet-auto-dry-run-default".to_string(),
            max_notional_per_order: "80".to_string(),
            max_total_session_notional: "80".to_string(),
            max_open_notional: "80".to_string(),
            max_orders_per_session: 1,
            max_fills_per_session: 1,
            max_consecutive_losses: 1,
            max_consecutive_rejections: 1,
            max_daily_realized_loss: "5".to_string(),
            max_position_age_seconds: 300,
            max_runtime_minutes: 15,
            max_leverage: "5".to_string(),
            require_flat_start: true,
            require_flat_stop: true,
            allowed_symbols: vec![Symbol::BtcUsdt],
            allowed_order_types: vec![LiveOrderType::Limit],
            require_fresh_reference_price: true,
            require_fresh_shadow: true,
            require_fresh_user_data_stream: true,
            require_evidence_logging: true,
            require_lessons_report: true,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MainnetAutoWatchdogStatus {
    pub running: bool,
    pub last_check_at: Option<i64>,
    pub last_stop_reason: Option<MainnetAutoStopReason>,
    pub last_message: Option<String>,
}

impl Default for MainnetAutoWatchdogStatus {
    fn default() -> Self {
        Self {
            running: false,
            last_check_at: None,
            last_stop_reason: None,
            last_message: Some("Mainnet auto watchdog is idle.".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MainnetAutoMarginPolicyStatus {
    pub allowed_margin_type: MainnetAutoAllowedMarginType,
    pub actual_margin_type: LiveMarginType,
    pub allowed: bool,
    pub blocker: Option<String>,
    pub warning: Option<String>,
}

impl MainnetAutoMarginPolicyStatus {
    pub fn evaluate(
        allowed_margin_type: MainnetAutoAllowedMarginType,
        actual_margin_type: LiveMarginType,
    ) -> Self {
        let allowed = allowed_margin_type.allows(actual_margin_type);
        let blocker = if actual_margin_type == LiveMarginType::Unknown {
            Some("margin_type_unknown".to_string())
        } else if !allowed {
            Some("margin_type_not_allowed".to_string())
        } else {
            None
        };
        let warning = if allowed && allowed_margin_type == MainnetAutoAllowedMarginType::Any {
            Some("margin_type_any_allowed".to_string())
        } else {
            None
        };
        Self {
            allowed_margin_type,
            actual_margin_type,
            allowed,
            blocker,
            warning,
        }
    }
}

impl Default for MainnetAutoMarginPolicyStatus {
    fn default() -> Self {
        Self::evaluate(
            MainnetAutoAllowedMarginType::Isolated,
            LiveMarginType::Unknown,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MainnetAutoPositionPolicyStatus {
    pub policy: AsoPositionPolicy,
    pub aso_delta_threshold: String,
    pub aso_zone_threshold: String,
    pub last_bulls: Option<f64>,
    pub last_bears: Option<f64>,
    pub last_delta: Option<f64>,
    pub last_zone: Option<f64>,
    pub desired_side: MainnetAutoDesiredSide,
    pub current_side: MainnetAutoDesiredSide,
    pub last_action: MainnetAutoPolicyAction,
    pub last_blocker: Option<String>,
    pub last_reason: Option<String>,
}

impl Default for MainnetAutoPositionPolicyStatus {
    fn default() -> Self {
        Self {
            policy: AsoPositionPolicy::CrossoverOnly,
            aso_delta_threshold: default_mainnet_auto_aso_delta_threshold(),
            aso_zone_threshold: default_mainnet_auto_aso_zone_threshold(),
            last_bulls: None,
            last_bears: None,
            last_delta: None,
            last_zone: None,
            desired_side: MainnetAutoDesiredSide::None,
            current_side: MainnetAutoDesiredSide::None,
            last_action: MainnetAutoPolicyAction::NoTrade,
            last_blocker: None,
            last_reason: Some("not_evaluated".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MainnetAutoStatus {
    pub state: MainnetAutoState,
    pub mode: MainnetAutoRunMode,
    pub config: MainnetAutoConfig,
    pub risk_budget: MainnetAutoRiskBudget,
    pub watchdog: MainnetAutoWatchdogStatus,
    pub margin_policy: MainnetAutoMarginPolicyStatus,
    pub position_policy: MainnetAutoPositionPolicyStatus,
    pub session_id: Option<String>,
    pub started_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
    pub last_signal_id: Option<String>,
    pub last_signal_open_time: Option<i64>,
    pub last_order_id: Option<String>,
    pub last_decision_id: Option<String>,
    pub last_decision_outcome: Option<MainnetAutoDecisionOutcome>,
    pub last_watchdog_stop_reason: Option<MainnetAutoStopReason>,
    pub blocking_reasons: Vec<String>,
    pub current_blockers: Vec<String>,
    pub latest_lessons_recommendation: Option<String>,
    pub live_orders_submitted: u64,
    pub dry_run_orders_submitted: u64,
    pub evidence_path: Option<String>,
    pub updated_at: i64,
}

impl Default for MainnetAutoStatus {
    fn default() -> Self {
        let config = MainnetAutoConfig::default();
        Self {
            state: MainnetAutoState::Disabled,
            mode: config.mode,
            config,
            risk_budget: MainnetAutoRiskBudget::default(),
            watchdog: MainnetAutoWatchdogStatus::default(),
            margin_policy: MainnetAutoMarginPolicyStatus::default(),
            position_policy: MainnetAutoPositionPolicyStatus::default(),
            session_id: None,
            started_at: None,
            expires_at: None,
            stopped_at: None,
            last_heartbeat_at: None,
            last_signal_id: None,
            last_signal_open_time: None,
            last_order_id: None,
            last_decision_id: None,
            last_decision_outcome: None,
            last_watchdog_stop_reason: None,
            blocking_reasons: vec!["mainnet_auto_config_disabled".to_string()],
            current_blockers: vec!["mainnet_auto_config_disabled".to_string()],
            latest_lessons_recommendation: Some("live_not_allowed".to_string()),
            live_orders_submitted: 0,
            dry_run_orders_submitted: 0,
            evidence_path: None,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MainnetAutoDecisionEvent {
    pub id: String,
    pub session_id: String,
    pub mode: MainnetAutoRunMode,
    pub outcome: MainnetAutoDecisionOutcome,
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub closed_candle_open_time: Option<i64>,
    pub signal_id: Option<String>,
    pub signal_side: Option<SignalSide>,
    pub strategy_id: String,
    pub aso_settings_snapshot: BTreeMap<String, String>,
    pub risk_budget_snapshot_id: String,
    pub reference_price_source: Option<String>,
    pub reference_price_age_ms: Option<i64>,
    pub intent_hash: Option<String>,
    pub would_submit: bool,
    pub blocking_reasons: Vec<String>,
    #[serde(default)]
    pub policy_mode: Option<AsoPositionPolicy>,
    #[serde(default)]
    pub aso_bulls: Option<f64>,
    #[serde(default)]
    pub aso_bears: Option<f64>,
    #[serde(default)]
    pub aso_delta: Option<f64>,
    #[serde(default)]
    pub aso_zone: Option<f64>,
    #[serde(default)]
    pub desired_side: Option<MainnetAutoDesiredSide>,
    #[serde(default)]
    pub current_position_side: Option<MainnetAutoDesiredSide>,
    #[serde(default)]
    pub policy_action: Option<MainnetAutoPolicyAction>,
    #[serde(default)]
    pub policy_reason: Option<String>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MainnetAutoWatchdogEvent {
    pub id: String,
    pub session_id: String,
    pub reason: MainnetAutoStopReason,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MainnetAutoLessonReport {
    pub id: String,
    pub session_id: String,
    pub mode: MainnetAutoRunMode,
    pub live_order_submitted: bool,
    #[serde(default)]
    pub position_policy: AsoPositionPolicy,
    pub signals_observed: u64,
    #[serde(default)]
    pub desired_side_evaluations: u64,
    #[serde(default)]
    pub enter_decisions: u64,
    #[serde(default)]
    pub hold_decisions: u64,
    #[serde(default)]
    pub reverse_decisions: u64,
    #[serde(default)]
    pub no_trade_decisions: u64,
    #[serde(default)]
    pub margin_type_block_count: u64,
    pub decisions_blocked: u64,
    pub would_submit_decisions: u64,
    pub duplicate_suppression_count: u64,
    pub top_blockers: Vec<String>,
    pub watchdog_stop_reason: Option<MainnetAutoStopReason>,
    pub risk_budget_utilization: BTreeMap<String, String>,
    pub reference_price_freshness_summary: String,
    pub aso_signal_summary: String,
    pub stale_or_degraded_state: Vec<String>,
    pub next_checks: Vec<String>,
    pub recommendation: String,
    pub explanation: String,
    pub lessons_path: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MainnetAutoEvidenceExportResult {
    pub path: String,
    pub files: Vec<String>,
    pub final_verdict: String,
    pub live_order_submitted: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveOrderPreflightResult {
    pub id: String,
    pub credential_id: Option<LiveCredentialId>,
    pub environment: LiveEnvironment,
    pub symbol: Symbol,
    pub side: Option<LiveOrderSide>,
    pub order_type: Option<LiveOrderType>,
    pub payload: BTreeMap<String, String>,
    pub accepted: bool,
    pub exchange_error_code: Option<i64>,
    pub exchange_error_message: Option<String>,
    pub local_blocking_reason: Option<LiveBlockingReason>,
    pub source_signal_id: Option<String>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveReadinessSnapshot {
    pub state: LiveRuntimeState,
    pub environment: LiveEnvironment,
    pub active_credential: Option<LiveCredentialSummary>,
    pub checks: Vec<LiveGateCheck>,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub warnings: Vec<LiveWarning>,
    pub account_snapshot: Option<LiveAccountSnapshot>,
    pub symbol_rules: Option<LiveSymbolRules>,
    pub can_arm: bool,
    pub can_execute_live: bool,
    pub refreshed_at: i64,
}

impl Default for LiveReadinessSnapshot {
    fn default() -> Self {
        Self {
            state: LiveRuntimeState::CredentialsMissing,
            environment: LiveEnvironment::Testnet,
            active_credential: None,
            checks: Vec::new(),
            blocking_reasons: vec![LiveBlockingReason::NoActiveCredential],
            warnings: vec![LiveWarning::TestnetEnvironment],
            account_snapshot: None,
            symbol_rules: None,
            can_arm: false,
            can_execute_live: false,
            refreshed_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveStatusSnapshot {
    pub feature_visible: bool,
    pub mode_preference: LiveModePreference,
    pub environment: LiveEnvironment,
    pub state: LiveRuntimeState,
    pub armed: bool,
    pub active_credential: Option<LiveCredentialSummary>,
    pub readiness: LiveReadinessSnapshot,
    pub reconciliation: LiveReconciliationStatus,
    pub account_snapshot: Option<LiveAccountSnapshot>,
    pub symbol_rules: Option<LiveSymbolRules>,
    pub intent_preview: Option<LiveOrderPreview>,
    pub recent_preflights: Vec<LiveOrderPreflightResult>,
    pub execution: LiveExecutionSnapshot,
    pub execution_availability: LiveExecutionAvailability,
    pub kill_switch: LiveKillSwitchState,
    pub risk_profile: LiveRiskProfile,
    pub auto_executor: LiveAutoExecutorStatus,
    pub mainnet_canary: LiveMainnetCanaryStatus,
    pub mainnet_auto: MainnetAutoStatus,
    pub updated_at: i64,
}

impl Default for LiveStatusSnapshot {
    fn default() -> Self {
        Self {
            feature_visible: true,
            mode_preference: LiveModePreference::Paper,
            environment: LiveEnvironment::Testnet,
            state: LiveRuntimeState::CredentialsMissing,
            armed: false,
            active_credential: None,
            readiness: LiveReadinessSnapshot::default(),
            reconciliation: LiveReconciliationStatus::default(),
            account_snapshot: None,
            symbol_rules: None,
            intent_preview: None,
            recent_preflights: Vec::new(),
            execution: LiveExecutionSnapshot::default(),
            execution_availability: LiveExecutionAvailability {
                can_execute_live: false,
                reason: LiveBlockingReason::NoActiveCredential,
                message: "TESTNET execution requires validated credentials and readiness gates."
                    .to_string(),
            },
            kill_switch: LiveKillSwitchState::default(),
            risk_profile: LiveRiskProfile::default(),
            auto_executor: LiveAutoExecutorStatus::default(),
            mainnet_canary: LiveMainnetCanaryStatus::default(),
            mainnet_auto: MainnetAutoStatus::default(),
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveStateRecord {
    pub mode_preference: LiveModePreference,
    pub environment: LiveEnvironment,
    pub armed: bool,
    pub updated_at: i64,
}

impl Default for LiveStateRecord {
    fn default() -> Self {
        Self {
            mode_preference: LiveModePreference::Paper,
            environment: LiveEnvironment::Testnet,
            armed: false,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveCredentialSecret {
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateLiveCredentialRequest {
    pub alias: String,
    pub environment: LiveEnvironment,
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateLiveCredentialRequest {
    pub alias: Option<String>,
    pub environment: Option<LiveEnvironment>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidateCredentialRequest {
    pub credential_id: LiveCredentialId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshReadinessRequest {
    pub environment: Option<LiveEnvironment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArmLiveModeRequest {
    pub confirm_read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisarmLiveModeRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetLiveModePreferenceRequest {
    pub mode_preference: LiveModePreference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveStartCheck {
    pub allowed: bool,
    pub blocking_reasons: Vec<LiveBlockingReason>,
    pub message: String,
    pub readiness: LiveReadinessSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeActivity {
    HistorySync,
    Rebuilding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Reconnecting,
    Stale,
    Resynced,
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candle {
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub open_time: i64,
    pub close_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsoPoint {
    pub open_time: i64,
    pub bulls: Option<f64>,
    pub bears: Option<f64>,
    pub length: usize,
    pub mode: AsoMode,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalEvent {
    pub id: String,
    pub symbol: Symbol,
    pub timeframe: Timeframe,
    pub open_time: i64,
    pub side: SignalSide,
    pub bulls: f64,
    pub bears: f64,
    pub closed_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trade {
    pub id: String,
    pub symbol: Symbol,
    pub quote_asset: QuoteAsset,
    pub side: PositionSide,
    pub action: TradeAction,
    pub source: TradeSource,
    pub qty: f64,
    pub price: f64,
    pub notional: f64,
    pub entry_price: Option<f64>,
    pub exit_price: Option<f64>,
    pub fee_paid: f64,
    pub realized_pnl: f64,
    pub opened_at: Option<i64>,
    pub closed_at: Option<i64>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub symbol: Symbol,
    pub quote_asset: QuoteAsset,
    pub side: PositionSide,
    pub qty: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub notional: f64,
    pub margin_used: f64,
    pub leverage: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub opened_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Wallet {
    pub quote_asset: QuoteAsset,
    pub initial_balance: f64,
    pub balance: f64,
    pub available_balance: f64,
    pub reserved_margin: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub fees_paid: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceStats {
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub equity: f64,
    pub return_pct: f64,
    pub trades: usize,
    pub closed_trades: usize,
    pub win_rate: f64,
    pub fees_paid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeStatus {
    pub running: bool,
    pub execution_mode: ExecutionMode,
    pub active_symbol: Symbol,
    pub timeframe: Timeframe,
    pub activity: Option<RuntimeActivity>,
    pub last_error: Option<String>,
    pub started_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionState {
    pub status: ConnectionStatus,
    pub status_since: Option<i64>,
    pub last_message_time: Option<i64>,
    pub reconnect_attempts: u64,
    pub resync_required: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub task_count: usize,
    pub collected_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEvent {
    pub id: String,
    pub timestamp: i64,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub active_symbol: Symbol,
    pub available_symbols: Vec<Symbol>,
    pub timeframe: Timeframe,
    pub aso_length: usize,
    pub aso_mode: AsoMode,
    pub leverage: f64,
    pub fee_rate: f64,
    pub sizing_mode: SizingMode,
    pub fixed_notional: f64,
    pub initial_wallet_balance_by_quote: BTreeMap<QuoteAsset, f64>,
    pub paper_enabled: bool,
    pub live_mode_visible: bool,
    pub auto_restart_on_apply: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mut initial_wallet_balance_by_quote = BTreeMap::new();
        initial_wallet_balance_by_quote.insert(QuoteAsset::Usdt, 10_000.0);
        initial_wallet_balance_by_quote.insert(QuoteAsset::Usdc, 10_000.0);

        Self {
            active_symbol: Symbol::BtcUsdt,
            available_symbols: ALLOWED_SYMBOLS.to_vec(),
            timeframe: Timeframe::M1,
            aso_length: 20,
            aso_mode: AsoMode::Both,
            leverage: 5.0,
            fee_rate: 0.0004,
            sizing_mode: SizingMode::FixedNotional,
            fixed_notional: 250.0,
            initial_wallet_balance_by_quote,
            paper_enabled: true,
            live_mode_visible: true,
            auto_restart_on_apply: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mainnet_auto_status_decodes_pre_watchdog_singleton_json() {
        let json = r#"{
            "state": "disabled",
            "mode": "dry_run",
            "config": {
                "enable_live_execution": false,
                "mode": "dry_run",
                "max_runtime_minutes": 15,
                "max_orders": 20,
                "max_fills": 20,
                "max_notional": "80",
                "max_daily_loss": "5",
                "require_flat_start": true,
                "require_flat_stop": true,
                "evidence_required": true,
                "lesson_report_required": true
            },
            "risk_budget": {
                "configured": true,
                "budget_id": "mainnet-auto-legacy",
                "max_notional_per_order": "80",
                "max_total_session_notional": "80",
                "max_open_notional": "80",
                "max_orders_per_session": 20,
                "max_fills_per_session": 20,
                "max_consecutive_losses": 1,
                "max_consecutive_rejections": 1,
                "max_daily_realized_loss": "5",
                "max_position_age_seconds": 900,
                "max_runtime_minutes": 15,
                "max_leverage": "5",
                "require_flat_start": true,
                "require_flat_stop": true,
                "allowed_symbols": ["BTCUSDT"],
                "allowed_order_types": ["MARKET"],
                "require_fresh_reference_price": true,
                "require_fresh_shadow": true,
                "require_fresh_user_data_stream": true,
                "require_evidence_logging": true,
                "require_lessons_report": true,
                "updated_at": 1
            },
            "session_id": null,
            "started_at": null,
            "expires_at": null,
            "stopped_at": null,
            "last_heartbeat_at": null,
            "last_signal_id": null,
            "last_signal_open_time": null,
            "last_order_id": null,
            "last_decision_id": null,
            "last_decision_outcome": null,
            "blocking_reasons": ["mainnet_auto_config_disabled"],
            "current_blockers": ["mainnet_auto_config_disabled"],
            "latest_lessons_recommendation": "live_not_allowed",
            "live_orders_submitted": 0,
            "dry_run_orders_submitted": 0,
            "evidence_path": null,
            "updated_at": 1
        }"#;

        let status: MainnetAutoStatus =
            serde_json::from_str(json).expect("legacy mainnet auto status decodes");

        assert_eq!(status.state, MainnetAutoState::Disabled);
        assert!(status.config.require_manual_canary_evidence);
        assert_eq!(status.watchdog, MainnetAutoWatchdogStatus::default());
        assert_eq!(status.last_watchdog_stop_reason, None);
    }
}
