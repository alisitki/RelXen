#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"
STAMP="${RELXEN_SOAK_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
OUT_DIR="${RELXEN_SOAK_ARTIFACT_DIR:-artifacts/testnet-soak/$STAMP}"

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

checkpoint() {
  local name="$1"
  local safe_name
  safe_name="$(printf '%s' "$name" | tr '[:upper:] ' '[:lower:]-' | tr -cd '[:alnum:]-_')"
  mkdir -p "$OUT_DIR/checkpoints"
  fetch_json "/api/live/status" | jq . >"$OUT_DIR/checkpoints/${safe_name}_live_status.json"
  fetch_json "/api/live/orders?limit=100" | jq . >"$OUT_DIR/checkpoints/${safe_name}_orders.json"
  fetch_json "/api/live/fills?limit=100" | jq . >"$OUT_DIR/checkpoints/${safe_name}_fills.json"
  printf '{"time":"%s","checkpoint":"%s"}\n' "$(date -u +%FT%TZ)" "$name" >>"$OUT_DIR/checkpoints.ndjson"
}

pause_for_operator() {
  local message="$1"
  if [[ "${RELXEN_SOAK_NON_INTERACTIVE:-false}" == "true" ]]; then
    echo "non-interactive: $message"
    return
  fi
  read -r -p "$message Press Enter when complete, or Ctrl-C to stop. " _
}

require_tool curl
require_tool jq

mkdir -p "$OUT_DIR"

echo "Checking RelXen server at $BASE_URL"
fetch_json "/api/health" | jq . >/dev/null
if [[ "$(fetch_json "/api/live/status" | jq -r '.mainnet_canary.enabled_by_server')" == "true" ]]; then
  echo "Refusing TESTNET soak: mainnet canary server gate is enabled." >&2
  echo "Restart with RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false before running this drill." >&2
  exit 3
fi
RELXEN_SOAK_ARTIFACT_DIR="$OUT_DIR" RELXEN_BASE_URL="$BASE_URL" RELXEN_SOAK_LABEL="soak-drill-start" \
  "$(dirname "$0")/export_live_evidence.sh" >/dev/null
checkpoint "start"

cat >"$OUT_DIR/operator_drill_checklist.md" <<'CHECKLIST'
# Operator Drill Checklist

Record each scenario as pass, fail, or not exercised:

- Credential/readiness/shadow bootstrap.
- Manual preview and preflight.
- Real TESTNET manual execution.
- Cancel flow.
- Flatten flow.
- Kill switch engage/release.
- Restart and recent-window repair.
- Shadow reconnect and repair/degraded behavior.
- TESTNET auto-executor closed-candle signal path.
- Recent-window repair limitation acknowledged.

Do not enable `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION` during this testnet soak.
Do not paste API secrets into artifacts, logs, or this file.
CHECKLIST

pause_for_operator "Complete credential validation, readiness refresh, and shadow bootstrap."
checkpoint "readiness-shadow"

pause_for_operator "Build a preview and run testnet preflight. Confirm the UI does not treat preflight as execution."
checkpoint "preview-preflight"

pause_for_operator "Submit one bounded real TESTNET order from the displayed preview, if credentials and gates allow."
checkpoint "manual-execution"

pause_for_operator "Exercise cancel if an order is working, or document immediate fill and skip reason."
checkpoint "cancel"

pause_for_operator "Exercise flatten only if a deterministic active-symbol TESTNET position exists."
checkpoint "flatten"

pause_for_operator "Engage kill switch, verify new submissions block, then release it."
checkpoint "kill-switch"

pause_for_operator "Restart the backend against the same DB, then verify recent orders/fills repair coherently."
checkpoint "restart-repair"

pause_for_operator "Force or observe shadow reconnect/repair, then verify recovered or degraded state is truthful."
checkpoint "reconnect-repair"

pause_for_operator "Start TESTNET auto mode and capture either one natural closed-candle auto order or a documented no-signal timeout."
checkpoint "auto-executor"

RELXEN_SOAK_ARTIFACT_DIR="$OUT_DIR" RELXEN_BASE_URL="$BASE_URL" RELXEN_SOAK_LABEL="soak-drill-final" \
  "$(dirname "$0")/export_live_evidence.sh" >/dev/null

echo "$OUT_DIR"
