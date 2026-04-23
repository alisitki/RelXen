// @vitest-environment jsdom
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api/client")>();
  return {
    ...actual,
    armLive: vi.fn(),
    cancelAllLiveOrders: vi.fn(),
    cancelLiveOrder: vi.fn(),
    createLiveCredential: vi.fn(),
    deleteLiveCredential: vi.fn(),
    disarmLive: vi.fn(),
    executeLivePreview: vi.fn(),
    flattenLivePosition: vi.fn(),
    getLiveIntentPreview: vi.fn(),
    listLiveCredentials: vi.fn(),
    liveStartCheck: vi.fn(),
    refreshLiveReadiness: vi.fn(),
    refreshLiveShadow: vi.fn(),
    runLivePreflight: vi.fn(),
    selectLiveCredential: vi.fn(),
    setLiveModePreference: vi.fn(),
    startLiveShadow: vi.fn(),
    stopLiveShadow: vi.fn(),
    updateLiveCredential: vi.fn(),
    validateLiveCredential: vi.fn()
  };
});

import {
  armLive,
  cancelAllLiveOrders,
  cancelLiveOrder,
  createLiveCredential,
  disarmLive,
  executeLivePreview,
  flattenLivePosition,
  getLiveIntentPreview,
  listLiveCredentials,
  liveStartCheck,
  refreshLiveReadiness,
  refreshLiveShadow,
  runLivePreflight,
  startLiveShadow,
  stopLiveShadow,
  validateLiveCredential
} from "../api/client";
import { useAppStore } from "../store/appStore";
import { makeBootstrapSnapshot, makeLiveStatus, renderWithClient, resetAppStore } from "../test/helpers";
import type { LiveCredentialSummary, LiveOrderPreview, LiveOrderRecord, LiveStatusSnapshot } from "../types";
import { LiveAccessPanel } from "./LiveAccessPanel";
import { ToastViewport } from "./ToastViewport";

const credential: LiveCredentialSummary = {
  id: "cred-1",
  alias: "testnet-readonly",
  environment: "testnet",
  api_key_hint: "abcd...7890",
  validation_status: "valid",
  last_validated_at: 1_000,
  last_validation_error: null,
  is_active: true,
  created_at: 1,
  updated_at: 1_000
};

type ReadyStatusOverrides = Partial<Omit<LiveStatusSnapshot, "readiness" | "reconciliation">> & {
  readiness?: Partial<LiveStatusSnapshot["readiness"]>;
  reconciliation?: Partial<Omit<LiveStatusSnapshot["reconciliation"], "stream">> & {
    stream?: Partial<LiveStatusSnapshot["reconciliation"]["stream"]>;
  };
};

function readyStatus(overrides: ReadyStatusOverrides = {}): LiveStatusSnapshot {
  const { readiness, reconciliation, ...statusOverrides } = overrides;
  const accountSnapshot: LiveStatusSnapshot["account_snapshot"] = {
    environment: "testnet",
    can_trade: true,
    multi_assets_margin: false,
    total_wallet_balance: 1000,
    total_margin_balance: 1000,
    available_balance: 900,
    assets: [],
    positions: [],
    fetched_at: 2_000
  };
  const symbolRules: LiveStatusSnapshot["symbol_rules"] = {
    environment: "testnet",
    symbol: "BTCUSDT",
    status: "TRADING",
    base_asset: "BTC",
    quote_asset: "USDT",
    price_precision: 2,
    quantity_precision: 3,
    filters: {
      tick_size: 0.1,
      step_size: 0.001,
      min_qty: 0.001,
      min_notional: 100
    },
    fetched_at: 2_000
  };
  return makeLiveStatus({
    mode_preference: "live_read_only",
    state: "ready_read_only",
    active_credential: credential,
    readiness: {
      state: "ready_read_only",
      environment: "testnet",
      active_credential: credential,
      blocking_reasons: [],
      warnings: ["testnet_environment"],
      checks: [{ code: "credential_valid", passed: true, message: "credential valid" }],
      account_snapshot: accountSnapshot,
      symbol_rules: symbolRules,
      can_arm: true,
      can_execute_live: false,
      refreshed_at: 2_000,
      ...readiness
    },
    account_snapshot: accountSnapshot,
    symbol_rules: symbolRules,
    reconciliation: {
      state: statusOverrides.state ?? "ready_read_only",
      blocking_reasons: [],
      warnings: [],
      ...reconciliation
    },
    execution: {
      state: "execution_blocked",
      environment: "testnet",
      can_submit: false,
      blocking_reasons: [],
      warnings: [],
      active_order: null,
      recent_orders: [],
      recent_fills: [],
      kill_switch_engaged: false,
      updated_at: 2_000
    },
    ...statusOverrides
  });
}

