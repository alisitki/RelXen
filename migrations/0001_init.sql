CREATE TABLE IF NOT EXISTS settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  active_symbol TEXT NOT NULL,
  available_symbols_json TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  aso_length INTEGER NOT NULL,
  aso_mode TEXT NOT NULL,
  leverage REAL NOT NULL,
  fee_rate REAL NOT NULL,
  sizing_mode TEXT NOT NULL,
  fixed_notional REAL NOT NULL,
  initial_wallet_balance_json TEXT NOT NULL,
  paper_enabled INTEGER NOT NULL,
  live_mode_visible INTEGER NOT NULL,
  auto_restart_on_apply INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS klines (
  symbol TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  open_time INTEGER NOT NULL,
  close_time INTEGER NOT NULL,
  open REAL NOT NULL,
  high REAL NOT NULL,
  low REAL NOT NULL,
  close REAL NOT NULL,
  volume REAL NOT NULL,
  closed INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (symbol, timeframe, open_time)
);

CREATE TABLE IF NOT EXISTS signals (
  id TEXT PRIMARY KEY,
  symbol TEXT NOT NULL,
  timeframe TEXT NOT NULL,
  open_time INTEGER NOT NULL,
  side TEXT NOT NULL,
  bulls REAL NOT NULL,
  bears REAL NOT NULL,
  closed_only INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_signals_symbol_timeframe_open_time
  ON signals(symbol, timeframe, open_time);

CREATE TABLE IF NOT EXISTS trades (
  id TEXT PRIMARY KEY,
  symbol TEXT NOT NULL,
  quote_asset TEXT NOT NULL,
  side TEXT NOT NULL,
  action TEXT NOT NULL,
  qty REAL NOT NULL,
  price REAL NOT NULL,
  notional REAL NOT NULL,
  fee_paid REAL NOT NULL,
  realized_pnl REAL NOT NULL,
  timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS paper_wallets (
  quote_asset TEXT PRIMARY KEY,
  initial_balance REAL NOT NULL,
  balance REAL NOT NULL,
  available_balance REAL NOT NULL,
  reserved_margin REAL NOT NULL,
  unrealized_pnl REAL NOT NULL,
  realized_pnl REAL NOT NULL,
  fees_paid REAL NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS paper_positions (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  symbol TEXT NOT NULL,
  quote_asset TEXT NOT NULL,
  side TEXT NOT NULL,
  qty REAL NOT NULL,
  entry_price REAL NOT NULL,
  mark_price REAL NOT NULL,
  notional REAL NOT NULL,
  margin_used REAL NOT NULL,
  leverage REAL NOT NULL,
  unrealized_pnl REAL NOT NULL,
  realized_pnl REAL NOT NULL,
  opened_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS logs (
  id TEXT PRIMARY KEY,
  timestamp INTEGER NOT NULL,
  level TEXT NOT NULL,
  target TEXT NOT NULL,
  message TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);
