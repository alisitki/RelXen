CREATE TABLE IF NOT EXISTS live_credentials (
  id TEXT PRIMARY KEY,
  alias TEXT NOT NULL,
  environment TEXT NOT NULL,
  api_key_hint TEXT NOT NULL,
  validation_status TEXT NOT NULL,
  last_validated_at INTEGER,
  last_validation_error TEXT,
  is_active INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_credentials_environment_active
  ON live_credentials(environment, is_active);

CREATE TABLE IF NOT EXISTS live_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  mode_preference TEXT NOT NULL,
  environment TEXT NOT NULL,
  armed INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
