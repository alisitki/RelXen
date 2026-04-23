import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatCompactBytes, formatNumber, formatTime } from "../lib/format";

export function SystemPanel() {
  const metrics = useAppStore((state) => state.systemMetrics);
  return (
    <div className="grid-span-3">
      <Panel title="System Panel">
        <div className="system-panel metric-grid">
          <Metric label="CPU" value={`${formatNumber(metrics?.cpu_usage_percent ?? 0, 1)}%`} />
          <Metric label="Memory" value={formatCompactBytes(metrics?.memory_used_bytes ?? 0)} />
          <Metric label="Capacity" value={formatCompactBytes(metrics?.memory_total_bytes ?? 0)} />
          <Metric label="Tasks" value={String(metrics?.task_count ?? 0)} />
        </div>
        <div className="muted">Collected {formatTime(metrics?.collected_at ?? null)}</div>
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
