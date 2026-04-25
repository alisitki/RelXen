# Final RC Snapshot

## Current Status

RelXen is at release-candidate cleanup status as of 2026-04-24.

Paper Mode V1 is complete. Real TESTNET execution has been validated. Two bounded manual MAINNET canaries have been completed and reconciled flat. The cancel endpoint ergonomics issue found during the second canary has been fixed. No order was submitted during this release-candidate cleanup pass.

MAINNET auto infrastructure now exists for default-off dry-run/status/evidence/lesson reporting. A credential-selected operator-DB dry-run was completed on 2026-04-24 with no live order submitted. Mainnet Auto Live Support v1 is implemented for an explicit 15-minute `BTCUSDT` session with session-level confirmation, watchdog, risk gates, and evidence/lesson logging, but no real live-auto session has been run by Codex. The operator terminal helper now matches the requested explicit live-trial flags and checks the running server is in session-scoped live mode before calling the existing start endpoint. Live MAINNET auto-execution remains disabled by default. Broader MAINNET operation is not enabled.

The dashboard has a shareable RC UI cleanup: the top of the app now shows mode, live scope, MAINNET auto block, MAINNET canary state, kill switch, active symbol, current state, and position state in plain text. The LIVE ACCESS panel is grouped into credential, readiness/shadow/account, preview/preflight, safety/canary controls, orders/fills, and advanced details.

## Completed Capabilities

- Local Paper Mode V1 dashboard with ASO signals, paper engine, SQLite persistence, restart/rebuild recovery, and backend-served frontend.
- Env-backed local credential source for operator convenience, with `.env` ignored and raw env secrets kept process-only.
- OS secure storage remains the preferred production-grade secret path.
- Signed Binance USD-M credential validation, read-only account snapshots, symbol rules, and live shadow reconciliation.
- Explicit TESTNET `MARKET` / `LIMIT` execution, cancel, cancel-all-active-symbol, flatten, kill switch, and closed-candle TESTNET auto execution.
- Default-off manual MAINNET canary path for non-marketable `LIMIT` orders only, behind server canary flag, exact confirmation text, fresh account/rules/shadow/reference-price gates, and conservative risk controls.
- Reference-price freshness hardening for MAINNET final preview/submit gates.
- Cancel route fix: `POST /api/live/orders/:order_ref/cancel` uses the path `order_ref` as authoritative, accepts omitted or matching optional body `order_ref`, and rejects mismatches.
- Operator UI cleanup that keeps safety-critical state visible by default without changing backend trading behavior.
- MAINNET auto dry-run infrastructure with fail-closed live-start blocking, persisted risk budget/state/decision/watchdog/lesson metadata, headless helper scripts, evidence export, and lesson reports.
- Gated MAINNET auto live-session support for an explicit operator-started 15-minute `BTCUSDT` `MARKET` trial: exact session confirmation, one-position/one-in-flight enforcement, watchdog runtime stop, closed-candle ASO signal path, mocked-adapter test coverage, and headless live trial/status scripts.
- Operator-DB MAINNET auto dry-run evidence under `artifacts/mainnet-auto/20260424T142250Z-operator-db-dry-run/`.

## Validated Evidence Summary

- TESTNET soak: `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`
  - Real TESTNET credential validation, readiness/shadow sync, preflight, manual execution, cancel, flatten, kill switch, restart repair, reconnect repair, and TESTNET auto proof.
  - This is a valid historical export with the older evidence shape: it uses masked `credentials.json` and does not include the newer canary-specific before/after snapshot filenames or `final_verdict.json`.
- First MAINNET canary: `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`
  - One `BTCUSDT SELL LIMIT 0.001 @ 77950` order submitted with ACK, canceled, reconciled with `executed_qty=0.000`, no fill, flat final position, restart repair passed, canary flag disabled afterward.
