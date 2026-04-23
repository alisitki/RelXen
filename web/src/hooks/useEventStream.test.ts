import { QueryClient } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { BootstrapPayload } from "../types";
import { useAppStore } from "../store/appStore";
import { makeLiveStatus } from "../test/helpers";
import { processEventBatch } from "./useEventStream";

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
  live_status: makeLiveStatus(),
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
    liveStatus: null,
    systemMetrics: null,
    recentLogs: [],
    toasts: [],
    chartVersion: 0,
    lastCandleUpdate: null,
    lastAsoUpdate: null,
    resyncRequestedAt: null
  });
}

describe("processEventBatch", () => {
  beforeEach(() => {
    resetStore();
    useAppStore.getState().setSnapshot(baseSnapshot);
  });

  it("triggers bootstrap reload on resync_required", async () => {
    const queryClient = new QueryClient();
    const refetchSpy = vi.spyOn(queryClient, "refetchQueries").mockResolvedValue();

    await processEventBatch(
      [{ type: "resync_required", payload: { reason: "socket closed" } }],
      useAppStore.getState().applyEvents,
      queryClient
    );

    expect(refetchSpy).toHaveBeenCalledWith({ queryKey: ["bootstrap"], type: "active" });
    expect(useAppStore.getState().resyncRequestedAt).not.toBeNull();
    expect(useAppStore.getState().connectionState?.status).toBe("stale");
    expect(useAppStore.getState().toasts.at(-1)?.message).toBe("Data stream went stale. Reloading snapshot.");
  });
});
