CREATE TABLE IF NOT EXISTS live_shadow_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  reconciliation_json TEXT NOT NULL,
  shadow_json TEXT,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS live_preflight_results (
  id TEXT PRIMARY KEY,
  environment TEXT NOT NULL,
  symbol TEXT NOT NULL,
  side TEXT,
  order_type TEXT,
  accepted INTEGER NOT NULL,
  result_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_preflight_results_created_at
  ON live_preflight_results(created_at DESC);
