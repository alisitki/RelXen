#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"
MODE="${1:---precheck}"

fetch_status() {
  curl -fsS "$BASE_URL/api/live/status"
}

fetch_auto() {
  curl -fsS "$BASE_URL/api/live/mainnet-auto/status"
}

render_precheck() {
  local live_json auto_json
  live_json="$(fetch_status)"
  auto_json="$(fetch_auto)"
  jq -n --argjson live "$live_json" --argjson auto "$auto_json" '{
    timestamp_ms: ((now * 1000) | floor),
    server_url: (env.RELXEN_BASE_URL // "http://localhost:3000"),
    active_symbol: ($live.symbol_rules.symbol // "unknown"),
    active_credential: {
      alias: ($live.active_credential.alias // null),
      source: ($live.active_credential.source // null),
      environment: ($live.active_credential.environment // null),
      api_key_hint: ($live.active_credential.api_key_hint // null)
    },
    mode: ($auto.mode // null),
    state: ($auto.state // null),
    mainnet_auto_config_enabled: ($auto.config.enable_live_execution // false),
    canary_enabled: ($live.mainnet_canary.enabled_by_server // false),
    kill_switch: (if $live.kill_switch.engaged then "engaged" else "released" end),
    account_mode: ($live.account_snapshot.position_mode // "unknown"),
    margin_mode: (if ($live.account_snapshot.multi_assets_margin == false) then "single_asset" else "unsupported_or_unknown" end),
    btcusdt_margin_type: ([$live.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .margin_type][0] // $auto.margin_policy.actual_margin_type // "unknown"),
    allowed_margin_type: ($auto.margin_policy.allowed_margin_type // $auto.config.allowed_margin_type // "isolated"),
    margin_policy_allowed: ($auto.margin_policy.allowed // false),
    margin_policy_blocker: ($auto.margin_policy.blocker // null),
    position_policy: ($auto.position_policy.policy // $auto.config.position_policy // "crossover_only"),
    aso_delta_threshold: ($auto.position_policy.aso_delta_threshold // $auto.config.aso_delta_threshold // "5"),
    aso_zone_threshold: ($auto.position_policy.aso_zone_threshold // $auto.config.aso_zone_threshold // "55"),
    aso_bulls: ($auto.position_policy.last_bulls // null),
    aso_bears: ($auto.position_policy.last_bears // null),
    aso_delta: ($auto.position_policy.last_delta // null),
    desired_side: ($auto.position_policy.desired_side // "none"),
    current_side: ($auto.position_policy.current_side // "none"),
    last_policy_action: ($auto.position_policy.last_action // "no_trade"),
    last_policy_blocker: ($auto.position_policy.last_blocker // null),
    leverage: ([$live.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .leverage][0] // null),
    available_usdt: ([$live.account_snapshot.assets[]? | select(.asset == "USDT") | .available_balance][0] // null),
    margin_balance: ($live.account_snapshot.total_margin_balance // null),
    btcusdt_position_amount: ([$live.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .position_amt][0] // 0),
    btcusdt_entry_price: ([$live.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .entry_price][0] // null),
    unrealized_pnl: ([$live.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .unrealized_pnl][0] // null),
    open_btcusdt_orders: ([$live.execution.recent_orders[]? | select(.environment == "mainnet" and .symbol == "BTCUSDT" and (.status == "working" or .status == "accepted" or .status == "submit_pending" or .status == "partially_filled" or .status == "cancel_pending"))] | length),
    recent_fills_count: ([$live.execution.recent_fills[]? | select(.environment == "mainnet" and .symbol == "BTCUSDT")] | length),
    last_reference_price: ($live.intent_preview.reference_price.price // null),
    last_reference_source: ($live.intent_preview.reference_price.source // null),
    last_reference_age_ms: ($live.intent_preview.reference_price.age_ms // null),
    last_heartbeat_at: ($auto.last_heartbeat_at // null),
    watchdog: ($auto.watchdog // null),
    last_signal_id: ($auto.last_signal_id // null),
    last_signal_open_time: ($auto.last_signal_open_time // null),
    last_decision_outcome: (if (($auto.last_decision_id // null) == null) then null else ($auto.last_decision_outcome // null) end),
    last_order_id: ($auto.last_order_id // null),
    blockers: ($auto.current_blockers // [])
  }'
}

render_summary() {
  local auto_json
  auto_json="$(fetch_auto)"
  jq '{
    session_id,
    state,
    mode,
    live_orders_submitted,
    dry_run_orders_submitted,
    last_decision_outcome: (if ((.last_decision_id // null) == null) then null else (.last_decision_outcome // null) end),
    last_watchdog_stop_reason,
    latest_lessons_recommendation,
    margin_policy,
    position_policy,
    evidence_path,
    blockers: .current_blockers
  }' <<<"$auto_json"
}

render_flat_check() {
  local live_json
  live_json="$(fetch_status)"
  jq '{
    open_btcusdt_orders: ([.execution.recent_orders[]? | select(.environment == "mainnet" and .symbol == "BTCUSDT" and (.status == "working" or .status == "accepted" or .status == "submit_pending" or .status == "partially_filled" or .status == "cancel_pending"))] | length),
    btcusdt_position_amount: ([.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .position_amt][0] // 0),
    btcusdt_margin_type: ([.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .margin_type][0] // "unknown"),
    final_flat: ((([.execution.recent_orders[]? | select(.environment == "mainnet" and .symbol == "BTCUSDT" and (.status == "working" or .status == "accepted" or .status == "submit_pending" or .status == "partially_filled" or .status == "cancel_pending"))] | length) == 0) and (([.account_snapshot.positions[]? | select(.symbol == "BTCUSDT") | .position_amt][0] // 0) == 0)),
    shadow_state: .reconciliation.stream.state,
    shadow_stale: .reconciliation.stream.stale,
    account_snapshot_at: .account_snapshot.fetched_at
  }' <<<"$live_json"
}

case "$MODE" in
  --precheck)
    render_precheck
    ;;
  --summary)
    render_summary
    ;;
  --flat-check)
    render_flat_check
    ;;
  --heartbeat)
    while true; do
      clear
      date
      render_precheck
      sleep "${RELXEN_HEARTBEAT_SECONDS:-5}"
    done
    ;;
  *)
    echo "usage: $0 [--precheck|--heartbeat|--summary|--flat-check]" >&2
    exit 2
    ;;
esac
