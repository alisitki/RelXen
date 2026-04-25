#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"
SYMBOL="BTCUSDT"
DURATION_MINUTES="${RELXEN_MAINNET_AUTO_MAX_RUNTIME_MINUTES:-15}"
ORDER_TYPE="MARKET"
REQUIRED_CONFIRMATION="START MAINNET AUTO LIVE BTCUSDT 15M"
CONFIRMATION="${RELXEN_MAINNET_AUTO_START_CONFIRMATION:-}"
MAX_LEVERAGE="${RELXEN_MAINNET_AUTO_MAX_LEVERAGE:-5}"
MAX_NOTIONAL="${RELXEN_MAINNET_AUTO_MAX_NOTIONAL:-80}"
MAX_SESSION_LOSS_USDT="${RELXEN_MAINNET_AUTO_MAX_DAILY_LOSS:-5}"
MAX_ORDERS="${RELXEN_MAINNET_AUTO_MAX_ORDERS:-20}"
MAX_FILLS="${RELXEN_MAINNET_AUTO_MAX_FILLS:-20}"
ALLOWED_MARGIN_TYPE="${RELXEN_MAINNET_AUTO_ALLOWED_MARGIN_TYPE:-isolated}"
POSITION_POLICY="${RELXEN_MAINNET_AUTO_POSITION_POLICY:-crossover_only}"
ASO_DELTA_THRESHOLD="${RELXEN_MAINNET_AUTO_ASO_DELTA_THRESHOLD:-5}"
ASO_ZONE_THRESHOLD="${RELXEN_MAINNET_AUTO_ASO_ZONE_THRESHOLD:-55}"

usage() {
  cat >&2 <<'USAGE'
usage: run_mainnet_auto_live_trial.sh \
  --symbol BTCUSDT \
  --duration-minutes 15 \
  --max-leverage 100 \
  --max-notional 80 \
  --max-session-loss-usdt 5 \
  --order-type MARKET \
  --allowed-margin-type isolated \
  --position-policy crossover_only \
  --aso-delta-threshold 5 \
  --aso-zone-threshold 55 \
  --confirm "START MAINNET AUTO LIVE BTCUSDT 15M"
USAGE
}

need_value() {
  if [[ $# -lt 2 || -z "${2:-}" || "${2:-}" == --* ]]; then
    echo "missing value for $1" >&2
    usage
    exit 2
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --symbol)
      need_value "$@"
      SYMBOL="${2:-}"
      shift 2
      ;;
    --duration-minutes)
      need_value "$@"
      DURATION_MINUTES="${2:-}"
      shift 2
      ;;
    --order-type)
      need_value "$@"
      ORDER_TYPE="${2:-}"
      shift 2
      ;;
    --max-leverage)
      need_value "$@"
      MAX_LEVERAGE="${2:-}"
      shift 2
      ;;
    --max-notional)
      need_value "$@"
      MAX_NOTIONAL="${2:-}"
      shift 2
      ;;
    --max-session-loss-usdt|--max-daily-loss-usdt|--max-daily-loss)
      need_value "$@"
      MAX_SESSION_LOSS_USDT="${2:-}"
      shift 2
      ;;
    --max-orders)
      need_value "$@"
      MAX_ORDERS="${2:-}"
      shift 2
      ;;
    --max-fills)
      need_value "$@"
      MAX_FILLS="${2:-}"
      shift 2
      ;;
    --allowed-margin-type)
      need_value "$@"
      ALLOWED_MARGIN_TYPE="${2:-}"
      shift 2
      ;;
    --position-policy)
      need_value "$@"
      POSITION_POLICY="${2:-}"
      shift 2
      ;;
    --aso-delta-threshold)
      need_value "$@"
      ASO_DELTA_THRESHOLD="${2:-}"
      shift 2
      ;;
    --aso-zone-threshold)
      need_value "$@"
      ASO_ZONE_THRESHOLD="${2:-}"
      shift 2
      ;;
    --confirm|--confirmation)
      need_value "$@"
      CONFIRMATION="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ "${RELXEN_ENABLE_MAINNET_AUTO_EXECUTION:-false}" != "true" ]]; then
  echo "Refusing to start live trial: RELXEN_ENABLE_MAINNET_AUTO_EXECUTION must be true in this shell and on the running server." >&2
  exit 2
