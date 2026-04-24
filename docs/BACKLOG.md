# Backlog

## Completed V1 Items

- Clean-room Rust workspace with domain/app/infra/server layering.
- React + Vite dashboard served statically by the backend.
- Supported symbols limited to `BTCUSDT` and `BTCUSDC`.
- Single active symbol and one open paper position at a time.
- Binance Futures REST/WebSocket market-data ingestion.
- Explicit ranged history loading for bootstrap, rebuild, runtime start, and reconnect recovery.
- ASO indicator, closed-candle signal generation, and paper engine.
- SQLite persistence for settings, klines, signals, trades, wallets, positions, and logs.
- Runtime WebSocket deltas, bootstrap snapshots, and deterministic `resync_required`.
- Realtime paper trade history events.
- Operator status UX for connection age, stale state, rebuild/history sync, and command feedback.
- Fixture-backed Binance adapter tests, real SQLite restart/rebuild tests, and server/frontend failure UX tests.
- Paper Mode V1 release-status and runbook docs.

## Completed Live-Foundation Items

- OS secure-storage abstraction with normal runtime backend and in-memory test backend.
- Env-backed local credential source with masked source metadata, authoritative TESTNET env startup selection, compatibility-alias fallback behavior, and explicit MAINNET selection requirement.
- Masked live credential metadata CRUD with active credential selection.
- SQLite live metadata persistence without raw secret storage.
- Binance USDⓈ-M Futures signed read-only credential validation.
- Read-only account snapshot and active-symbol rules retrieval for `BTCUSDT` / `BTCUSDC`.
- Live readiness, blocking reasons, warnings, arming/disarming, and start-gating.
- Live status bootstrap payload, REST APIs, websocket update events, and frontend LIVE ACCESS panel.

## Completed Live-Shadow/Preflight Items

- Binance USDⓈ-M listenKey create/keepalive/close lifecycle.
- User-data stream parsing for account, order-trade, account-config, expiration, and unknown events.
- Read-only shadow account, position, open-order, stream, stale, degraded, and ambiguity state.
- REST shadow refresh and fail-closed degraded state handling.
- Decimal-based live order-intent preview for `BTCUSDT` / `BTCUSDC` and `MARKET` / `LIMIT`.
- Exchange-rule checks for tick size, step size, min qty, min notional, symbol status, and unsupported account modes.
- Testnet-only `order/test` preflight with persisted results and explicit no-order-placed messaging.

## Completed Constrained Testnet Executor Items

- TESTNET `MARKET` / `LIMIT` order submission through Binance USDⓈ-M new-order endpoint.
- TESTNET order cancel, cancel-all-active-symbol, and manual flatten.
- Local live order/fill persistence and execution state cache.
- User-data `ORDER_TRADE_UPDATE` reconciliation into live orders and fills.
- REST fallback methods for order query, open orders, and recent user trades.
- REST APIs, websocket events, and frontend controls for execute/cancel/cancel-all/flatten/orders/fills.

## Completed Mainnet-Readiness Hardening Items

- Kill switch with API/bootstrap/websocket/frontend visibility.
- TESTNET closed-candle auto-executor with explicit start/stop controls.
- Persisted duplicate signal/intent suppression for auto-execution.
- ACK-only real submission handling with exchange-authoritative reconciliation.
- Dedicated Binance position-mode and multi-assets-mode checks.
- Forced user-data reconnect and REST repair before the 24-hour stream limit.
- Recent-window-only execution repair policy due to Binance query retention limits.
- Operator-configured risk profile required before MAINNET canary readiness.
- Manual MAINNET canary execution path behind `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` by default, exact confirmation text, arming, risk profile, fresh shadow/rules/account state, and all normal gates.
- MAINNET auto-execution remains blocked.

## Completed Soak/Evidence Items

