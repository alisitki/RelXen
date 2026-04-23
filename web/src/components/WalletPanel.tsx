import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatNumber } from "../lib/format";

export function WalletPanel() {
  const wallets = useAppStore((state) => state.wallets);

  return (
    <div className="grid-span-4">
      <Panel title="Wallet Panel">
        <div className="wallet-panel list">
          {wallets.map((wallet) => (
            <div key={wallet.quote_asset} className="list-item">
              <div className="list-item__meta">
                <strong>{wallet.quote_asset}</strong>
                <span>available {formatNumber(wallet.available_balance)}</span>
              </div>
              <div className="metric-grid">
                <Metric label="Balance" value={formatNumber(wallet.balance)} />
                <Metric label="Reserved" value={formatNumber(wallet.reserved_margin)} />
                <Metric label="Unrealized" value={formatNumber(wallet.unrealized_pnl)} />
                <Metric label="Fees" value={formatNumber(wallet.fees_paid)} />
              </div>
            </div>
          ))}
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