fi

if [[ "${RELXEN_MAINNET_AUTO_MODE:-dry_run}" != "live" ]]; then
  echo "Refusing to start live trial: RELXEN_MAINNET_AUTO_MODE must be live in this shell and on the running server." >&2
  exit 2
fi

if [[ ! "$DURATION_MINUTES" =~ ^[0-9]+$ || ! "$MAX_ORDERS" =~ ^[0-9]+$ || ! "$MAX_FILLS" =~ ^[0-9]+$ ]]; then
  echo "Refusing to start live trial: duration, max-orders, and max-fills must be integer values." >&2
  exit 2
fi

if [[ "$SYMBOL" != "BTCUSDT" || "$DURATION_MINUTES" != "15" || "$ORDER_TYPE" != "MARKET" ]]; then
  echo "Refusing to start live trial: v1 supports BTCUSDT MARKET for exactly 15 minutes." >&2
  exit 2
fi

if [[ ! "$MAX_LEVERAGE" =~ ^[0-9]+([.][0-9]+)?$ ]] || ! awk "BEGIN { exit !($MAX_LEVERAGE > 0 && $MAX_LEVERAGE <= 100) }"; then
  echo "Refusing to start live trial: --max-leverage must be greater than 0 and no more than 100." >&2
  exit 2
fi

if [[ "$MAX_NOTIONAL" != "80" || "$MAX_SESSION_LOSS_USDT" != "5" || "$MAX_ORDERS" != "20" || "$MAX_FILLS" != "20" ]]; then
  echo "Refusing to start live trial: v1 operator batch requires notional=80, loss=5, max-orders=20, max-fills=20." >&2
  exit 2
fi

case "$ALLOWED_MARGIN_TYPE" in
  isolated|cross|any) ;;
  *)
    echo "Refusing to start live trial: --allowed-margin-type must be isolated, cross, or any." >&2
    exit 2
    ;;
esac

case "$POSITION_POLICY" in
  crossover_only|always_in_market|flat_allowed) ;;
  *)
    echo "Refusing to start live trial: --position-policy must be crossover_only, always_in_market, or flat_allowed." >&2
    exit 2
    ;;
esac

if [[ "$CONFIRMATION" != "$REQUIRED_CONFIRMATION" ]]; then
  echo "Refusing to start live trial. Required confirmation:" >&2
  echo "$REQUIRED_CONFIRMATION" >&2
  echo "Pass it with --confirm or RELXEN_MAINNET_AUTO_START_CONFIRMATION." >&2
  exit 2
fi

auto_status="$(curl -fsS "$BASE_URL/api/live/mainnet-auto/status")"
server_live_enabled="$(jq -r '.config.enable_live_execution // false' <<<"$auto_status")"
server_mode="$(jq -r '.mode // "unknown"' <<<"$auto_status")"
server_allowed_margin_type="$(jq -r '.config.allowed_margin_type // "isolated"' <<<"$auto_status")"
server_position_policy="$(jq -r '.config.position_policy // "crossover_only"' <<<"$auto_status")"
server_aso_delta_threshold="$(jq -r '.config.aso_delta_threshold // "5"' <<<"$auto_status")"
server_aso_zone_threshold="$(jq -r '.config.aso_zone_threshold // "55"' <<<"$auto_status")"

if [[ "$server_live_enabled" != "true" || "$server_mode" != "live" ]]; then
  echo "Refusing to start live trial: running server is not in session-scoped live-auto mode." >&2
  echo "Server config enable_live_execution=$server_live_enabled mode=$server_mode" >&2
  exit 2
fi

