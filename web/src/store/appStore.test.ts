import { beforeEach, describe, expect, it } from "vitest";

import type { BootstrapPayload, Trade } from "../types";
import { useAppStore } from "./appStore";

const baseSnapshot: BootstrapPayload = {
  metadata: {
    app_name: "RelXen",
    version: "0.1.0",
    started_at: 1
  },
  runtime_status: {
    running: true,
    execution_mode: "paper",
    active_symbol: "BTCUSDT",
    timeframe: "1m",
    activity: null,
    last_error: null,
    started_at: 1
  },
  settings: {
    active_symbol: "BTCUSDT",
    available_symbols: ["BTCUSDT", "BTCUSDC"],
    timeframe: "1m",
    aso_length: 20,
    aso_mode: "both",
    leverage: 5,
    fee_rate: 0.0004,
    sizing_mode: "fixed_notional",
    fixed_notional: 250,
    initial_wallet_balance_by_quote: {
      USDT: 10000,
      USDC: 10000
    },
    paper_enabled: true,
    live_mode_visible: true,
    auto_restart_on_apply: true
  },
  allowed_symbols: ["BTCUSDT", "BTCUSDC"],
  active_symbol: "BTCUSDT",
  candles: [],
  aso_points: [],
  recent_signals: [],
  recent_trades: [],
  current_position: null,
  wallets: [],
  performance: {
    realized_pnl: 0,
    unrealized_pnl: 0,
    equity: 20000,
    return_pct: 0,
    trades: 0,
    closed_trades: 0,
    win_rate: 0,
    fees_paid: 0
  },
  connection_state: {
    status: "connected",
    status_since: 1,
    last_message_time: 1,
    reconnect_attempts: 0,
    resync_required: false,
    detail: "stream healthy"
  },
  live_status: {
    feature_visible: true,
    mode_preference: "paper",
    environment: "testnet",
    state: "credentials_missing",
    armed: false,
    active_credential: null,
    readiness: {
      state: "credentials_missing",
      environment: "testnet",
      active_credential: null,
      checks: [],
      blocking_reasons: ["no_active_credential"],
      warnings: ["testnet_environment"],
      account_snapshot: null,
      symbol_rules: null,
      can_arm: false,
      can_execute_live: false,
      refreshed_at: 1
    },
    reconciliation: {
      state: "credentials_missing",
      stream: {
        state: "stopped",
        environment: "testnet",
        listen_key_hint: null,
        status_since: 1,
        started_at: null,
        last_event_time: null,
        last_rest_sync_at: null,
        reconnect_attempts: 0,
        stale: false,
        detail: null
      },
      shadow: null,
      blocking_reasons: [],
      warnings: [],
      updated_at: 1
    },
    account_snapshot: null,
    symbol_rules: null,
    intent_preview: null,
    recent_preflights: [],
    execution: {
      state: "credentials_missing",
      environment: "testnet",
      can_submit: false,
      blocking_reasons: ["no_active_credential"],
      warnings: [],
      active_order: null,
      recent_orders: [],
      recent_fills: [],
      kill_switch_engaged: false,
      updated_at: 1
    },
    execution_availability: {
      can_execute_live: false,
      reason: "mainnet_execution_blocked",
      message: "MAINNET execution is blocked; TESTNET execution requires readiness gates."
    },
    updated_at: 1
  },
  system_metrics: {
    cpu_usage_percent: 0,
    memory_used_bytes: 0,
    memory_total_bytes: 0,
    task_count: 1,
    collected_at: 1
  },
  recent_logs: []
};

function resetStore() {
  useAppStore.setState({
    bootstrapped: false,
    metadata: null,
    runtimeStatus: null,
    settings: null,
    allowedSymbols: [],
    activeSymbol: null,
    candles: [],
    asoPoints: [],
    recentSignals: [],
    recentTrades: [],
    currentPosition: null,
    wallets: [],
    performance: null,
    connectionState: null,
    systemMetrics: null,
    recentLogs: [],
    liveStatus: null,
    toasts: [],
    chartVersion: 0,
    lastCandleUpdate: null,
    lastAsoUpdate: null,
    resyncRequestedAt: null
  });
}

function trade(id: string, timestamp: number): Trade {
  return {
    id,
    symbol: "BTCUSDT",
    quote_asset: "USDT",
    side: "long",
    action: "open",
    source: "signal",
    qty: 0.01,
    price: 100000,
    notional: 1000,
    entry_price: 100000,
    exit_price: null,
    fee_paid: 0.4,
    realized_pnl: 0,
    opened_at: timestamp,
    closed_at: null,
    timestamp
  };
}

describe("appStore trade events", () => {
  beforeEach(() => {
    resetStore();
    useAppStore.getState().setSnapshot(baseSnapshot);
  });

  it("appends realtime trade events incrementally and clears on reset", () => {
    useAppStore.getState().applyEvents([
      { type: "trade_appended", payload: trade("t-1", 1) },
      { type: "trade_appended", payload: trade("t-2", 2) }
    ]);

    expect(useAppStore.getState().recentTrades.map((item) => item.id)).toEqual(["t-1", "t-2"]);

    useAppStore.getState().applyEvents([{ type: "trade_history_reset" }]);

    expect(useAppStore.getState().recentTrades).toEqual([]);
  });
});
