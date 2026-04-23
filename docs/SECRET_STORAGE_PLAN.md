# Secret Storage Plan

## Principle

Live credentials must be stored outside the RelXen SQLite database and outside frontend storage. The implemented normal-runtime strategy is an OS secure-storage adapter behind the app secret-store port; tests use in-memory stores.

## Allowed Storage

- macOS Keychain.
- Windows Credential Manager.
- Linux Secret Service, libsecret, or a distribution-supported equivalent.
- In-memory access only for the shortest practical execution scope.

The repository persists non-secret metadata in SQLite. Raw API secrets live in secure storage. API keys are accepted only through local backend requests and returned to the UI as masked hints.

## Forbidden Storage

- Plaintext SQLite secrets.
- Plaintext `.env`, TOML, JSON, YAML, or config-file secrets for normal operation.
- Frontend `localStorage` or `sessionStorage` secrets.
- Raw-secret API responses or WebSocket events.
- Logs, traces, panic messages, or test snapshots containing secrets.
- Reversible masking stored alongside the raw secret.

## Credential Metadata Model

SQLite may store metadata such as:

- Credential alias or operator-provided name.
- Exchange and environment marker, for example `binance_futures_testnet` or `binance_futures_mainnet`.
- Masked public key display, for example first four and last four characters only.
- Last validated timestamp.
- Validation status: `unknown`, `valid`, `invalid_api_key`, `invalid_signature`, `permission_denied`, `timestamp_skew`, `environment_mismatch`, `network_error`, `exchange_error`, `response_decode_error`, `secure_store_unavailable`.
- Capability summary, for example market data, account read, trading permission.

SQLite must not store raw API secret material or enough data to reconstruct it.

## Save Flow

1. Operator submits credentials over a local backend API.
2. Backend validates request shape and refuses to echo raw values.
3. Backend stores raw material in OS secure storage.
4. Backend persists only metadata in SQLite.
5. Backend returns masked metadata only.
6. Operator validation is run through the explicit validation command.

## Validate Flow

1. Backend loads raw material from OS secure storage.
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

## Dev-Only Unsafe Overrides

A future implementation may allow explicit local-development overrides such as environment variables only when all of these are true:

- The feature is named as unsafe, for example `RELXEN_UNSAFE_DEV_API_SECRET`.
- It is disabled by default.
- It is rejected for mainnet unless a separate explicit development flag is set.
- Startup logs warn that unsafe credential injection is active without printing values.
- Documentation says it is not supported for normal operation.

Unsafe overrides are for adapter development and CI fakes only. They must not become the default operator path.
