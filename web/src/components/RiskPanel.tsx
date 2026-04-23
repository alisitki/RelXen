import { useMutation } from "@tanstack/react-query";

import { closeAllPaper, resetPaper } from "../api/client";
import { notifyCommandError, notifyCommandSuccess } from "../lib/commandFeedback";
import { useAppStore } from "../store/appStore";
import { formatNumber } from "../lib/format";
import { Panel } from "./Panel";

export function RiskPanel() {
  const settings = useAppStore((state) => state.settings);
  const performance = useAppStore((state) => state.performance);
  const runtimeActivity = useAppStore((state) => state.runtimeStatus?.activity ?? null);
  const setSnapshot = useAppStore((state) => state.setSnapshot);
  const addToast = useAppStore((state) => state.addToast);

  const closeMutation = useMutation({
    mutationFn: closeAllPaper,
    onSuccess: (snapshot) => {
      setSnapshot(snapshot);
      notifyCommandSuccess(addToast, "paper_close_all");
    },
    onError: (error) => {
      notifyCommandError(addToast, "paper_close_all", error);
    }
  });

  const resetMutation = useMutation({
    mutationFn: resetPaper,
    onSuccess: (snapshot) => {
      setSnapshot(snapshot);
      notifyCommandSuccess(addToast, "paper_reset");
    },
    onError: (error) => {
      notifyCommandError(addToast, "paper_reset", error);
    }
  });

  if (!settings || !performance) {
    return null;
  }
  const commandBusy = runtimeActivity !== null || closeMutation.isPending || resetMutation.isPending;

  return (
    <div className="grid-span-3">
      <Panel title="Risk Panel">
        <div className="risk-panel metric-grid">
          <Metric label="Paper" value={settings.paper_enabled ? "ENABLED" : "DISABLED"} />
          <Metric label="Sizing" value={`${settings.sizing_mode} ${formatNumber(settings.fixed_notional)}`} />
          <Metric label="Leverage" value={`${formatNumber(settings.leverage)}x`} />
          <Metric label="Fee Rate" value={String(settings.fee_rate)} />
          <Metric label="Fees Paid" value={formatNumber(performance.fees_paid)} />
          <Metric label="Live Mode" value={settings.live_mode_visible ? "VISIBLE LOCKED" : "HIDDEN"} />
        </div>
        <div className="action-row">
          <button onClick={() => closeMutation.mutate()} disabled={commandBusy}>
            Close All
          </button>
          <button onClick={() => resetMutation.mutate()} disabled={commandBusy}>
            Reset Paper
          </button>
        </div>
        {runtimeActivity ? (
          <div className="muted">Paper commands are paused while history sync/rebuild is active.</div>
        ) : null}
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
