// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api/client")>();
  return {
    ...actual,
    closeAllPaper: vi.fn(),
    putSettings: vi.fn(),
    resetPaper: vi.fn(),
    startRuntime: vi.fn(),
    stopRuntime: vi.fn()
  };
});

import { ApiClientError, putSettings, resetPaper } from "../api/client";
import { useAppStore } from "../store/appStore";
import { makeBootstrapSnapshot, makeTrade, renderWithClient, resetAppStore } from "../test/helpers";
import { ConnectionPanel } from "./ConnectionPanel";
import { ControlPanel } from "./ControlPanel";
import { RiskPanel } from "./RiskPanel";
import { ToastViewport } from "./ToastViewport";
import { TradeHistoryPanel } from "./TradeHistoryPanel";

describe("release-gate operator UI", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetAppStore();
    useAppStore.getState().setSnapshot(makeBootstrapSnapshot());
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("surfaces a typed settings rebuild failure without applying stale local state", async () => {
    const user = userEvent.setup();
    vi.mocked(putSettings).mockRejectedValueOnce(
      new ApiClientError("history failed: expected contiguous candles", 422, "history")
    );

    renderWithClient(
      <>
        <ControlPanel />
        <ToastViewport />
      </>
    );

    await user.selectOptions(screen.getByLabelText("Timeframe"), "5m");
    await user.click(screen.getByRole("button", { name: "Apply" }));

    expect(
      await screen.findByText("History rebuild failed. Runtime kept the last valid state.")
    ).toBeTruthy();
    expect(useAppStore.getState().settings?.timeframe).toBe("1m");
    expect((screen.getByLabelText("Timeframe") as HTMLSelectElement).value).toBe("5m");
    expect(screen.queryByText("Settings applied.")).toBeNull();
  });

  it("applies settings successfully and renders success feedback", async () => {
    const user = userEvent.setup();
    vi.mocked(putSettings).mockResolvedValueOnce(
      makeBootstrapSnapshot({
        settings: { timeframe: "5m" },
        runtime_status: { timeframe: "5m" }
      })
    );

    renderWithClient(
      <>
        <ControlPanel />
        <ToastViewport />
      </>
    );

    await user.selectOptions(screen.getByLabelText("Timeframe"), "5m");
    await user.click(screen.getByRole("button", { name: "Apply" }));

    expect(await screen.findByText("Settings applied.")).toBeTruthy();
    expect(useAppStore.getState().settings?.timeframe).toBe("5m");
  });

  it("normalizes paper reset failures and avoids false success feedback", async () => {
    const user = userEvent.setup();
    vi.mocked(resetPaper).mockRejectedValueOnce(
      new ApiClientError("controlled clear_trades failure", 500, "internal")
    );

    renderWithClient(
      <>
        <RiskPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Reset Paper" }));

    expect(await screen.findByText("Failed to reset paper account.")).toBeTruthy();
    expect(screen.queryByText("Paper account reset.")).toBeNull();
  });

  it("renders paper reset success feedback", async () => {
    const user = userEvent.setup();
    vi.mocked(resetPaper).mockResolvedValueOnce(makeBootstrapSnapshot());

    renderWithClient(
      <>
        <RiskPanel />
        <ToastViewport />
      </>
    );

    await user.click(screen.getByRole("button", { name: "Reset Paper" }));

    expect(await screen.findByText("Paper account reset.")).toBeTruthy();
  });

  it("renders deterministic runtime and connection status text", () => {
    vi.useFakeTimers();
    vi.setSystemTime(20_000);
    useAppStore.getState().setSnapshot(
      makeBootstrapSnapshot({
        runtime_status: { activity: "history_sync" },
        connection_state: {
          status: "stale",
          status_since: 1_000,
          last_message_time: 1_000
        }
      })
    );

    render(<ConnectionPanel />);

    expect(screen.getByText("STALE 19s")).toBeTruthy();
    expect(screen.getByText("HISTORY SYNC")).toBeTruthy();
    expect(screen.getByText("Feed has been stale too long. Bootstrap reload may be required.")).toBeTruthy();

    cleanup();
    resetAppStore();
    vi.setSystemTime(9_000);
    useAppStore.getState().setSnapshot(
      makeBootstrapSnapshot({
        runtime_status: { activity: "rebuilding" },
        connection_state: {
          status: "reconnecting",
          status_since: 1_000,
          last_message_time: 1_000
        }
      })
    );

    render(<ConnectionPanel />);

    expect(screen.getByText("RECONNECTING 8s")).toBeTruthy();
    expect(screen.getByText("REBUILDING")).toBeTruthy();

    cleanup();
    resetAppStore();
    useAppStore.getState().setSnapshot(makeBootstrapSnapshot());

    render(<ConnectionPanel />);

    expect(screen.getByText("CONNECTED")).toBeTruthy();
    expect(screen.getByText("STEADY")).toBeTruthy();
  });

  it("renders websocket-appended trades incrementally", async () => {
    render(<TradeHistoryPanel />);

    expect(screen.getByText("No paper trades recorded yet.")).toBeTruthy();

    act(() => {
      useAppStore.getState().applyEvents([{ type: "trade_appended", payload: makeTrade("trade-1", 1_000) }]);
    });

    await waitFor(() => expect(screen.getByText("OPEN · ▲ LONG")).toBeTruthy());
    expect(screen.getByText("SIGNAL")).toBeTruthy();
    expect(screen.getByText("BTCUSDT")).toBeTruthy();
  });
});
