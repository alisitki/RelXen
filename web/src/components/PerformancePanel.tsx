import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatNumber, formatPercent } from "../lib/format";

export function PerformancePanel() {
  const performance = useAppStore((state) => state.performance);
  if (!performance) {
    return null;
  }

  return (
    <div className="grid-span-4">
      <Panel title="Performance Panel">
        <div className="performance-panel metric-grid">
          <Metric label="Equity" value={formatNumber(performance.equity)} />
          <Metric label="Realized" value={formatNumber(performance.realized_pnl)} />
          <Metric label="Unrealized" value={formatNumber(performance.unrealized_pnl)} />
          <Metric label="Return" value={formatPercent(performance.return_pct)} />
          <Metric label="Trades" value={String(performance.trades)} />
          <Metric label="Win Rate" value={formatPercent(performance.win_rate)} />
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
