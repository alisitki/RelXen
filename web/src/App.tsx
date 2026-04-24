import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";

import { getBootstrap } from "./api/client";
import { ChartSection } from "./components/ChartSection";
import { ConnectionPanel } from "./components/ConnectionPanel";
import { ControlPanel } from "./components/ControlPanel";
import { Header } from "./components/Header";
import { LiveAccessPanel } from "./components/LiveAccessPanel";
import { LogPanel } from "./components/LogPanel";
import { PerformancePanel } from "./components/PerformancePanel";
import { PositionPanel } from "./components/PositionPanel";
import { RiskPanel } from "./components/RiskPanel";
import { SafetyStatusBar } from "./components/SafetyStatusBar";
import { SystemPanel } from "./components/SystemPanel";
import { ToastViewport } from "./components/ToastViewport";
import { TradeHistoryPanel } from "./components/TradeHistoryPanel";
import { WalletPanel } from "./components/WalletPanel";
import { useEventStream } from "./hooks/useEventStream";
import { toErrorMessage } from "./lib/errors";
import { useAppStore } from "./store/appStore";

export default function App() {
  const setSnapshot = useAppStore((state) => state.setSnapshot);

  const bootstrapQuery = useQuery({
    queryKey: ["bootstrap"],
    queryFn: getBootstrap
  });

  useEffect(() => {
    if (bootstrapQuery.data) {
      setSnapshot(bootstrapQuery.data);
    }
  }, [bootstrapQuery.data, setSnapshot]);

  useEventStream(Boolean(bootstrapQuery.data));

  if (bootstrapQuery.isLoading) {
    return <main className="loading-shell">Loading bootstrap snapshot...</main>;
  }

  if (bootstrapQuery.isError) {
    return (
      <main className="loading-shell">
        Bootstrap failed: {toErrorMessage(bootstrapQuery.error, "Bootstrap failed.")}
      </main>
    );
  }

  return (
    <main className="app-shell">
      <Header />
      <SafetyStatusBar />
      <ToastViewport />
      <div className="dashboard-grid">
        <ControlPanel />
        <ChartSection />
        <PositionPanel />
        <PerformancePanel />
        <WalletPanel />
        <ConnectionPanel />
        <LiveAccessPanel />
        <SystemPanel />
        <RiskPanel />
        <TradeHistoryPanel />
        <LogPanel />
      </div>
    </main>
  );
}
