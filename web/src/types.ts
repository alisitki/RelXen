export type SymbolCode = "BTCUSDT" | "BTCUSDC";
export type Timeframe = "1m" | "5m" | "15m" | "1h";
export type AsoMode = "intrabar" | "group" | "both";
export type SignalSide = "buy" | "sell";
export type PositionSide = "long" | "short";
export type TradeSource = "signal" | "manual";
export type RuntimeActivity = "history_sync" | "rebuilding";
export type ConnectionStatus = "reconnecting" | "stale" | "resynced" | "connected" | "disconnected";
export type ToastKind = "info" | "error";
export type LiveEnvironment = "testnet" | "mainnet";
export type LiveModePreference = "paper" | "live_read_only";
export type LiveRuntimeState =
  | "disabled"
  | "credentials_missing"
  | "secure_store_unavailable"
  | "validation_missing"
  | "validation_pending"
  | "validation_failed"
  | "rules_unavailable"
  | "account_snapshot_unavailable"
  | "not_ready"
  | "ready_read_only"
  | "armed_read_only"
  | "shadow_starting"
  | "shadow_syncing"
  | "shadow_running"
  | "shadow_degraded"
  | "preflight_ready"
  | "preflight_blocked"
  | "testnet_execution_ready"
  | "testnet_auto_ready"
  | "testnet_auto_running"
  | "testnet_submit_pending"
  | "testnet_order_open"
  | "testnet_partially_filled"
  | "testnet_filled"
  | "testnet_cancel_pending"
  | "testnet_flatten_pending"
  | "execution_degraded"
  | "execution_blocked"
  | "mainnet_execution_blocked"
  | "mainnet_canary_ready"
  | "mainnet_manual_execution_enabled"
  | "kill_switch_engaged"
  | "start_blocked"
  | "execution_not_implemented"
  | "error";
export type LiveCredentialValidationStatus =
  | "unknown"
  | "valid"
  | "invalid_api_key"
  | "invalid_signature"
  | "permission_denied"
  | "timestamp_skew"
  | "environment_mismatch"
  | "network_error"
  | "exchange_error"
  | "response_decode_error"
  | "secure_store_unavailable";
export type LiveBlockingReason =
  | "no_active_credential"
  | "env_credentials_missing"
  | "env_credential_partial"
  | "secure_store_unavailable"
  | "validation_failed"
  | "validation_missing"
  | "validation_stale"
  | "account_snapshot_missing"
  | "symbol_rules_missing"
  | "rules_missing"
  | "unsupported_symbol"
  | "unsupported_timeframe"
  | "unsupported_account_mode"
  | "paper_position_open"
  | "runtime_busy"
  | "shadow_stream_down"
  | "shadow_state_ambiguous"
  | "preflight_not_supported_on_mainnet"
  | "intent_unavailable"
  | "min_notional"
  | "precision_invalid"
  | "mainnet_execution_blocked"
  | "mainnet_canary_disabled"
  | "mainnet_canary_risk_profile_missing"
  | "mainnet_canary_limit_required"
  | "mainnet_canary_limit_marketable"
  | "mainnet_confirmation_missing"
  | "mainnet_auto_blocked"
  | "reference_price_unavailable"
  | "reference_price_stale"
  | "reference_price_source_failed"
  | "stale_shadow_state"
  | "preview_mismatch"
  | "execution_status_unknown"
  | "duplicate_client_order_id"
  | "order_rejected"
  | "order_not_found"
  | "cancel_failed"
  | "flatten_failed"
  | "kill_switch_engaged"
  | "risk_limit_exceeded"
  | "auto_executor_stopped"
  | "duplicate_signal_suppressed"
  | "recent_window_repair_only"
  | "execution_not_implemented";
export type LiveWarning =
  | "validation_stale"
  | "rules_snapshot_stale"
  | "account_snapshot_stale"
  | "shadow_snapshot_stale"
  | "shadow_stream_stale"
  | "testnet_environment"
  | "open_exchange_position_detected"
  | "unsupported_exchange_mode";
