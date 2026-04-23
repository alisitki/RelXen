import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatNumber, formatTime } from "../lib/format";

export function TradeHistoryPanel() {
  const recentTrades = useAppStore((state) => state.recentTrades);

  return (
    <div className="grid-span-6">
      <Panel title="Trade History Panel">
        <div className="trade-panel list">
          {recentTrades.map((trade) => (
            <div key={trade.id} className="list-item">
              <div className="list-item__meta">
                <strong>
                  {trade.action.toUpperCase()} · {trade.side === "long" ? "▲ LONG" : "▼ SHORT"}
                </strong>
                <span>{trade.source === "manual" ? "MANUAL" : "SIGNAL"}</span>
              </div>
              <div className="list-item__meta">
                <span>{trade.symbol}</span>
                <span>
                  open {formatTime(trade.opened_at ?? trade.timestamp)} · close {formatTime(trade.closed_at)}
                </span>
              </div>
              <div className="list-item__meta">
                <span>qty {formatNumber(trade.qty, 6)}</span>
                <span>
                  entry {formatNumber(trade.entry_price ?? trade.price)} · exit{" "}
                  {formatNumber(trade.exit_price ?? trade.price)}
                </span>
              </div>
              <div className="list-item__meta">
                <span>fee {formatNumber(trade.fee_paid)}</span>
                <span>pnl {formatNumber(trade.realized_pnl)}</span>
              </div>
            </div>
          ))}
          {recentTrades.length === 0 && <div className="muted">No paper trades recorded yet.</div>}
        </div>
      </Panel>
    </div>
  );
}
