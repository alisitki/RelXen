#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"
STAMP="${RELXEN_SOAK_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
EVIDENCE_KIND="${RELXEN_EVIDENCE_KIND:-testnet-soak}"
DEFAULT_OUT_DIR="artifacts/testnet-soak/$STAMP"
if [[ "$EVIDENCE_KIND" == "mainnet-canary" ]]; then
  DEFAULT_OUT_DIR="artifacts/mainnet-canary/$STAMP"
fi
OUT_DIR="${RELXEN_SOAK_ARTIFACT_DIR:-$DEFAULT_OUT_DIR}"
LABEL="${RELXEN_SOAK_LABEL:-manual-export}"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 2
  fi
}

fetch_json() {
  local endpoint="$1"
  curl --globoff --fail --silent --show-error "$BASE_URL$endpoint"
}

write_json() {
  local endpoint="$1"
  local file="$2"
  local tmp
  tmp="$(mktemp)"
  fetch_json "$endpoint" >"$tmp"
  jq . "$tmp" >"$file"
  rm -f "$tmp"
}

append_timeline() {
  local source="$1"
  local file="$2"
  if [[ -s "$file" ]]; then
    jq -c --arg source "$source" '.[]? | {source: $source, item: .}' "$file" >>"$OUT_DIR/timeline.ndjson"
  fi
}

require_tool curl
require_tool jq

mkdir -p "$OUT_DIR"
: >"$OUT_DIR/timeline.ndjson"

generated_at="$(date -u +%FT%TZ)"

write_json "/api/live/status" "$OUT_DIR/live_status_after.json"
if [[ ! -f "$OUT_DIR/live_status_before.json" ]]; then
  cp "$OUT_DIR/live_status_after.json" "$OUT_DIR/live_status_before.json"
fi
write_json "/api/live/credentials" "$OUT_DIR/credentials.json"
cp "$OUT_DIR/credentials.json" "$OUT_DIR/credentials_masked.json"
write_json "/api/bootstrap" "$OUT_DIR/bootstrap_snapshot.json"
if [[ ! -f "$OUT_DIR/bootstrap_snapshot_before.json" ]]; then
  cp "$OUT_DIR/bootstrap_snapshot.json" "$OUT_DIR/bootstrap_snapshot_before.json"
fi
cp "$OUT_DIR/bootstrap_snapshot.json" "$OUT_DIR/bootstrap_snapshot_after.json"
write_json "/api/live/orders?limit=100" "$OUT_DIR/orders.json"
write_json "/api/live/fills?limit=100" "$OUT_DIR/fills.json"
write_json "/api/live/preflights?limit=100" "$OUT_DIR/preflights.json"
write_json "/api/logs?limit=300" "$OUT_DIR/logs.json"

jq '.account_snapshot' "$OUT_DIR/live_status_after.json" >"$OUT_DIR/account_snapshot_after.json"
if [[ ! -f "$OUT_DIR/account_snapshot_before.json" ]]; then
  cp "$OUT_DIR/account_snapshot_after.json" "$OUT_DIR/account_snapshot_before.json"
fi
jq '.reconciliation.shadow.positions // []' "$OUT_DIR/live_status_after.json" >"$OUT_DIR/position_snapshot_after.json"
if [[ ! -f "$OUT_DIR/position_snapshot_before.json" ]]; then
  cp "$OUT_DIR/position_snapshot_after.json" "$OUT_DIR/position_snapshot_before.json"
fi
jq '[.[]? | select(.status == "accepted" or .status == "working" or .status == "partially_filled" or .status == "submit_pending" or .status == "cancel_pending")]' \
  "$OUT_DIR/orders.json" >"$OUT_DIR/open_orders_after.json"
if [[ ! -f "$OUT_DIR/open_orders_before.json" ]]; then
  cp "$OUT_DIR/open_orders_after.json" "$OUT_DIR/open_orders_before.json"
fi
jq '.risk_profile' "$OUT_DIR/live_status_after.json" >"$OUT_DIR/risk_profile.json"
jq '{
  reference_price: .intent_preview.reference_price,
  marketability_check: .intent_preview.marketability_check
}' "$OUT_DIR/live_status_after.json" >"$OUT_DIR/reference_price.json"

jq '{
  readiness: .readiness.blocking_reasons,
  execution: .execution.blocking_reasons,
  execution_warnings: .execution.warnings,
  auto_executor: .auto_executor.blocking_reasons,
  mainnet_canary: .mainnet_canary.blocking_reasons,
  kill_switch_engaged: .kill_switch.engaged
}' "$OUT_DIR/live_status_after.json" >"$OUT_DIR/blocking_reasons.json"

