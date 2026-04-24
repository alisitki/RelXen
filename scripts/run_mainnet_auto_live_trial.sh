#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"
SYMBOL="BTCUSDT"
DURATION_MINUTES="15"
ORDER_TYPE="MARKET"
REQUIRED_CONFIRMATION="START MAINNET AUTO LIVE BTCUSDT 15M"
CONFIRMATION="${RELXEN_MAINNET_AUTO_START_CONFIRMATION:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --symbol)
      SYMBOL="${2:-}"
      shift 2
      ;;
    --duration-minutes)
      DURATION_MINUTES="${2:-}"
      shift 2
      ;;
    --confirmation)
      CONFIRMATION="${2:-}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
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

if [[ "$SYMBOL" != "BTCUSDT" || "$DURATION_MINUTES" != "15" || "$ORDER_TYPE" != "MARKET" ]]; then
  echo "Refusing to start live trial: v1 supports BTCUSDT MARKET for exactly 15 minutes." >&2
  exit 2
fi

if [[ "$CONFIRMATION" != "$REQUIRED_CONFIRMATION" ]]; then
  echo "Refusing to start live trial. Required confirmation:" >&2
  echo "$REQUIRED_CONFIRMATION" >&2
  echo "Pass it with --confirmation or RELXEN_MAINNET_AUTO_START_CONFIRMATION." >&2
  exit 2
fi

echo "Configuring bounded MAINNET auto live risk budget. This is not persisted live approval beyond the running server policy."
curl -fsS -X PUT "$BASE_URL/api/live/mainnet-auto/risk-budget" \
  -H 'content-type: application/json' \
  -d '{
    "configured": true,
    "budget_id": "mainnet-auto-live-trial-v1",
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
    "updated_at": 0
  }' \
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
