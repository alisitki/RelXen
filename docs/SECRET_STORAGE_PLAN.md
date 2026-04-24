# Secret Storage Plan

## Principle

Live credentials must be stored outside the RelXen SQLite database and outside frontend storage. The preferred production-grade strategy is an OS secure-storage adapter behind the app secret-store port; tests use in-memory stores. Local operators may explicitly enable env-backed credentials to avoid OS secure-storage prompts during validation, but `.env` is local-only convenience and not production-grade secret storage.

## Allowed Storage

- macOS Keychain.
- Windows Credential Manager.
- Linux Secret Service, libsecret, or a distribution-supported equivalent.
- In-memory access only for the shortest practical execution scope.
- Local `.env` only when `RELXEN_CREDENTIAL_SOURCE=env` is intentionally enabled. Raw env values remain process-only; SQLite may store masked metadata and `source=env`.

The repository persists non-secret metadata in SQLite, including credential source. Raw secure-store API secrets live in secure storage. Raw env API secrets are read from process memory only after `.env` has been loaded and env credentials are explicitly enabled. API keys are returned to the UI as masked hints only.

## Forbidden Storage

- Plaintext SQLite secrets.
- Plaintext `.env`, TOML, JSON, YAML, or config-file secrets for production-grade operation.
- Frontend `localStorage` or `sessionStorage` secrets.
- Raw-secret API responses or WebSocket events.
- Logs, traces, panic messages, or test snapshots containing secrets.
- Reversible masking stored alongside the raw secret.

## Credential Metadata Model

SQLite may store metadata such as:

- Credential alias or operator-provided name.
- Exchange and environment marker, for example `binance_futures_testnet` or `binance_futures_mainnet`.
- Credential source, for example `secure_store` or `env`.
- Masked public key display, for example first four and last four characters only.
- Last validated timestamp.
- Validation status: `unknown`, `valid`, `invalid_api_key`, `invalid_signature`, `permission_denied`, `timestamp_skew`, `environment_mismatch`, `network_error`, `exchange_error`, `response_decode_error`, `secure_store_unavailable`.
- Capability summary, for example market data, account read, trading permission.

SQLite must not store raw API secret material or enough data to reconstruct it.

## Env Credential Source

- `RELXEN_CREDENTIAL_SOURCE=env` is authoritative.
- `RELXEN_ENABLE_ENV_CREDENTIALS=true` is a compatibility alias only when `RELXEN_CREDENTIAL_SOURCE` is unset.
- Supported variables are exactly `BINANCE_TESTNET_API_KEY`, `BINANCE_TESTNET_API_SECRET_KEY`, `BINANCE_MAINNET_API_KEY`, and `BINANCE_MAINNET_API_SECRET_KEY`.
- In compatibility-alias mode, TESTNET env credentials may auto-select only when no valid active TESTNET credential exists.
- In authoritative `RELXEN_CREDENTIAL_SOURCE=env` mode, the TESTNET env credential is selected at startup ahead of any persisted secure-store TESTNET active credential to avoid OS secure-storage prompts during local validation.
- MAINNET env credentials never auto-select and still require explicit selection, `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=true`, exact confirmation text, and all normal canary gates.
- `.env.example` must contain placeholders only, and `.env` must stay gitignored and uncommitted.

## Save Flow

1. Operator submits credentials over a local backend API.
2. Backend validates request shape and refuses to echo raw values.
3. Backend stores raw material in OS secure storage for secure-store credentials; env credentials are read-only runtime summaries.
4. Backend persists only metadata in SQLite.
5. Backend returns masked metadata only.
6. Operator validation is run through the explicit validation command.

## Validate Flow

1. Backend loads raw material from OS secure storage or from process-only env memory.
2. Backend calls exchange account/permission endpoints that do not place orders.
3. Backend verifies environment, permissions, timestamp behavior, and account accessibility.
4. Backend updates metadata with validation status and timestamp.
5. Backend returns masked metadata and operator-meaningful failure reasons.

## Must Never Be Persisted

- API secret.
- Full API key if not strictly required for secure-store lookup.
- Request signatures.
- Raw signed URLs.
- Exchange response bodies that include sensitive account identifiers beyond the chosen metadata.

## Failure Reporting

Credential failures should be specific enough for action but not leak secret material:

- `credentials_missing`
- `invalid_api_key`
- `invalid_signature`
- `permission_denied`
- `timestamp_skew`
- `environment_mismatch`
- `network_error`
- `exchange_error`
- `response_decode_error`
- `secure_store_unavailable`

Raw exchange error bodies should be sanitized before returning to the frontend.

## Rotation And Revocation

- Operators must be able to replace credentials for an alias.
- Revocation should disarm live mode immediately.
- Validation status should become `unknown` after replacement until validation succeeds.
- A failed validation after previously valid credentials should move live state out of `ready` or `armed`.
- Deleting credentials should remove secure-storage material and metadata references.

## Local Env Convenience

Env-backed credentials are supported as a local operator convenience, not a hidden bypass. They are disabled unless explicitly configured, never persist raw values, and do not relax TESTNET or MAINNET execution gates. OS secure storage remains preferred for production-grade secret handling.