if [[ "$server_allowed_margin_type" != "$ALLOWED_MARGIN_TYPE" || "$server_position_policy" != "$POSITION_POLICY" || "$server_aso_delta_threshold" != "$ASO_DELTA_THRESHOLD" || "$server_aso_zone_threshold" != "$ASO_ZONE_THRESHOLD" ]]; then
  echo "Refusing to start live trial: script policy flags must match the running server config." >&2
  echo "Server allowed_margin_type=$server_allowed_margin_type position_policy=$server_position_policy aso_delta_threshold=$server_aso_delta_threshold aso_zone_threshold=$server_aso_zone_threshold" >&2
  echo "Script allowed_margin_type=$ALLOWED_MARGIN_TYPE position_policy=$POSITION_POLICY aso_delta_threshold=$ASO_DELTA_THRESHOLD aso_zone_threshold=$ASO_ZONE_THRESHOLD" >&2
  exit 2
fi

echo "Configuring bounded MAINNET auto live risk budget. This is not persisted live approval beyond the running server policy."
curl -fsS -X PUT "$BASE_URL/api/live/mainnet-auto/risk-budget" \
  -H 'content-type: application/json' \
  -d "$(jq -n \
    --arg budget_id "mainnet-auto-live-trial-v1" \
    --arg max_notional "$MAX_NOTIONAL" \
    --arg max_session_loss_usdt "$MAX_SESSION_LOSS_USDT" \
    --arg max_leverage "$MAX_LEVERAGE" \
    --argjson max_orders "$MAX_ORDERS" \
    --argjson max_fills "$MAX_FILLS" \
    --argjson max_runtime_minutes "$DURATION_MINUTES" \
    '{
      configured: true,
      budget_id: $budget_id,
      max_notional_per_order: $max_notional,
      max_total_session_notional: $max_notional,
      max_open_notional: $max_notional,
      max_orders_per_session: $max_orders,
      max_fills_per_session: $max_fills,
      max_consecutive_losses: 1,
      max_consecutive_rejections: 1,
      max_daily_realized_loss: $max_session_loss_usdt,
      max_position_age_seconds: 900,
      max_runtime_minutes: $max_runtime_minutes,
      max_leverage: $max_leverage,
      require_flat_start: true,
      require_flat_stop: true,
      allowed_symbols: ["BTCUSDT"],
      allowed_order_types: ["MARKET"],
      require_fresh_reference_price: true,
      require_fresh_shadow: true,
      require_fresh_user_data_stream: true,
      require_evidence_logging: true,
      require_lessons_report: true,
      updated_at: 0
    }')" \
  | jq '{budget_id, allowed_symbols, allowed_order_types, max_runtime_minutes, max_orders_per_session, max_fills_per_session, max_notional_per_order, max_daily_realized_loss, max_leverage}'

echo "Starting bounded MAINNET auto live session. No per-order confirmation is used after this session-level confirmation."
curl -fsS -X POST "$BASE_URL/api/live/mainnet-auto/start" \
  -H 'content-type: application/json' \
  -d "$(jq -n \
    --arg symbol "$SYMBOL" \
    --arg order_type "$ORDER_TYPE" \
    --arg confirmation_text "$CONFIRMATION" \
    --arg allowed_margin_type "$ALLOWED_MARGIN_TYPE" \
    --arg position_policy "$POSITION_POLICY" \
    --arg aso_delta_threshold "$ASO_DELTA_THRESHOLD" \
    --arg aso_zone_threshold "$ASO_ZONE_THRESHOLD" \
    --argjson duration_minutes "$DURATION_MINUTES" \
    '{symbol: $symbol, duration_minutes: $duration_minutes, order_type: $order_type, confirmation_text: $confirmation_text, allowed_margin_type: $allowed_margin_type, position_policy: $position_policy, aso_delta_threshold: $aso_delta_threshold, aso_zone_threshold: $aso_zone_threshold}')" \
  | jq '{state, mode, session_id, started_at, expires_at, margin_policy, position_policy, current_blockers, live_orders_submitted, evidence_path}'
