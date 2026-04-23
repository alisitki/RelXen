import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";

import { notifyCommandError } from "../lib/commandFeedback";
import { useAppStore } from "../store/appStore";
import type { OutboundEvent } from "../types";

export async function processEventBatch(
  events: OutboundEvent[],
  applyEvents: (events: OutboundEvent[]) => void,
  queryClient: QueryClient
): Promise<void> {
  if (events.length === 0) {
    return;
  }

  applyEvents(events);
  if (events.some((event) => event.type === "resync_required")) {
    try {
      await queryClient.refetchQueries({ queryKey: ["bootstrap"], type: "active" });
    } catch (error) {
      notifyCommandError(useAppStore.getState().addToast, "bootstrap_reload", error);
    }
  }
}

export function useEventStream(enabled: boolean): void {
  const queryClient = useQueryClient();
  const applyEvents = useAppStore((state) => state.applyEvents);
  const queueRef = useRef<OutboundEvent[]>([]);
  const frameRef = useRef<number | null>(null);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(`${protocol}//${window.location.host}/api/ws`);
    let closedByEffect = false;

    const flush = () => {
      frameRef.current = null;
      const batch = queueRef.current;
      queueRef.current = [];
      void processEventBatch(batch, applyEvents, queryClient);
    };

    socket.onmessage = (message) => {
      const event = JSON.parse(message.data) as OutboundEvent;
      queueRef.current.push(event);
      if (frameRef.current === null) {
        frameRef.current = window.requestAnimationFrame(flush);
      }
    };

    socket.onclose = () => {
      if (closedByEffect) {
        return;
      }
      queueRef.current.push({ type: "resync_required", payload: { reason: "socket closed" } });
      if (frameRef.current === null) {
        frameRef.current = window.requestAnimationFrame(flush);
      }
    };

    return () => {
      closedByEffect = true;
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
      }
      socket.close();
    };
  }, [applyEvents, enabled, queryClient]);
}
