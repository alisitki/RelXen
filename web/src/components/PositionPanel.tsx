import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatNumber, formatTime } from "../lib/format";

export function PositionPanel() {
  const position = useAppStore((state) => state.currentPosition);

  return (
    <div className="grid-span-4">
      <Panel title="Position Panel">
        <div className="position-panel">
          <div className={`label-strong ${position ? (position.side === "long" ? "label-long" : "label-short") : "label-flat"}`}>
            {position ? (position.side === "long" ? "▲ LONG" : "▼ SHORT") : "■ FLAT"}
          </div>
          {position ? (
            <div className="metric-grid">
              <Metric label="Entry" value={formatNumber(position.entry_price)} />
              <Metric label="Mark" value={formatNumber(position.mark_price)} />
              <Metric label="Qty" value={formatNumber(position.qty, 6)} />
              <Metric label="Notional" value={formatNumber(position.notional)} />
              <Metric label="Margin" value={formatNumber(position.margin_used)} />
              <Metric label="Opened" value={formatTime(position.opened_at)} />
            </div>
          ) : (
            <div className="muted">No paper position is open.</div>
          )}
        </div>
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
