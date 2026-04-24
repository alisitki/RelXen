import { useAppStore } from "../store/appStore";
import type { LiveStatusSnapshot } from "../types";

export function SafetyStatusBar() {
  const settings = useAppStore((state) => state.settings);
  const activeSymbol = useAppStore((state) => state.activeSymbol);
  const liveStatus = useAppStore((state) => state.liveStatus);
  const currentPosition = useAppStore((state) => state.currentPosition);

  if (!settings && !liveStatus) {
    return null;
  }

  const liveMode = liveStatus ? liveModeLabel(liveStatus) : "PAPER MODE";
  const mode = liveStatus?.mode_preference === "live_read_only" ? "LIVE READ-ONLY" : "PAPER MODE";
  const canary = liveStatus ? canaryLabel(liveStatus) : "MAINNET CANARY: DISABLED";
  const killSwitch = liveStatus?.kill_switch.engaged ? "KILL SWITCH: ENGAGED" : "KILL SWITCH: RELEASED";
  const position = currentPosition ? `PAPER POSITION: ${currentPosition.side.toUpperCase()}` : "PAPER POSITION: FLAT";

  return (
    <section className="safety-status" aria-label="Safety status summary">
      <StatusTile label="Mode" value={mode} tone="neutral" />
      <StatusTile label="Live Scope" value={liveMode} tone={liveMode.includes("MAINNET") ? "warning" : "safe"} />
      <StatusTile label="Mainnet Auto" value="MAINNET AUTO: BLOCKED" tone="safe" />
      <StatusTile label="Mainnet Canary" value={canary} tone={canary.includes("ENABLED") ? "warning" : "safe"} />
      <StatusTile label="Kill Switch" value={killSwitch} tone={liveStatus?.kill_switch.engaged ? "danger" : "safe"} />
      <StatusTile label="Active Symbol" value={activeSymbol ?? settings?.active_symbol ?? "UNKNOWN"} tone="neutral" />
      <StatusTile label="Current State" value={stateLabel(liveStatus?.state)} tone="neutral" />
      <StatusTile label="Position" value={position} tone={currentPosition ? "warning" : "safe"} />
    </section>
  );
}

function StatusTile({ label, value, tone }: { label: string; value: string; tone: "safe" | "warning" | "danger" | "neutral" }) {
  return (
    <div className={`safety-status__tile safety-status__tile--${tone}`}>
      <div className="safety-status__label">{label}</div>
      <div className="safety-status__value">{value}</div>
    </div>
  );
}

function liveModeLabel(status: LiveStatusSnapshot): string {
  if (status.environment === "mainnet") {
    return status.mainnet_canary.enabled_by_server ? "MAINNET MANUAL CANARY ONLY" : "MAINNET EXECUTION BLOCKED";
  }
  if (status.environment === "testnet") {
    return status.auto_executor.state === "running" ? "TESTNET AUTO RUNNING" : "TESTNET MANUAL GATED";
  }
  return "PAPER MODE";
}

function canaryLabel(status: LiveStatusSnapshot): string {
  if (!status.mainnet_canary.enabled_by_server) {
    return "MAINNET CANARY: DISABLED";
  }
  if (status.mainnet_canary.manual_execution_enabled) {
    return "MAINNET CANARY: CONFIRMATION READY";
  }
  if (status.mainnet_canary.canary_ready) {
    return "MAINNET CANARY: READY";
  }
  return "MAINNET CANARY: ENABLED BUT BLOCKED";
}

function stateLabel(state: string | undefined): string {
  if (!state) {
    return "PAPER READY";
  }
  return state.replaceAll("_", " ").toUpperCase();
}