export type LiveShadowStreamState =
  | "stopped"
  | "starting"
  | "connecting"
  | "syncing"
  | "running"
  | "reconnecting"
  | "degraded"
  | "expired";
export type LiveOrderSide = "BUY" | "SELL";
export type LiveOrderType = "MARKET" | "LIMIT";
export type LiveExecutionState =
  | "disabled"
  | "credentials_missing"
  | "validation_missing"
  | "validation_failed"
  | "shadow_only"
  | "preflight_ready"
  | "testnet_execution_ready"
  | "testnet_auto_ready"
  | "testnet_auto_running"
  | "testnet_submit_pending"
  | "testnet_order_open"
  | "testnet_partially_filled"
  | "testnet_filled"
  | "testnet_cancel_pending"
  | "testnet_flatten_pending"
  | "execution_degraded"
  | "execution_blocked"
  | "mainnet_execution_blocked"
  | "mainnet_canary_ready"
  | "mainnet_manual_execution_enabled"
  | "kill_switch_engaged"
  | "error";
export type LiveOrderStatus =
  | "local_created"
  | "submit_pending"
  | "accepted"
  | "working"
  | "partially_filled"
  | "filled"
  | "cancel_pending"
  | "canceled"
  | "rejected"
  | "expired"
  | "expired_in_match"
  | "unknown_needs_repair";
export type LiveAutoExecutorStateKind = "stopped" | "ready" | "running" | "blocked" | "degraded";
export type LiveIntentLockStatus = "created" | "submitted" | "blocked" | "repaired";

export interface AppMetadata {
  app_name: string;
  version: string;
  started_at: number;
}

export interface RuntimeStatus {
  running: boolean;
  execution_mode: "paper" | "live_locked";
  active_symbol: SymbolCode;
  timeframe: Timeframe;
  activity: RuntimeActivity | null;
  last_error: string | null;
  started_at: number | null;
}

export interface Settings {
  active_symbol: SymbolCode;
  available_symbols: SymbolCode[];
  timeframe: Timeframe;
  aso_length: number;
  aso_mode: AsoMode;
  leverage: number;
  fee_rate: number;
  sizing_mode: "fixed_notional";
  fixed_notional: number;
  initial_wallet_balance_by_quote: Record<"USDT" | "USDC", number>;
  paper_enabled: boolean;
  live_mode_visible: boolean;
  auto_restart_on_apply: boolean;
}

export interface Candle {
  symbol: SymbolCode;
  timeframe: Timeframe;
  open_time: number;
  close_time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  closed: boolean;
}

export interface AsoPoint {
  open_time: number;
  bulls: number | null;
  bears: number | null;
  length: number;
  mode: AsoMode;
  ready: boolean;
}

export interface SignalEvent {
  id: string;
  symbol: SymbolCode;
  timeframe: Timeframe;
  open_time: number;
  side: SignalSide;
  bulls: number;
  bears: number;
  closed_only: boolean;
}

export interface Trade {
  id: string;
  symbol: SymbolCode;
  quote_asset: "USDT" | "USDC";
  side: PositionSide;
  action: "open" | "close" | "reverse";
  source: TradeSource;
  qty: number;
  price: number;
  notional: number;
  entry_price: number | null;
  exit_price: number | null;
  fee_paid: number;
  realized_pnl: number;
  opened_at: number | null;
  closed_at: number | null;
  timestamp: number;
}

export interface Position {
  symbol: SymbolCode;
  quote_asset: "USDT" | "USDC";
  side: PositionSide;
  qty: number;
  entry_price: number;
  mark_price: number;
  notional: number;
  margin_used: number;
  leverage: number;
  unrealized_pnl: number;
  realized_pnl: number;
  opened_at: number;
  updated_at: number;
}

export interface Wallet {
  quote_asset: "USDT" | "USDC";
  initial_balance: number;
  balance: number;
  available_balance: number;
  reserved_margin: number;
  unrealized_pnl: number;
  realized_pnl: number;
  fees_paid: number;
  updated_at: number;
}

