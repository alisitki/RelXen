# Testnet Soak Runbook

## Purpose

This runbook defines the bounded TESTNET soak drill for RelXen live execution. It is an operator evidence workflow, not a new execution feature. It proves the existing credential, shadow, preview, preflight, TESTNET execution, cancel, flatten, kill-switch, restart-repair, reconnect-repair, and auto-execution paths under controlled conditions.

MAINNET canary must remain disabled during this drill.

## Preconditions

- Paper Mode V1 is runnable.
- `RELXEN_ENABLE_MAINNET_CANARY_EXECUTION=false` or unset.
- A Binance USD-M Futures TESTNET credential exists in the RelXen secure-store flow.
- The active credential environment is `testnet`.
- Active symbol is `BTCUSDT` or `BTCUSDC`.
- Account mode checks report one-way mode and single-asset mode.
- A conservative risk profile is configured.
- The operator understands that TESTNET execution still sends real TESTNET exchange orders.
- `curl` and `jq` are installed for evidence export.

If valid TESTNET credentials are not available, run the export/smoke portions only and mark real exchange scenarios as not exercised.

## Start Server

Use normal local runtime unless you intentionally need a separate database:

```sh
cargo run -p relxen-server
```

Confirm health:

```sh
curl http://localhost:3000/api/health
curl http://localhost:3000/api/live/status
```

## Evidence Directory

Evidence exports are written under:

```sh
artifacts/testnet-soak/<timestamp>/
```

That directory is intentionally ignored by git. Do not commit raw drill artifacts unless they have been reviewed for account identifiers and operational sensitivity.

Manual export:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/export_live_evidence.sh
```

Guided drill capture:

```sh
RELXEN_BASE_URL=http://localhost:3000 scripts/run_testnet_soak.sh
```

The guided script does not create credentials, arm execution, or place orders. It pauses for the operator to perform each action through the UI/API and captures checkpoints.

## Required Drill Scenarios

### 1. Credential / Readiness / Shadow Bootstrap

Steps:

- Select the TESTNET credential.
- Validate it.
- Refresh readiness.
- Start shadow sync.
- Confirm `/api/live/status` shows a fresh, coherent shadow state.

Pass criteria:

- Credential validation succeeds.
- Shadow stream is running or the status gives a typed safe blocker.
- Blocking reasons do not include missing rules/account/mode checks when the scenario is expected to proceed.

### 2. Manual Preview + Preflight Sanity

Steps:

- Build a `MARKET` or `LIMIT` preview.
- Run testnet preflight.
- Verify the UI says preflight passed or failed without claiming order placement.

Pass criteria:

- Preflight result is persisted under `/api/live/preflights`.
- No live order is created by preflight alone.

### 3. Real TESTNET Manual Execution

Steps:

- Confirm the displayed preview is fresh.
- Submit one bounded TESTNET order through the UI confirmation.
- Watch order state through `/api/live/orders` and the UI.

Pass criteria:

- The submission enters an ACK/pending or working state first.
- Filled/canceled/final status appears only after authoritative user-data or REST repair state.
- No local fill is invented from the submit response.

### 4. Cancel Flow

Steps:

- Prefer a small `LIMIT` order away from the mark price so it remains working long enough to cancel.
- Submit cancel for the RelXen-created order.
- Observe user-data or REST repair result.

Pass criteria:

- Cancel request is accepted or rejected with a typed reason.
- Final canceled state is authoritative.

If the order fills immediately, record that cancel was not naturally exercisable and repeat with a safer non-marketable limit if account conditions allow.

### 5. Flatten Flow

Steps:

- If a deterministic active-symbol TESTNET position exists, run flatten.
- RelXen should cancel active-symbol open orders first, then submit a reduce-only MARKET close when safe.

Pass criteria:

- Flatten fails closed when shadow position state is missing or ambiguous.
- If flatten submits, final position/fill state is reconciled authoritatively.

### 6. Kill Switch

Steps:

- Engage kill switch.
- Attempt to build/submit a new live execution action.
- Release kill switch after verifying the block.

Pass criteria:

- New submissions are blocked immediately while kill switch is engaged.
- Cancel/flatten remain available only if deterministic and safe.
- Release does not bypass other gates.

### 7. Restart / Recent-Window Repair

Steps:

- Ensure recent order/fill state exists.
- Stop the server.
- Restart against the same SQLite database.
- Refresh live status and orders/fills.

Pass criteria:

- Recent orders/fills remain visible.
- Incomplete or ambiguous records are repaired from the recent window or marked degraded.
- No duplicate order is submitted on restart.

Repair is intentionally recent-window only because Binance order/trade query retention is finite.

### 8. Reconnect / Repair

Steps:

- Stop and restart shadow sync, or simulate a stream interruption through normal operator controls.
- Observe reconnect and repair.

Pass criteria:

- State transitions to syncing/degraded/recovered truthfully.
- If coherence cannot be proven, new submissions remain blocked.

### 9. Auto-Executor Proof

Steps:

- Start TESTNET auto mode only after shadow/rules/account state is fresh.
- Wait for a natural closed-candle ASO signal within the bounded window.
- Capture whether exactly one order was submitted for the signal.

Pass criteria:

- Auto mode never trades from unfinished candles.
- Same signal/open time is not submitted twice across reconnect or restart.
- If no natural signal appears, mark the scenario not exercised. Do not add an unsafe synthetic signal to a normal runtime.

### 10. Recent-Window Repair Honesty

Steps:

- Review `repair_events.json`, order state, and logs.
- Confirm any repair claim references only recent-window-supported evidence.

Pass criteria:

- Report does not claim infinite historical recovery.
- Older ambiguity remains degraded/operator-reviewed.

## Evidence Bundle Files

The export script writes:

- `manifest.json`
- `session_summary.md`
- `timeline.ndjson`
- `live_status_before.json`
- `live_status_after.json`
- `credentials.json`
- `bootstrap_snapshot.json`
- `orders.json`
- `fills.json`
- `preflights.json`
- `blocking_reasons.json`
- `repair_events.json`
- `logs.json`
- optional guided `checkpoints/` snapshots

`credentials.json` contains masked credential summaries only. It must never contain raw API keys or secrets.

Artifacts must distinguish:

- preview
- preflight
- submit ACK
- working/open
- partial fill
- fill
- cancel
- flatten
- restart repair
- reconnect repair

## After The Drill

Update `docs/LATEST_TESTNET_SOAK_REPORT.md` with:

- what was actually exercised for real
- what was mocked or only smoke-tested
- pass/fail/not-exercised result per scenario
- bugs found and fixed
- bugs deferred
- current mainnet canary go/no-go recommendation
- evidence bundle path

Do not recommend MAINNET canary until a real TESTNET evidence bundle covers manual execution, kill switch, restart repair, reconnect repair, and either auto-execution or a documented no-signal timeout.
