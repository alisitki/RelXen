import { describe, expect, it, vi } from "vitest";

import { ApiClientError } from "../api/client";
import { notifyCommandError, notifyCommandSuccess } from "./commandFeedback";

describe("command feedback", () => {
  it("uses explicit success text for settings commands", () => {
    const addToast = vi.fn();

    notifyCommandSuccess(addToast, "settings_apply");

    expect(addToast).toHaveBeenCalledWith("Settings applied.", "info");
  });

  it("uses normalized history error text for settings commands", () => {
    const addToast = vi.fn();

    notifyCommandError(
      addToast,
      "settings_apply",
      new ApiClientError("history failed: expected 2 closed candles but found 1", 422, "history")
    );

    expect(addToast).toHaveBeenCalledWith(
      "History rebuild failed. Runtime kept the last valid state.",
      "error"
    );
  });

  it("hides raw internal server detail behind the command fallback", () => {
    const addToast = vi.fn();

    notifyCommandError(
      addToast,
      "paper_reset",
      new ApiClientError("controlled clear_trades failure", 500, "internal")
    );

    expect(addToast).toHaveBeenCalledWith("Failed to reset paper account.", "error");
  });
});
