#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"

result="$(curl -fsS -X POST "$BASE_URL/api/live/mainnet-auto/export-evidence")"
printf '%s\n' "$result" | jq '{
  path,
  final_verdict,
  live_order_submitted,
  files
}'
