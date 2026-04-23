import type { ConnectionState, RuntimeActivity } from "../types";

export const STALE_WARNING_AFTER_MS = 15_000;

export function connectionLabel(connection: ConnectionState | null, nowMs?: number): string {
  if (!connection) {
    return "DISCONNECTED";
  }

  const baseLabel = connection.resync_required ? "STALE" : statusLabel(connection.status);
  const ageLabel = formatConnectionAge(connection, nowMs);

  if (ageLabel) {
    return `${baseLabel} ${ageLabel}`;
  }

  return baseLabel;
}

export function connectionWarning(connection: ConnectionState | null, nowMs?: number): string | null {
  if (!connection || !nowMs || !connection.status_since) {
    return null;
  }

  const agingState =
    connection.resync_required || connection.status === "stale" || connection.status === "reconnecting";
  if (!agingState) {
    return null;
  }

  if (nowMs - connection.status_since < STALE_WARNING_AFTER_MS) {
    return null;
  }

  if (connection.resync_required || connection.status === "stale") {
    return "Feed has been stale too long. Bootstrap reload may be required.";
  }

  return "Reconnect is taking longer than expected. Live deltas may be delayed.";
}

export function runtimeActivityLabel(activity: RuntimeActivity | null | undefined): string | null {
  switch (activity) {
    case "history_sync":
      return "HISTORY SYNC";
    case "rebuilding":
      return "REBUILDING";
    default:
      return null;
  }
}

function statusLabel(status: ConnectionState["status"]): string {
  switch (status) {
    case "reconnecting":
      return "RECONNECTING";
    case "stale":
      return "STALE";
    case "resynced":
      return "RESYNCED";
    case "connected":
      return "CONNECTED";
    case "disconnected":
    default:
      return "DISCONNECTED";
  }
}

function formatConnectionAge(connection: ConnectionState, nowMs?: number): string | null {
  if (!nowMs) {
    return null;
  }

  const agingState =
    connection.resync_required || connection.status === "stale" || connection.status === "reconnecting";
  if (!agingState || !connection.status_since) {
    return null;
  }

  const elapsedSeconds = Math.max(0, Math.floor((nowMs - connection.status_since) / 1000));
  if (elapsedSeconds < 60) {
    return `${elapsedSeconds}s`;
  }

  const minutes = Math.floor(elapsedSeconds / 60);
  const seconds = elapsedSeconds % 60;
  if (minutes < 60) {
    return `${minutes}m ${seconds}s`;
  }

  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}
