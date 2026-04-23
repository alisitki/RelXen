import { useAppStore } from "../store/appStore";
import { formatTime } from "../lib/format";
import { connectionLabel, runtimeActivityLabel } from "../lib/connection";

export function Header() {
  const metadata = useAppStore((state) => state.metadata);
  const runtime = useAppStore((state) => state.runtimeStatus);
  const connection = useAppStore((state) => state.connectionState);
  const position = useAppStore((state) => state.currentPosition);
  const activityLabel = runtimeActivityLabel(runtime?.activity);

  return (
    <header className="header">
      <div>
        <h1 className="header__title">RelXen Futures Paper Desk</h1>
        <div className="header__meta">
          {metadata?.app_name} v{metadata?.version} · started {formatTime(metadata?.started_at ?? null)}
        </div>
      </div>
      <div className="header__pillset">
        <div className="status-pill">
          Runtime <strong>{runtime?.running ? "RUNNING" : "STOPPED"}</strong>
        </div>
        {activityLabel ? (
          <div className="status-pill">
            Sync <strong>{activityLabel}</strong>
          </div>
        ) : null}
        <div className="status-pill">
          Feed <strong>{connectionLabel(connection)}</strong>
        </div>
        <div className="status-pill">
          Position{" "}
          <strong>
            {position ? (position.side === "long" ? "▲ LONG" : "▼ SHORT") : "■ FLAT"}
          </strong>
        </div>
      </div>
    </header>
  );
}
