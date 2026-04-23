import { describe, expect, it } from "vitest";

import { connectionLabel, connectionWarning, runtimeActivityLabel, STALE_WARNING_AFTER_MS } from "./connection";
import type { ConnectionState } from "../types";

const baseConnection: ConnectionState = {
  status: "connected",
  status_since: 1_000,
  last_message_time: 1_000,
  reconnect_attempts: 0,
  resync_required: false,
  detail: "stream healthy"
};

describe("connection helpers", () => {
  it("formats reconnecting and stale ages deterministically", () => {
    expect(
      connectionLabel(
        {
          ...baseConnection,
          status: "reconnecting",
          status_since: 1_000
        },
        9_000
      )
    ).toBe("RECONNECTING 8s");

    expect(
      connectionLabel(
        {
          ...baseConnection,
          status: "stale",
          status_since: 1_000
        },
        15_000
      )
    ).toBe("STALE 14s");
  });

  it("emits a warning when the stream has been stale too long", () => {
    expect(
      connectionWarning(
        {
          ...baseConnection,
          status: "stale",
          status_since: 5_000
        },
        5_000 + STALE_WARNING_AFTER_MS + 1
      )
    ).toBe("Feed has been stale too long. Bootstrap reload may be required.");
  });

  it("formats runtime activity states explicitly", () => {
    expect(runtimeActivityLabel("history_sync")).toBe("HISTORY SYNC");
    expect(runtimeActivityLabel("rebuilding")).toBe("REBUILDING");
    expect(runtimeActivityLabel(null)).toBeNull();
  });
});
