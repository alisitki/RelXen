import { useEffect, useState } from "react";

import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatTime } from "../lib/format";
import { connectionLabel, connectionWarning, runtimeActivityLabel } from "../lib/connection";

export function ConnectionPanel() {
  const connection = useAppStore((state) => state.connectionState);
  const runtimeActivity = useAppStore((state) => state.runtimeStatus?.activity ?? null);
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    const shouldTrackAge =
      connection?.resync_required || connection?.status === "reconnecting" || connection?.status === "stale";
    if (!shouldTrackAge) {
      return;
    }

    const timer = window.setInterval(() => setNowMs(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [connection?.resync_required, connection?.status, connection?.status_since]);

  const stateLabel = connectionLabel(connection, nowMs);
  const warning = connectionWarning(connection, nowMs);

  return (
    <div className="grid-span-3">
      <Panel title="Connection Panel">
        <div className="connection-panel metric-grid">
          <Metric label="State" value={stateLabel} />
          <Metric label="Runtime" value={runtimeActivityLabel(runtimeActivity) ?? "STEADY"} />
          <Metric label="Last Tick" value={formatTime(connection?.last_message_time ?? null)} />
          <Metric label="Reconnects" value={String(connection?.reconnect_attempts ?? 0)} />
          <Metric label="Recovery" value={connection?.resync_required ? "BOOTSTRAP RELOAD" : "INCREMENTAL"} />
        </div>
        <div className="muted">{connection?.detail ?? "No connection detail available."}</div>
        {warning ? <div className="muted">{warning}</div> : null}
      </Panel>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <div className="metric__label">{label}</div>
      <div className="metric__value">{value}</div>
    </div>
  );
}
