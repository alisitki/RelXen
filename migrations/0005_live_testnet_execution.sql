CREATE TABLE IF NOT EXISTS live_orders (
  id TEXT PRIMARY KEY NOT NULL,
  client_order_id TEXT NOT NULL UNIQUE,
  exchange_order_id TEXT,
  environment TEXT NOT NULL,
  symbol TEXT NOT NULL,
  side TEXT NOT NULL,
  order_type TEXT NOT NULL,
  status TEXT NOT NULL,
  order_json TEXT NOT NULL,
  submitted_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_orders_updated_at ON live_orders(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_live_orders_symbol_status ON live_orders(symbol, status);

CREATE TABLE IF NOT EXISTS live_fills (
  id TEXT PRIMARY KEY NOT NULL,
  order_id TEXT,
  client_order_id TEXT,
  exchange_order_id TEXT,
  environment TEXT NOT NULL DEFAULT 'testnet',
  symbol TEXT NOT NULL,
  side TEXT NOT NULL,
  fill_json TEXT NOT NULL,
  event_time INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_fills_created_at ON live_fills(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_live_fills_symbol ON live_fills(symbol, event_time DESC);

CREATE TABLE IF NOT EXISTS live_execution_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  execution_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