- Second MAINNET canary: `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`
  - One `BTCUSDT BUY LIMIT 0.001 @ 77800` order submitted with ACK, canceled, reconciled with `executed_qty=0.000`, no fill, flat final position, restart repair passed, canary flag disabled afterward.
  - The first cancel attempt failed because the old payload shape required duplicated body `order_ref`; retry succeeded on the same order. The code was fixed afterward. Evidence remains truthful.

## Safe Startup Instructions

Safe local startup:

```sh
RELXEN_CREDENTIAL_SOURCE=env \
RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false \
RELXEN_AUTO_START=false \
cargo run -p relxen-server
```

Safe status checks:

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/bootstrap
curl http://localhost:3000/api/live/status
curl http://localhost:3000/api/live/credentials
curl -I http://localhost:3000/
```

Expected safe posture:

- `mainnet_canary.enabled_by_server=false`
- `mainnet_canary.manual_execution_enabled=false`
- MAINNET auto state stopped/blocked
- credentials shown as masked metadata only
- no open MAINNET BTCUSDT order
- BTCUSDT position flat

## Current Live Safety Posture

- MAINNET canary is disabled by default.
- MAINNET auto live execution remains blocked by default. The support path exists but requires a separate explicit live-auto execution task before any real run.
- MAINNET manual canary requires a separate explicit operator task, server canary flag, exact confirmation text, fresh gates, and immediate cancel/reconcile.
- Supported live symbols remain `BTCUSDT` and `BTCUSDC`.
- Conditional/algo orders are unsupported.
- ACK is not treated as fill; order/fill/account truth comes from user-data stream and bounded REST repair.
- `.env` is local operator convenience, not production-grade secret storage.

## Explicitly Not Enabled

- Broader MAINNET operation.
- Live MAINNET auto-execution by default or without the separate explicit 15-minute execution batch.
- Conditional/algo orders such as stop, take-profit, or trailing orders.
- Liquidation heatmap/liquidation context as a signal, API, panel, or live decision layer.
- Hedge mode, multi-assets mode, multi-symbol concurrent live runtime, auth, Tauri packaging, strategy marketplace, or optimization tooling.

## Evidence Artifact Policy

`artifacts/testnet-soak/`, `artifacts/mainnet-canary/`, and `artifacts/mainnet-auto/` are ignored local operational artifacts. They are not part of the release-candidate commit set by default.

The committed repository should contain durable summaries and reports in `docs/`, while raw evidence bundles stay local unless a future task explicitly curates and secret-scans a bundle for publication. Historical evidence must not be rewritten to hide real failures; reports should preserve the truth that the second canary's first cancel attempt failed due to the old payload shape and the retry succeeded.

`web/dist/`, `target/`, `var/`, and `web/node_modules/` are generated or local runtime outputs and remain ignored. `.env` remains ignored and untracked.

## Test And Build Gate Status

The release-candidate gate for this cleanup pass was run with:

- `git diff --check`
- `bash -n scripts/export_live_evidence.sh`
- `bash -n scripts/run_testnet_soak.sh`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd web && npm test`
- `cd web && npm run build`
- `cargo build --workspace --release`

Safe smoke was run with MAINNET canary disabled and MAINNET auto stopped. No order was submitted during the smoke.

## Final Known Risks

- Two bounded manual MAINNET canaries prove the guarded canary path, not broad production MAINNET trading.
- TESTNET evidence uses an older export schema; the docs record that limitation rather than rewriting history.
- Execution repair is bounded by recent-window exchange query limits.
- `.env` credential mode is useful for local validation but is not production-grade secret storage.
- Liquidation heatmap/liquidation context remains undesigned and must not influence live execution.
- The operator-DB dry-run produced a would-submit lesson, but that only supports preparing a separate explicit live-auto plan; it is not approval to trade.

## Exact Next Bounded Task

Operator may start the explicit 15-minute MAINNET auto trial from the documented terminal sequence. After the run, export evidence, verify flat stop/no open order, review lessons, and update docs with the actual outcome. Do not enable live MAINNET auto in normal startup.
