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

usage() {
  cat >&2 <<'USAGE'
usage: run_mainnet_auto_live_trial.sh \
  --symbol BTCUSDT \
  --duration-minutes 15 \
  --max-leverage 5 \
  --max-notional 80 \
  --max-session-loss-usdt 5 \
  --order-type MARKET \
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

if [[ "$MAX_LEVERAGE" != "5" || "$MAX_NOTIONAL" != "80" || "$MAX_SESSION_LOSS_USDT" != "5" || "$MAX_ORDERS" != "20" || "$MAX_FILLS" != "20" ]]; then
  echo "Refusing to start live trial: v1 operator batch requires leverage=5, notional=80, loss=5, max-orders=20, max-fills=20." >&2
  exit 2
fi

if [[ "$CONFIRMATION" != "$REQUIRED_CONFIRMATION" ]]; then
  echo "Refusing to start live trial. Required confirmation:" >&2
  echo "$REQUIRED_CONFIRMATION" >&2
  echo "Pass it with --confirm or RELXEN_MAINNET_AUTO_START_CONFIRMATION." >&2
  exit 2
fi

auto_status="$(curl -fsS "$BASE_URL/api/live/mainnet-auto/status")"
server_live_enabled="$(jq -r '.config.enable_live_execution // false' <<<"$auto_status")"
server_mode="$(jq -r '.mode // "unknown"' <<<"$auto_status")"

if [[ "$server_live_enabled" != "true" || "$server_mode" != "live" ]]; then
  echo "Refusing to start live trial: running server is not in session-scoped live-auto mode." >&2
  echo "Server config enable_live_execution=$server_live_enabled mode=$server_mode" >&2
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
  | jq '{budget_id, allowed_symbols, allowed_order_types, max_runtime_minutes, max_orders_per_session, max_fills_per_session, max_notional_per_order, max_daily_realized_loss}'

echo "Starting bounded MAINNET auto live session. No per-order confirmation is used after this session-level confirmation."
curl -fsS -X POST "$BASE_URL/api/live/mainnet-auto/start" \
  -H 'content-type: application/json' \
  -d "$(jq -n \
    --arg symbol "$SYMBOL" \
    --arg order_type "$ORDER_TYPE" \
    --arg confirmation_text "$CONFIRMATION" \
    --argjson duration_minutes "$DURATION_MINUTES" \
    '{symbol: $symbol, duration_minutes: $duration_minutes, order_type: $order_type, confirmation_text: $confirmation_text}')" \
  | jq '{state, mode, session_id, started_at, expires_at, current_blockers, live_orders_submitted, evidence_path}'
