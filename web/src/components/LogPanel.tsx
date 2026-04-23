import { Panel } from "./Panel";
import { useAppStore } from "../store/appStore";
import { formatTime } from "../lib/format";

export function LogPanel() {
  const logs = useAppStore((state) => state.recentLogs);
  return (
    <div className="grid-span-6">
      <Panel title="Log Panel">
        <div className="log-panel list">
          {logs.map((log) => (
            <div key={log.id} className="list-item">
              <div className="list-item__meta">
                <strong>{log.level.toUpperCase()}</strong>
                <span>{formatTime(log.timestamp)}</span>
              </div>
              <div className="list-item__meta">
                <span>{log.target}</span>
              </div>
              <div>{log.message}</div>
            </div>
          ))}
          {logs.length === 0 && <div className="muted">No recent logs.</div>}
        </div>
      </Panel>
    </div>
  );
}