jq '[.[]? | select(
  ((.message // "") | test("repair|reconnect|resync|degraded|kill|auto|execute|cancel|flatten|shadow"; "i")) or
  ((.target // "") | test("live|runtime|relxen"; "i"))
)]' "$OUT_DIR/logs.json" >"$OUT_DIR/repair_events.json"

jq '[.[]? | select(((.message // "") | test("kill switch|kill-switch|kill_switch"; "i")))]' \
  "$OUT_DIR/logs.json" >"$OUT_DIR/kill_switch_events.json"

if [[ ! -f "$OUT_DIR/final_verdict.json" ]]; then
  jq -n --arg generated_at "$generated_at" --arg label "$LABEL" '{
    generated_at: $generated_at,
    label: $label,
    verdict: "pending_operator_review",
    raw_secret_policy: "No raw secrets are exported."
  }' >"$OUT_DIR/final_verdict.json"
fi

append_timeline "orders" "$OUT_DIR/orders.json"
append_timeline "fills" "$OUT_DIR/fills.json"
append_timeline "preflights" "$OUT_DIR/preflights.json"
append_timeline "repair_logs" "$OUT_DIR/repair_events.json"

jq -n \
  --arg generated_at "$generated_at" \
  --arg base_url "$BASE_URL" \
  --arg label "$LABEL" \
  --arg evidence_kind "$EVIDENCE_KIND" \
  '{
    generated_at: $generated_at,
    label: $label,
    base_url: $base_url,
    evidence_kind: $evidence_kind,
    secret_policy: "No raw secrets are exported. RelXen live APIs expose masked credential metadata only.",
    files: [
      "manifest.json",
      "session_summary.md",
      "timeline.ndjson",
      "live_status_before.json",
      "live_status_after.json",
      "credentials.json",
      "credentials_masked.json",
      "bootstrap_snapshot.json",
      "bootstrap_snapshot_before.json",
      "bootstrap_snapshot_after.json",
      "account_snapshot_before.json",
      "account_snapshot_after.json",
      "position_snapshot_before.json",
      "position_snapshot_after.json",
      "open_orders_before.json",
      "open_orders_after.json",
      "orders.json",
      "fills.json",
      "preflights.json",
      "blocking_reasons.json",
      "repair_events.json",
      "kill_switch_events.json",
      "risk_profile.json",
      "reference_price.json",
      "final_verdict.json",
      "logs.json"
    ]
  }' >"$OUT_DIR/manifest.json"

state="$(jq -r '.state' "$OUT_DIR/live_status_after.json")"
environment="$(jq -r '.environment' "$OUT_DIR/live_status_after.json")"
execution_state="$(jq -r '.execution.state' "$OUT_DIR/live_status_after.json")"
auto_state="$(jq -r '.auto_executor.state' "$OUT_DIR/live_status_after.json")"
kill_switch="$(jq -r '.kill_switch.engaged' "$OUT_DIR/live_status_after.json")"
mainnet_canary_enabled="$(jq -r '.mainnet_canary.enabled_by_server' "$OUT_DIR/live_status_after.json")"
credentials_count="$(jq 'length' "$OUT_DIR/credentials.json")"
active_credential="$(jq -r 'if .active_credential == null then "none" else .active_credential.alias end' "$OUT_DIR/live_status_after.json")"
orders_count="$(jq 'length' "$OUT_DIR/orders.json")"
fills_count="$(jq 'length' "$OUT_DIR/fills.json")"
preflights_count="$(jq 'length' "$OUT_DIR/preflights.json")"
repair_events_count="$(jq 'length' "$OUT_DIR/repair_events.json")"

cat >"$OUT_DIR/session_summary.md" <<SUMMARY
# RelXen Testnet Soak Evidence Export

- Generated at: \`$generated_at\`
- Label: \`$LABEL\`
- Evidence kind: \`$EVIDENCE_KIND\`
- Base URL: \`$BASE_URL\`
- Live state: \`$state\`
- Environment: \`$environment\`
- Execution state: \`$execution_state\`
- Auto-executor state: \`$auto_state\`
- Kill switch engaged: \`$kill_switch\`
- Mainnet canary server gate enabled: \`$mainnet_canary_enabled\`
- Active credential: \`$active_credential\`
- Masked credential summaries exported: \`$credentials_count\`
- Recent orders exported: \`$orders_count\`
- Recent fills exported: \`$fills_count\`
- Recent preflights exported: \`$preflights_count\`
- Repair/degradation log entries exported: \`$repair_events_count\`

This bundle is an evidence snapshot, not proof by itself that a real exchange drill occurred.
Use it with \`docs/TESTNET_SOAK_RUNBOOK.md\` and record which drill scenarios were actually exercised.

Raw API secrets are not included.
SUMMARY

echo "$OUT_DIR"
