import { afterEach, describe, expect, it, vi } from "vitest";

import { cancelLiveOrder, cancelLiveOrderWithPayload } from "./client";

function mockFetchResponse() {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(
      JSON.stringify({
        accepted: true,
        order: null,
        blocking_reason: null,
        message: "cancel submitted",
        created_at: 1
      }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" }
      }
    )
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("api client live cancel", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends cancel target in the path without duplicating order_ref in the body", async () => {
    const fetchMock = mockFetchResponse();

    await cancelLiveOrder("rx-order-1", true);

    expect(fetchMock).toHaveBeenCalledWith("/api/live/orders/rx-order-1/cancel", {
      headers: { "Content-Type": "application/json" },
      method: "POST",
      body: JSON.stringify({ confirm_testnet: true })
    });
  });

  it("keeps mainnet confirmation payload without adding order_ref", async () => {
    const fetchMock = mockFetchResponse();

    await cancelLiveOrderWithPayload("rx-order-2", {
      confirm_testnet: false,
      confirm_mainnet_canary: true,
      confirmation_text: "CANCEL MAINNET BTCUSDT rx-order-2"
    });

    expect(fetchMock).toHaveBeenCalledWith("/api/live/orders/rx-order-2/cancel", {
      headers: { "Content-Type": "application/json" },
      method: "POST",
      body: JSON.stringify({
        confirm_testnet: false,
        confirm_mainnet_canary: true,
        confirmation_text: "CANCEL MAINNET BTCUSDT rx-order-2"
      })
    });
  });
});