export interface PerformanceStats {
  realized_pnl: number;
  unrealized_pnl: number;
  equity: number;
  return_pct: number;
  trades: number;
  closed_trades: number;
  win_rate: number;
  fees_paid: number;
}

export interface ConnectionState {
  status: ConnectionStatus;
  status_since: number | null;
  last_message_time: number | null;
  reconnect_attempts: number;
  resync_required: boolean;
  detail: string | null;
}

export interface SystemMetrics {
  cpu_usage_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  task_count: number;
  collected_at: number;
}

export interface LogEvent {
  id: string;
  timestamp: number;
  level: string;
  target: string;
  message: string;
}

export interface LiveCredentialSummary {
  id: string;
  alias: string;
  environment: LiveEnvironment;
  source: "secure_store" | "env";
  api_key_hint: string;
  validation_status: LiveCredentialValidationStatus;
  last_validated_at: number | null;
  last_validation_error: string | null;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface LiveCredentialValidationResult {
  credential_id: string;
  environment: LiveEnvironment;
  status: LiveCredentialValidationStatus;
  validated_at: number;
  message: string | null;
}

export interface LiveGateCheck {
  code: string;
  passed: boolean;
  message: string;
}

export interface LiveAssetBalance {
  asset: string;
  wallet_balance: number;
  available_balance: number;
  unrealized_pnl: number;
}

export interface LivePositionSnapshot {
  symbol: SymbolCode;
  position_side: string;
  position_amt: number;
  entry_price: number;
  mark_price: number | null;
  unrealized_pnl: number;
  leverage: number | null;
}

export interface LiveAccountSnapshot {
  environment: LiveEnvironment;
  can_trade: boolean;
  multi_assets_margin: boolean | null;
  position_mode: string | null;
  account_mode_checked_at: number | null;
  total_wallet_balance: number;
  total_margin_balance: number;
  available_balance: number;
  assets: LiveAssetBalance[];
  positions: LivePositionSnapshot[];
  fetched_at: number;
}

export interface LiveSymbolFilterSummary {
  tick_size: number | null;
  step_size: number | null;
  min_qty: number | null;
  min_notional: number | null;
}

export interface LiveSymbolRules {
  environment: LiveEnvironment;
  symbol: SymbolCode;
  status: string;
  base_asset: string;
  quote_asset: "USDT" | "USDC";
  price_precision: number;
  quantity_precision: number;
  filters: LiveSymbolFilterSummary;
  fetched_at: number;
}

export interface LiveShadowStreamStatus {
  state: LiveShadowStreamState;
  environment: LiveEnvironment;
  listen_key_hint: string | null;
  status_since: number;
  started_at: number | null;
  last_event_time: number | null;
  last_rest_sync_at: number | null;
  reconnect_attempts: number;
  stale: boolean;
  detail: string | null;
}

export interface LiveShadowBalance {
  asset: string;
  wallet_balance: string;
  cross_wallet_balance: string | null;
  balance_change: string | null;
  updated_at: number;
}

export interface LiveShadowPosition {
  symbol: SymbolCode;
  position_side: string;
  position_amt: string;
  entry_price: string;
  unrealized_pnl: string;
  margin_type: string | null;
  isolated_wallet: string | null;
  updated_at: number;
}

export interface LiveShadowOrder {
  order_id: string;
  client_order_id: string | null;
  symbol: SymbolCode;
  side: LiveOrderSide;
  order_type: LiveOrderType;
  time_in_force: string | null;
  original_qty: string;
  executed_qty: string;
  price: string | null;
  avg_price: string | null;
  status: string;
  execution_type: string | null;
  reduce_only: boolean;
  position_side: string | null;
  last_filled_qty?: string | null;
  last_filled_price?: string | null;
  commission?: string | null;
  commission_asset?: string | null;
  trade_id?: string | null;
  self_trade_prevention_mode?: string | null;
  price_match?: string | null;
  expire_reason?: string | null;
  last_update_time: number;
}

export interface LiveAccountShadow {
  environment: LiveEnvironment;
  balances: LiveShadowBalance[];
  positions: LiveShadowPosition[];
  open_orders: LiveShadowOrder[];
  can_trade: boolean;
  multi_assets_margin: boolean | null;
  position_mode: string | null;
  last_event_time: number | null;
  last_rest_sync_at: number | null;
  updated_at: number;
  ambiguous: boolean;
  divergence_reasons: LiveBlockingReason[];
}

export interface LiveReconciliationStatus {
  state: LiveRuntimeState;
  stream: LiveShadowStreamStatus;
  shadow: LiveAccountShadow | null;
  blocking_reasons: LiveBlockingReason[];
  warnings: LiveWarning[];
  updated_at: number;
}

export interface LiveOrderSizingBreakdown {
  requested_notional: string;
  available_balance: string;
  leverage: string;
  required_margin: string;
  raw_quantity: string;
  rounded_quantity: string;
  estimated_notional: string;
}

export interface LiveOrderIntent {
  id: string;
  intent_hash: string;
  environment: LiveEnvironment;
  symbol: SymbolCode;
  side: LiveOrderSide;
  order_type: LiveOrderType;
  quantity: string;
  price: string | null;
  reduce_only: boolean;
  time_in_force: string | null;
  source_signal_id: string | null;
  source_open_time: number | null;
  reason: string;
  exchange_payload: Record<string, string>;
  sizing: LiveOrderSizingBreakdown;
  validation_notes: string[];
  blocking_reasons: LiveBlockingReason[];
  can_preflight: boolean;
  can_execute_now: boolean;
  built_at: number;
}

export interface LiveReferencePriceSnapshot {
  environment: LiveEnvironment;
  symbol: SymbolCode;
  price: string | null;
  source: string | null;
  observed_at: number | null;
  fetched_at: number | null;
  age_ms: number | null;
  stale: boolean;
  failure_reason: string | null;
  blocking_reason: LiveBlockingReason | null;
}

export interface LiveMarketabilityCheck {
  reference_price: string | null;
  reference_price_source: string | null;
  reference_price_age_ms: number | null;
  rounded_order_price: string | null;
  marketable_after_rounding: boolean | null;
  checked_at: number;
}

export interface LiveOrderPreview {
  built_at: number;
  intent: LiveOrderIntent | null;
  blocking_reasons: LiveBlockingReason[];
  validation_errors: string[];
  reference_price?: LiveReferencePriceSnapshot | null;
  marketability_check?: LiveMarketabilityCheck | null;
  message: string;
}

export interface LiveOrderPreflightResult {
  id: string;
  credential_id: string | null;
  environment: LiveEnvironment;
  symbol: SymbolCode;
  side: LiveOrderSide | null;
  order_type: LiveOrderType | null;
  payload: Record<string, string>;
  accepted: boolean;
  exchange_error_code: number | null;
  exchange_error_message: string | null;
  local_blocking_reason: LiveBlockingReason | null;
  source_signal_id: string | null;
  message: string;
  created_at: number;
}

export interface LiveOrderRecord {
  id: string;
  credential_id: string | null;
  environment: LiveEnvironment;
  symbol: SymbolCode;
  side: LiveOrderSide;
  order_type: LiveOrderType;
  status: LiveOrderStatus;
  client_order_id: string;
  exchange_order_id: string | null;
  quantity: string;
  price: string | null;
  executed_qty: string;
  avg_price: string | null;
  reduce_only: boolean;
  time_in_force: string | null;
  intent_id: string | null;
  intent_hash: string | null;
  source_signal_id: string | null;
  source_open_time: number | null;
  reason: string;
  payload: Record<string, string>;
  response_type: string | null;
  self_trade_prevention_mode: string | null;
  price_match: string | null;
  expire_reason: string | null;
  last_error: string | null;
  submitted_at: number;
  updated_at: number;
}

export interface LiveFillRecord {
  id: string;
  order_id: string | null;
  client_order_id: string | null;
  exchange_order_id: string | null;
  symbol: SymbolCode;
  side: LiveOrderSide;
  quantity: string;
  price: string;
  commission: string | null;
  commission_asset: string | null;
  realized_pnl: string | null;
  trade_id: string | null;
  event_time: number;
  created_at: number;
}

export interface LiveExecutionSnapshot {
  state: LiveExecutionState;
  environment: LiveEnvironment;
  can_submit: boolean;
  blocking_reasons: LiveBlockingReason[];
  warnings: LiveWarning[];
  active_order: LiveOrderRecord | null;
  recent_orders: LiveOrderRecord[];
  recent_fills: LiveFillRecord[];
  kill_switch_engaged: boolean;
  repair_recent_window_only: boolean;
  mainnet_canary_enabled: boolean;
  updated_at: number;
}

export interface LiveExecutionRequest {
  intent_id?: string | null;
  confirm_testnet: boolean;
  confirm_mainnet_canary?: boolean;
  confirmation_text?: string | null;
}

export interface LiveExecutionResult {
  accepted: boolean;
  order: LiveOrderRecord | null;
  blocking_reason: LiveBlockingReason | null;
  message: string;
  created_at: number;
}

export interface LiveCancelResult {
  accepted: boolean;
  order: LiveOrderRecord | null;
  blocking_reason: LiveBlockingReason | null;
  message: string;
  created_at: number;
}

export interface LiveCancelAllRequest {
  confirm_testnet: boolean;
  confirm_mainnet_canary?: boolean;
  confirmation_text?: string | null;
}

export interface LiveFlattenResult {
  accepted: boolean;
  canceled_orders: LiveOrderRecord[];
  flatten_order: LiveOrderRecord | null;
  blocking_reason: LiveBlockingReason | null;
  message: string;
  created_at: number;
}

export interface LiveKillSwitchState {
  engaged: boolean;
  reason: string | null;
  engaged_at: number | null;
  released_at: number | null;
  updated_at: number;
}

export interface LiveRiskLimits {
  max_notional_per_order: string;
  max_open_notional_active_symbol: string;
  max_leverage: string;
  max_orders_per_session: number;
  max_fills_per_session: number;
  max_consecutive_rejections: number;
  max_daily_realized_loss: string;
}

export interface LiveRiskProfile {
  configured: boolean;
  profile_name: string | null;
  limits: LiveRiskLimits;
  updated_at: number;
}

export interface LiveAutoExecutorStatus {
  state: LiveAutoExecutorStateKind;
  environment: LiveEnvironment;
  order_type: LiveOrderType;
  started_at: number | null;
  stopped_at: number | null;
  last_signal_id: string | null;
  last_signal_open_time: number | null;
  last_intent_hash: string | null;
  last_order_id: string | null;
  last_message: string | null;
  blocking_reasons: LiveBlockingReason[];
  updated_at: number;
}

export interface LiveMainnetCanaryStatus {
  enabled_by_server: boolean;
  risk_profile_configured: boolean;
  canary_ready: boolean;
  manual_execution_enabled: boolean;
  required_confirmation: string | null;
  blocking_reasons: LiveBlockingReason[];
  updated_at: number;
}

export interface LiveReadinessSnapshot {
  state: LiveRuntimeState;
  environment: LiveEnvironment;
  active_credential: LiveCredentialSummary | null;
  checks: LiveGateCheck[];
  blocking_reasons: LiveBlockingReason[];
  warnings: LiveWarning[];
  account_snapshot: LiveAccountSnapshot | null;
  symbol_rules: LiveSymbolRules | null;
  can_arm: boolean;
  can_execute_live: boolean;
  refreshed_at: number;
}

export interface LiveExecutionAvailability {
  can_execute_live: boolean;
  reason: LiveBlockingReason;
  message: string;
}

export interface LiveStatusSnapshot {
  feature_visible: boolean;
  mode_preference: LiveModePreference;
  environment: LiveEnvironment;
  state: LiveRuntimeState;
  armed: boolean;
  active_credential: LiveCredentialSummary | null;
  readiness: LiveReadinessSnapshot;
  reconciliation: LiveReconciliationStatus;
  account_snapshot: LiveAccountSnapshot | null;
  symbol_rules: LiveSymbolRules | null;
  intent_preview: LiveOrderPreview | null;
  recent_preflights: LiveOrderPreflightResult[];
  execution: LiveExecutionSnapshot;
  execution_availability: LiveExecutionAvailability;
  kill_switch: LiveKillSwitchState;
  risk_profile: LiveRiskProfile;
  auto_executor: LiveAutoExecutorStatus;
  mainnet_canary: LiveMainnetCanaryStatus;
  updated_at: number;
}

export interface CreateLiveCredentialRequest {
  alias: string;
  environment: LiveEnvironment;
  api_key: string;
  api_secret: string;
}

export interface UpdateLiveCredentialRequest {
  alias?: string;
  environment?: LiveEnvironment;
  api_key?: string;
  api_secret?: string;
}

export interface LiveStartCheck {
  allowed: boolean;
  blocking_reasons: LiveBlockingReason[];
  message: string;
  readiness: LiveReadinessSnapshot;
}

export interface ToastMessage {
  id: number;
  kind: ToastKind;
  message: string;
  created_at: number;
}

export interface BootstrapPayload {
  metadata: AppMetadata;
  runtime_status: RuntimeStatus;
  settings: Settings;
  allowed_symbols: SymbolCode[];
  active_symbol: SymbolCode;
  candles: Candle[];
  aso_points: AsoPoint[];
  recent_signals: SignalEvent[];
  recent_trades: Trade[];
  current_position: Position | null;
  wallets: Wallet[];
  performance: PerformanceStats;
  connection_state: ConnectionState;
  live_status: LiveStatusSnapshot;
  system_metrics: SystemMetrics;
  recent_logs: LogEvent[];
}

export type OutboundEvent =
  | { type: "snapshot"; payload: BootstrapPayload }
  | { type: "candle_partial"; payload: Candle }
  | { type: "candle_closed"; payload: Candle }
  | { type: "aso_updated"; payload: AsoPoint }
  | { type: "signal_emitted"; payload: SignalEvent }
  | { type: "trade_appended"; payload: Trade }
  | { type: "trade_history_reset" }
  | { type: "position_updated"; payload: Position | null }
  | { type: "wallet_updated"; payload: Wallet[] }
  | { type: "performance_updated"; payload: PerformanceStats }
  | { type: "connection_changed"; payload: ConnectionState }
  | { type: "runtime_changed"; payload: RuntimeStatus }
  | { type: "live_status_updated"; payload: LiveStatusSnapshot }
  | { type: "live_shadow_status_updated"; payload: LiveReconciliationStatus }
  | { type: "live_shadow_account_updated"; payload: LiveAccountShadow }
  | { type: "live_intent_preview_updated"; payload: LiveOrderPreview }
  | { type: "live_preflight_result_appended"; payload: LiveOrderPreflightResult }
  | { type: "live_execution_state_updated"; payload: LiveExecutionSnapshot }
  | { type: "live_execution_blocked"; payload: { reason: string } }
  | { type: "live_kill_switch_updated"; payload: LiveKillSwitchState }
  | { type: "live_auto_state_updated"; payload: LiveAutoExecutorStatus }
  | { type: "live_execution_degraded"; payload: { reason: string } }
  | { type: "live_execution_resynced" }
  | { type: "live_mainnet_gate_updated"; payload: { enabled: boolean } }
  | { type: "live_order_submitted"; payload: LiveOrderRecord }
  | { type: "live_order_updated"; payload: LiveOrderRecord }
  | { type: "live_fill_appended"; payload: LiveFillRecord }
  | { type: "live_flatten_started"; payload: { symbol: SymbolCode } }
  | { type: "live_flatten_finished"; payload: { message: string } }
  | { type: "live_shadow_degraded"; payload: { reason: string } }
  | { type: "live_shadow_resynced" }
  | { type: "log_appended"; payload: LogEvent }
  | { type: "system_metrics"; payload: SystemMetrics }
  | { type: "resync_required"; payload: { reason: string } };
