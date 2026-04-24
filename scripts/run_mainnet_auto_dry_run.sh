#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"

if [[ "${RELXEN_ENABLE_MAINNET_AUTO_EXECUTION:-false}" == "true" ]]; then
  echo "Refusing to run dry-run helper with RELXEN_ENABLE_MAINNET_AUTO_EXECUTION=true." >&2
  exit 2
fi

echo "Starting MAINNET auto dry-run. No order endpoint is called by this script."
curl -fsS -X POST "$BASE_URL/api/live/mainnet-auto/dry-run/start" \
  | jq '{state, mode, current_blockers, last_decision_outcome, live_orders_submitted}'

echo "Exporting MAINNET auto dry-run evidence."
curl -fsS -X POST "$BASE_URL/api/live/mainnet-auto/export-evidence" \
  | jq '{path, final_verdict, live_order_submitted}'
