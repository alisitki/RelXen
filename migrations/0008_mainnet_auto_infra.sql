CREATE TABLE IF NOT EXISTS mainnet_auto_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  status_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mainnet_auto_risk_budget (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  budget_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mainnet_auto_decisions (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  mode TEXT NOT NULL,
  outcome TEXT NOT NULL,
  symbol TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  signal_open_time INTEGER,
  would_submit INTEGER NOT NULL,
  decision_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mainnet_auto_decisions_created_at
  ON mainnet_auto_decisions(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_mainnet_auto_decisions_signal
  ON mainnet_auto_decisions(symbol, timeframe, signal_open_time, outcome);

CREATE TABLE IF NOT EXISTS mainnet_auto_watchdog_events (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  event_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mainnet_auto_watchdog_events_created_at
  ON mainnet_auto_watchdog_events(created_at DESC);

CREATE TABLE IF NOT EXISTS mainnet_auto_lesson_reports (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  recommendation TEXT NOT NULL,
  report_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mainnet_auto_lesson_reports_created_at
  ON mainnet_auto_lesson_reports(created_at DESC);
