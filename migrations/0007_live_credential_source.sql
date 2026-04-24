ALTER TABLE live_credentials
  ADD COLUMN source TEXT NOT NULL DEFAULT 'secure_store';
