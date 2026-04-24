#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${RELXEN_BASE_URL:-http://localhost:3000}"

status="$(curl -fsS "$BASE_URL/api/live/mainnet-auto/status")"
printf '%s\n' "$status" | jq '{
  state,
  mode,
  live_config_enabled: .config.enable_live_execution,
  blockers: .current_blockers,
  last_decision_outcome,
  latest_lessons_recommendation,
  evidence_path
}'