describe("live access panel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetAppStore();
    useAppStore.getState().setSnapshot(makeBootstrapSnapshot());
    vi.mocked(listLiveCredentials).mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
  });

  it("creates credentials without echoing raw secrets", async () => {
    const user = userEvent.setup();
    vi.mocked(listLiveCredentials).mockResolvedValueOnce([]).mockResolvedValueOnce([credential]);
    vi.mocked(createLiveCredential).mockResolvedValueOnce(credential);

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    await user.type(screen.getByLabelText("Live Alias"), "testnet-readonly");
    await user.type(screen.getByLabelText("Live API Key"), "abcd1234567890");
    await user.type(screen.getByLabelText("Live API Secret"), "super-secret-value");
    await user.click(screen.getByRole("button", { name: "Create Credential" }));

    await waitFor(() =>
      expect(createLiveCredential).toHaveBeenCalledWith({
        alias: "testnet-readonly",
        environment: "testnet",
        api_key: "abcd1234567890",
        api_secret: "super-secret-value"
      })
    );
    expect(await screen.findByText("Live credential metadata saved.")).toBeTruthy();
    expect((screen.getByLabelText("Live API Secret") as HTMLInputElement).value).toBe("");
    expect(document.body.textContent).not.toContain("super-secret-value");
  });

  it("renders readiness blockers, warnings, rules, and account summary", () => {
    useAppStore.getState().setLiveStatus(
      readyStatus({
        state: "validation_failed",
        readiness: {
          state: "validation_failed",
          active_credential: credential,
          blocking_reasons: ["validation_failed", "symbol_rules_missing"],
          warnings: ["testnet_environment", "validation_stale"],
          can_arm: false
        }
      })
    );
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);

    renderWithClient(<LiveAccessPanel />);

    expect(screen.getByText("VALIDATION FAILED")).toBeTruthy();
    expect(screen.getByText("validation_failed, symbol_rules_missing")).toBeTruthy();
    expect(screen.getByText("testnet_environment, validation_stale")).toBeTruthy();
    expect(screen.getByText(/BTCUSDT TRADING tick 0.1/)).toBeTruthy();
    expect(screen.getByText(/available 900/)).toBeTruthy();
  });

  it("renders validation success and validation failure feedback", async () => {
    const user = userEvent.setup();
    useAppStore.getState().setLiveStatus(readyStatus());
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);
    vi.mocked(validateLiveCredential)
      .mockResolvedValueOnce({
        credential_id: "cred-1",
        environment: "testnet",
        status: "valid",
        validated_at: 3_000,
        message: null
      })
      .mockResolvedValueOnce({
        credential_id: "cred-1",
        environment: "testnet",
        status: "invalid_signature",
        validated_at: 4_000,
        message: "Invalid API signature."
      });

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Validate" }));
    expect(await screen.findByText("Live credential validated.")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Validate" }));
    expect(await screen.findByText("Invalid API signature.")).toBeTruthy();
  });

  it("arms, disarms, and keeps live start hard-blocked", async () => {
    const user = userEvent.setup();
    const armedStatus = readyStatus({ state: "armed_read_only", armed: true, readiness: { state: "armed_read_only" } });
    useAppStore.getState().setLiveStatus(readyStatus());
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);
    vi.mocked(armLive).mockResolvedValueOnce(armedStatus);
    vi.mocked(disarmLive).mockResolvedValueOnce(readyStatus({ armed: false, state: "ready_read_only" }));
    vi.mocked(liveStartCheck).mockResolvedValueOnce({
      allowed: false,
      blocking_reasons: ["execution_not_implemented"],
      message: "Autonomous live start is not implemented; use manual TESTNET execution controls.",
      readiness: readyStatus().readiness
    });

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Arm Read-Only" }));
    expect(await screen.findByText("Live read-only mode armed.")).toBeTruthy();
    expect(screen.getAllByText("ARMED READ-ONLY").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Start Live Check" }));
    expect(
      await screen.findByText("Autonomous live start is not implemented; use manual TESTNET execution controls.")
    ).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Disarm" }));
    expect(await screen.findByText("Live mode disarmed.")).toBeTruthy();
  });

  it("refreshes readiness and keeps the paper/live distinction explicit", async () => {
    const user = userEvent.setup();
    const status = readyStatus();
    useAppStore.getState().setLiveStatus(status);
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);
    vi.mocked(refreshLiveReadiness).mockResolvedValueOnce(status);

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    expect(screen.getByRole("button", { name: "PAPER MODE" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "LIVE READ-ONLY" })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Refresh Readiness" }));
    expect(await screen.findByText("Live readiness refreshed.")).toBeTruthy();
    expect(screen.getByText("READY READ-ONLY")).toBeTruthy();
  });

  it("starts and stops live shadow sync with explicit stream state", async () => {
    const user = userEvent.setup();
    const shadowStatus = readyStatus({
      state: "shadow_running",
      reconciliation: {
        state: "shadow_running",
        stream: {
          state: "running",
          last_event_time: 5_000,
          last_rest_sync_at: 4_000,
          detail: "user data stream healthy"
        },
        shadow: {
          environment: "testnet",
          balances: [
            {
              asset: "USDT",
              wallet_balance: "1000",
              cross_wallet_balance: "1000",
              balance_change: null,
              updated_at: 5_000
            }
          ],
          positions: [],
          open_orders: [],
          can_trade: true,
          multi_assets_margin: false,
          position_mode: "one_way",
          last_event_time: 5_000,
          last_rest_sync_at: 4_000,
          updated_at: 5_000,
          ambiguous: false,
          divergence_reasons: []
        },
        blocking_reasons: [],
        warnings: [],
        updated_at: 5_000
      }
    });
    useAppStore.getState().setLiveStatus(readyStatus());
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);
    vi.mocked(startLiveShadow).mockResolvedValueOnce(shadowStatus);
    vi.mocked(refreshLiveShadow).mockResolvedValueOnce(shadowStatus);
    vi.mocked(stopLiveShadow).mockResolvedValueOnce(
      readyStatus({
        state: "ready_read_only",
        reconciliation: {
          state: "ready_read_only",
          stream: { state: "stopped", detail: "shadow stopped by operator" }
        }
      })
    );

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Start Shadow Sync" }));
    expect(await screen.findByText("Live shadow sync started.")).toBeTruthy();
    expect(screen.getByText("LIVE SHADOW ACTIVE")).toBeTruthy();
    expect(screen.getByText(/RUNNING · FRESH/)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Refresh Shadow" }));
    expect(await screen.findByText("Live shadow state refreshed.")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Stop Shadow Sync" }));
    expect(await screen.findByText("Live shadow sync stopped.")).toBeTruthy();
  });

  it("renders intent preview and successful preflight without implying order placement", async () => {
    const user = userEvent.setup();
    useAppStore.getState().setLiveStatus(
      readyStatus({
        state: "shadow_running",
        reconciliation: {
          state: "shadow_running",
          stream: { state: "running", last_event_time: 5_000, last_rest_sync_at: 4_000 }
        }
      })
    );
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);
    vi.mocked(getLiveIntentPreview).mockResolvedValueOnce({
      built_at: 6_000,
      intent: {
        id: "intent-1",
        environment: "testnet",
        symbol: "BTCUSDT",
        side: "BUY",
        order_type: "MARKET",
        quantity: "0.010",
        price: null,
        reduce_only: false,
        time_in_force: null,
        source_signal_id: "signal-1",
        source_open_time: 5_000,
        reason: "closed candle BUY signal",
        exchange_payload: {
          symbol: "BTCUSDT",
          side: "BUY",
          type: "MARKET",
          quantity: "0.010"
        },
        sizing: {
          requested_notional: "250",
          available_balance: "900",
          leverage: "5",
          required_margin: "50",
          raw_quantity: "0.0104",
          rounded_quantity: "0.010",
          estimated_notional: "240"
        },
        validation_notes: ["quantity rounded down to step size"],
        blocking_reasons: [],
        can_preflight: true,
        intent_hash: "intent-hash-1",
        can_execute_now: false,
        built_at: 6_000
      },
      blocking_reasons: [],
      validation_errors: [],
      message: "Intent preview built. Execution remains disabled."
    });
    vi.mocked(runLivePreflight).mockResolvedValueOnce({
      id: "preflight-1",
      credential_id: "cred-1",
      environment: "testnet",
      symbol: "BTCUSDT",
      side: "BUY",
      order_type: "MARKET",
      payload: {
        symbol: "BTCUSDT",
        side: "BUY",
        type: "MARKET",
        quantity: "0.010"
      },
      accepted: true,
      exchange_error_code: null,
      exchange_error_message: null,
      local_blocking_reason: null,
      source_signal_id: "signal-1",
      message: "PREFLIGHT PASSED. No order was placed.",
      created_at: 7_000
    });

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Build Preview" }));
    expect(await screen.findByText("Live order intent preview built.")).toBeTruthy();
    expect(screen.getByText(/BUY MARKET BTCUSDT qty 0.010/)).toBeTruthy();
    expect(screen.getByText(/TESTNET ONLY/)).toBeTruthy();
    expect(screen.getAllByText(/EXECUTION BLOCKED/).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Run Preflight" }));
    expect(await screen.findAllByText("PREFLIGHT PASSED. No order was placed.")).toHaveLength(2);
    expect(document.body.textContent).not.toContain("ORDER PLACED");
  });

  it("submits, cancels, and flattens only through explicit TESTNET confirmations", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);

    const intentPreview: LiveOrderPreview = {
      built_at: 6_000,
      intent: {
        id: "intent-1",
        intent_hash: "intent-hash-1",
        environment: "testnet",
        symbol: "BTCUSDT",
        side: "BUY",
        order_type: "MARKET",
        quantity: "0.010",
        price: null,
        reduce_only: false,
        time_in_force: null,
        source_signal_id: "signal-1",
        source_open_time: 5_000,
        reason: "closed candle BUY signal",
        exchange_payload: {
          symbol: "BTCUSDT",
          side: "BUY",
          type: "MARKET",
          quantity: "0.010"
        },
        sizing: {
          requested_notional: "250",
          available_balance: "900",
          leverage: "5",
          required_margin: "50",
          raw_quantity: "0.0104",
          rounded_quantity: "0.010",
          estimated_notional: "240"
        },
        validation_notes: [],
        blocking_reasons: [],
        can_preflight: true,
        can_execute_now: true,
        built_at: 6_000
      },
      blocking_reasons: [],
      validation_errors: [],
      message: "TESTNET order intent is ready for explicit operator execution."
    };
    const workingOrder: LiveOrderRecord = {
      id: "order-1",
      credential_id: "cred-1",
      environment: "testnet",
      symbol: "BTCUSDT",
      side: "BUY",
      order_type: "MARKET",
      status: "working",
      client_order_id: "rx_exec_1",
      exchange_order_id: "123",
      quantity: "0.010",
      price: null,
      executed_qty: "0",
      avg_price: null,
      reduce_only: false,
      time_in_force: null,
      intent_id: "intent-1",
      intent_hash: "intent-hash-1",
      source_signal_id: "signal-1",
      reason: "closed candle BUY signal",
      payload: {
        symbol: "BTCUSDT",
        side: "BUY",
        type: "MARKET",
        quantity: "0.010"
      },
      last_error: null,
      submitted_at: 7_000,
      updated_at: 7_000
    };
    const canceledOrder: LiveOrderRecord = {
      ...workingOrder,
      status: "canceled",
      updated_at: 8_000
    };
    const flattenOrder: LiveOrderRecord = {
      ...workingOrder,
      id: "order-flat",
      side: "SELL",
      status: "submit_pending",
      reduce_only: true,
      reason: "manual flatten",
      updated_at: 9_000
    };

    useAppStore.getState().setLiveStatus(
      readyStatus({
        state: "testnet_execution_ready",
        armed: true,
        intent_preview: intentPreview,
        reconciliation: {
          state: "shadow_running",
          stream: { state: "running", last_event_time: 5_000, last_rest_sync_at: 4_000 },
          shadow: {
            environment: "testnet",
            balances: [],
            positions: [
              {
                symbol: "BTCUSDT",
                position_side: "BOTH",
                position_amt: "0.010",
                entry_price: "24000",
                unrealized_pnl: "0",
                margin_type: null,
                isolated_wallet: null,
                updated_at: 5_000
              }
            ],
            open_orders: [],
            can_trade: true,
            multi_assets_margin: false,
            position_mode: "one_way",
            last_event_time: 5_000,
            last_rest_sync_at: 4_000,
            updated_at: 5_000,
            ambiguous: false,
            divergence_reasons: []
          }
        },
        execution: {
          state: "testnet_execution_ready",
          environment: "testnet",
          can_submit: true,
          blocking_reasons: [],
          warnings: [],
          active_order: null,
          recent_orders: [],
          recent_fills: [],
          kill_switch_engaged: false,
          updated_at: 6_000
        },
        execution_availability: {
          can_execute_live: true,
          reason: "execution_not_implemented",
          message: "TESTNET execution is ready after all gates pass."
        }
      })
    );
    vi.mocked(listLiveCredentials).mockResolvedValue([credential]);
    vi.mocked(executeLivePreview).mockResolvedValueOnce({
      accepted: true,
      order: workingOrder,
      blocking_reason: null,
      message: "TESTNET order submitted; waiting for authoritative reconciliation.",
      created_at: 7_000
    });
    vi.mocked(cancelLiveOrder).mockResolvedValueOnce({
      accepted: true,
      order: canceledOrder,
      blocking_reason: null,
      message: "TESTNET cancel submitted.",
      created_at: 8_000
    });
    vi.mocked(cancelAllLiveOrders).mockResolvedValueOnce([
      {
        accepted: true,
        order: canceledOrder,
        blocking_reason: null,
        message: "TESTNET cancel submitted.",
        created_at: 8_000
      }
    ]);
    vi.mocked(flattenLivePosition).mockResolvedValueOnce({
      accepted: true,
      canceled_orders: [],
      flatten_order: flattenOrder,
      blocking_reason: null,
      message: "TESTNET flatten submitted.",
      created_at: 9_000
    });

    renderWithClient(
      <>
        <LiveAccessPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Execute TESTNET Preview" }));
    expect(window.confirm).toHaveBeenCalledWith(expect.stringContaining("real TESTNET Binance Futures order"));
    expect(executeLivePreview).toHaveBeenCalledWith({ intent_id: "intent-1", confirm_testnet: true });
    expect(await screen.findByText("TESTNET order submission accepted.")).toBeTruthy();
    expect(screen.getByText(/BUY MARKET BTCUSDT qty 0.010 · WORKING/)).toBeTruthy();
    expect(document.body.textContent).not.toContain("FILLED");

    await user.click(screen.getByRole("button", { name: "Cancel Open TESTNET Order" }));
    expect(cancelLiveOrder).toHaveBeenCalledWith("order-1", true);
    expect(await screen.findByText("TESTNET cancel submitted.")).toBeTruthy();
    expect(screen.getByText(/CANCELED/)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Flatten TESTNET Position" }));
    expect(flattenLivePosition).toHaveBeenCalledWith(true);
    expect(await screen.findByText("TESTNET flatten submitted.")).toBeTruthy();
  });
});