- Real TESTNET credential creation, selection, and validation through the secure-store flow.
- Real TESTNET readiness/shadow bootstrap, preview, preflight, manual execution, cancel, flatten, kill switch, restart repair, reconnect repair, and auto duplicate-suppression evidence.
- Real evidence bundle generated under `artifacts/testnet-soak/20260423T1455Z-real-testnet-soak/`.
- Evidence export now includes masked credential summaries to make credential blockers auditable without exposing secrets.
- Evidence export includes credential source metadata and can write the required masked MAINNET canary before/after snapshots.
- Bounded TESTNET soak runbook covering credential readiness, shadow sync, preview, preflight, real TESTNET execution, cancel, flatten, kill switch, restart repair, reconnect repair, auto-executor proof, and recent-window repair limits.
- Secret-safe evidence export script using existing read-only API endpoints.
- Guided operator soak wrapper with checkpoint capture and no built-in order placement.
- Mainnet canary checklist with explicit hard preconditions and no-go conditions.
- Latest soak report updated with real exchange evidence, targeted fixes, env validation evidence, and the current MAINNET NO-GO recommendation.
- TESTNET-only, default-off drill helper for replaying the latest persisted closed signal through the existing auto executor when no natural signal appears during a bounded soak window.
- Manual shadow refresh now performs bounded recent-window execution repair.
- Recent-window repaired fills now backfill local order references when authoritative exchange trade data can be matched to a repaired order.
- Git ignore policy for generated soak artifacts under `artifacts/testnet-soak/`.
- Env-backed TESTNET and MAINNET credential validation without raw-secret persistence or secure-store prompts.
- MAINNET canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T070419Z-balance-blocked/`.
- MAINNET leverage-gated canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T073409Z-leverage-gated/`.
- MAINNET balance-funded canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T083256Z-balance-funded/`.
- MAINNET leverage-fixed canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T084721Z-leverage-fixed/`.
- MAINNET leverage-fixed canary retry NO-GO evidence bundle under `artifacts/mainnet-canary/20260424T085756Z-leverage-fixed/` after reference price became unavailable post kill-switch drill.
- Reference-price freshness hardening with explicit internal-market/REST mark-price resolver, preview diagnostics, and final submit refresh enforcement.
- Successful guarded MAINNET canary evidence bundle under `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`: one `BTCUSDT` non-marketable `LIMIT` order submitted, canceled, reconciled flat, restart-repair checked, and canary flag disabled afterward.
- Post-canary audit of `artifacts/mainnet-canary/20260424T092625Z-reference-price-fixed/`: no raw secrets found in scanned repository/evidence surfaces, no MAINNET fill, no open BTCUSDT position, no active canary flag after safe-default restart, and canary order/fill evidence scoped to the single MAINNET canary while preserving original all-recent exports separately.
- Second MAINNET canary readiness dry-run under `artifacts/mainnet-canary/20260424T121504Z-second-canary-dry-run/`: no real order submitted, `env-mainnet` validated, mainnet shadow refreshed, previous canary remained canceled/no-fill, kill switch engage/release passed, MAINNET canary flag stayed disabled, MAINNET auto stayed stopped, and a fresh non-marketable `BUY LIMIT BTCUSDT 0.001 @ 77800` preview passed with required margin `15.56` against available `USDT=25.0902305`.
- Second bounded MAINNET canary execution under `artifacts/mainnet-canary/20260424T122751Z-second-canary-execution/`: one real `BUY LIMIT BTCUSDT 0.001 @ 77800` order submitted with ACK, canceled, reconciled with `executed_qty=0.000`, no fill, flat final position, restart repair passed, and mainnet canary disabled afterward. The run exposed a cancel payload ergonomics issue: the first cancel request omitted body `order_ref` and was rejected before a retry canceled the same order.
- Cancel endpoint body ergonomics fixed: `POST /api/live/orders/:order_ref/cancel` now uses the path `order_ref` as authoritative, accepts omitted or matching optional body `order_ref`, rejects mismatched body `order_ref`, and preserves confirmation gates.
- Post-fix safe-default smoke confirmed health/bootstrap/live-status/live-credentials/static frontend, MAINNET canary disabled, MAINNET auto stopped, previous MAINNET BTCUSDT orders still canceled, no MAINNET BTCUSDT fills, flat BTCUSDT account snapshot position, and no raw env secrets in smoke API payloads.
- Mainnet canary closure / operator handoff completed: evidence bundles were reviewed, final live state was smoke-checked with MAINNET canary disabled and no unexpected exposure, final status docs were aligned, and `docs/OPERATOR_HANDOFF.md` was created.
- Release-candidate cleanup completed: git/worktree hygiene, ignored artifact policy, generated-output policy, final safe-smoke status, and known residual risks were captured in `docs/FINAL_RC_SNAPSHOT.md`.
- Shareable RC UI cleanup completed: top safety summary, clearer LIVE ACCESS grouping, friendlier safety language, and frontend regression coverage for MAINNET auto blocked / canary disabled / preflight-not-execution / canceled-not-filled states.
- Shadow/reconciliation environment mismatch now blocks readiness, preview, and execution gates.

## Immediate Next Task

Review `docs/FINAL_RC_SNAPSHOT.md` and `docs/OPERATOR_HANDOFF.md`, then choose one bounded post-RC task. If another canary is ever requested, run a fresh dry-run checklist first and keep the canary flag disabled until a separate explicit execution task.

## Deferred Live Execution Work

- Broader mainnet enablement policy after the bounded manual canary evidence.
- Conditional/algo orders such as STOP, TAKE_PROFIT, and trailing orders.
- Hedge mode and multi-assets mode support if explicitly designed and tested.
- Portfolio-level exposure controls beyond the active symbol.
- Broker-grade audit/export reporting.
- Automated incident drill reporting and operator attestations beyond the current soak evidence bundle.
- Liquidation heatmap/liquidation-context module. It needs separate source-quality and semantics design, and it must not become a live decision layer until mainnet safety hardening is reviewed separately.

## Not-Now Items

- MAINNET auto-execution.
- Liquidation heatmap/liquidation-context panels, APIs, or strategy inputs.
- Plaintext secret storage.
- Treating paper-engine state as exchange-authoritative truth.
- Tauri packaging.
- Multi-user auth.
- Multi-symbol concurrent runtime.
- Strategy marketplace.
- Optimization engine.
