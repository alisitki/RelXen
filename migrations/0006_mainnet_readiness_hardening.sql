CREATE TABLE IF NOT EXISTS live_kill_switch (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  state_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS live_risk_profile (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  profile_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS live_auto_executor_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  auto_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS live_intent_locks (
  key TEXT PRIMARY KEY NOT NULL,
  environment TEXT NOT NULL,
  symbol TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  signal_open_time INTEGER NOT NULL,
  status TEXT NOT NULL,
  order_id TEXT,
  lock_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_intent_locks_signal
  ON live_intent_locks(environment, symbol, timeframe, signal_open_time);

CREATE INDEX IF NOT EXISTS idx_live_intent_locks_updated_at
  ON live_intent_locks(updated_at DESC);

CREATE TABLE IF NOT EXISTS live_execution_repair_log (
  id TEXT PRIMARY KEY NOT NULL,
  order_id TEXT,
  action TEXT NOT NULL,
  result TEXT NOT NULL,
  detail TEXT,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_execution_repair_log_created_at
  ON live_execution_repair_log(created_at DESC);
