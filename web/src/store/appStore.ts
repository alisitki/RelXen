import { create } from "zustand";

import type {
  AsoPoint,
  BootstrapPayload,
  Candle,
  LogEvent,
  LiveFillRecord,
  LiveOrderRecord,
  LiveStatusSnapshot,
  OutboundEvent,
  Position,
  RuntimeStatus,
  SignalEvent,
  SystemMetrics,
  ToastKind,
  ToastMessage,
  Trade,
  Wallet
} from "../types";

type CandleUpdate = { id: number; candle: Candle } | null;
type AsoUpdate = { id: number; point: AsoPoint } | null;

interface AppStoreState {
  bootstrapped: boolean;
  metadata: BootstrapPayload["metadata"] | null;
  runtimeStatus: RuntimeStatus | null;
  settings: BootstrapPayload["settings"] | null;
  allowedSymbols: BootstrapPayload["allowed_symbols"];
  activeSymbol: BootstrapPayload["active_symbol"] | null;
  candles: Candle[];
  asoPoints: AsoPoint[];
  recentSignals: SignalEvent[];
  recentTrades: Trade[];
  currentPosition: Position | null;
  wallets: Wallet[];
  performance: BootstrapPayload["performance"] | null;
  connectionState: BootstrapPayload["connection_state"] | null;
  systemMetrics: SystemMetrics | null;
  recentLogs: LogEvent[];
  liveStatus: LiveStatusSnapshot | null;
  toasts: ToastMessage[];
  chartVersion: number;
  lastCandleUpdate: CandleUpdate;
  lastAsoUpdate: AsoUpdate;
  resyncRequestedAt: number | null;
  setSnapshot: (snapshot: BootstrapPayload) => void;
  setLiveStatus: (status: LiveStatusSnapshot) => void;
  applyEvents: (events: OutboundEvent[]) => void;
  addToast: (message: string, kind?: ToastKind) => void;
  dismissToast: (id: number) => void;
}

let eventId = 0;
let toastId = 0;

export const useAppStore = create<AppStoreState>((set) => ({
  bootstrapped: false,
  metadata: null,
  runtimeStatus: null,
  settings: null,
  allowedSymbols: [],
  activeSymbol: null,
  candles: [],
  asoPoints: [],
  recentSignals: [],
  recentTrades: [],
  currentPosition: null,
  wallets: [],
  performance: null,
  connectionState: null,
  systemMetrics: null,
  recentLogs: [],
  liveStatus: null,
  toasts: [],
  chartVersion: 0,
  lastCandleUpdate: null,
  lastAsoUpdate: null,
  resyncRequestedAt: null,
  setSnapshot: (snapshot) =>
    set((state) => ({
      bootstrapped: true,
      metadata: snapshot.metadata,
      runtimeStatus: snapshot.runtime_status,
      settings: snapshot.settings,
      allowedSymbols: snapshot.allowed_symbols,
      activeSymbol: snapshot.active_symbol,
      candles: snapshot.candles,
      asoPoints: snapshot.aso_points,
      recentSignals: snapshot.recent_signals,
      recentTrades: snapshot.recent_trades,
      currentPosition: snapshot.current_position,
      wallets: snapshot.wallets,
      performance: snapshot.performance,
      connectionState: snapshot.connection_state,
      systemMetrics: snapshot.system_metrics,
      recentLogs: snapshot.recent_logs,
      liveStatus: snapshot.live_status,
      toasts:
        state.resyncRequestedAt !== null
          ? enqueueToast(state.toasts, {
              id: ++toastId,
              kind: "info",
              message: "Snapshot reloaded after resync.",
              created_at: Date.now()
            })
          : state.toasts,
      chartVersion: state.chartVersion + 1,
      lastCandleUpdate: null,
      lastAsoUpdate: null,
      resyncRequestedAt: null
    })),
  setLiveStatus: (status) =>
    set(() => ({
      liveStatus: status
    })),
  applyEvents: (events) =>
    set((state) => {
      let next = { ...state };
      for (const event of events) {
        switch (event.type) {
          case "snapshot":
            next = {
              ...next,
              bootstrapped: true,
              metadata: event.payload.metadata,
              runtimeStatus: event.payload.runtime_status,
              settings: event.payload.settings,
              allowedSymbols: event.payload.allowed_symbols,
              activeSymbol: event.payload.active_symbol,
              candles: event.payload.candles,
              asoPoints: event.payload.aso_points,
              recentSignals: event.payload.recent_signals,
              recentTrades: event.payload.recent_trades,
              currentPosition: event.payload.current_position,
              wallets: event.payload.wallets,
              performance: event.payload.performance,
              connectionState: event.payload.connection_state,
              systemMetrics: event.payload.system_metrics,
              recentLogs: event.payload.recent_logs,
              liveStatus: event.payload.live_status,
              toasts:
                next.resyncRequestedAt !== null
                  ? enqueueToast(next.toasts, {
                      id: ++toastId,
                      kind: "info",
                      message: "Snapshot reloaded after resync.",
                      created_at: Date.now()
                    })
                  : next.toasts,
              chartVersion: next.chartVersion + 1,
              lastCandleUpdate: null,
              lastAsoUpdate: null,
              resyncRequestedAt: null
            };
            break;
          case "resync_required":
            next.resyncRequestedAt = Date.now();
            next.connectionState = next.connectionState
              ? {
                  ...next.connectionState,
                  status: "stale",
                  status_since: Date.now(),
                  resync_required: true,
                  detail: event.payload.reason
                }
              : next.connectionState;
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "error",
              message: "Data stream went stale. Reloading snapshot.",
              created_at: Date.now()
            });
            break;
          case "candle_partial":
          case "candle_closed":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.candles = upsertByOpenTime(next.candles, event.payload);
            next.lastCandleUpdate = { id: ++eventId, candle: event.payload };
            break;
          case "aso_updated":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.asoPoints = upsertAsoPoint(next.asoPoints, event.payload);
            next.lastAsoUpdate = { id: ++eventId, point: event.payload };
            break;
          case "signal_emitted":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.recentSignals = keepTail([...next.recentSignals, event.payload], 200);
            break;
          case "trade_appended":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.recentTrades = appendTrade(next.recentTrades, event.payload, 100);
            break;
          case "trade_history_reset":
            next.recentTrades = [];
            break;
          case "position_updated":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.currentPosition = event.payload;
            break;
          case "wallet_updated":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.wallets = event.payload;
            break;
          case "performance_updated":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.performance = event.payload;
            break;
          case "connection_changed":
            next.connectionState = event.payload;
            break;
          case "runtime_changed":
            if (next.resyncRequestedAt !== null) {
              break;
            }
            next.runtimeStatus = event.payload;
            break;
          case "live_status_updated":
            next.liveStatus = event.payload;
            break;
          case "live_shadow_status_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                state: event.payload.state,
                reconciliation: event.payload,
                updated_at: event.payload.updated_at
              };
            }
            break;
          case "live_shadow_account_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                reconciliation: {
                  ...next.liveStatus.reconciliation,
                  shadow: event.payload,
                  updated_at: event.payload.updated_at
                },
                updated_at: event.payload.updated_at
              };
            }
            break;
          case "live_intent_preview_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                intent_preview: event.payload,
                updated_at: event.payload.built_at
              };
            }
            break;
          case "live_preflight_result_appended":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                recent_preflights: keepTail(
                  [...next.liveStatus.recent_preflights.filter((item) => item.id !== event.payload.id), event.payload],
                  50
                ),
                updated_at: event.payload.created_at
              };
            }
            break;
          case "live_execution_state_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                execution: event.payload,
                updated_at: event.payload.updated_at
              };
            }
            break;
          case "live_execution_blocked":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "error",
              message: `Live execution blocked: ${event.payload.reason}`,
              created_at: Date.now()
            });
            break;
          case "live_kill_switch_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                kill_switch: event.payload,
                execution: {
                  ...next.liveStatus.execution,
                  kill_switch_engaged: event.payload.engaged,
                  updated_at: event.payload.updated_at
                },
                updated_at: event.payload.updated_at
              };
            }
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: event.payload.engaged ? "error" : "info",
              message: event.payload.engaged ? "Kill switch engaged." : "Kill switch released.",
              created_at: Date.now()
            });
            break;
          case "live_auto_state_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                auto_executor: event.payload,
                updated_at: event.payload.updated_at
              };
            }
            break;
          case "live_execution_degraded":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "error",
              message: `Live execution degraded: ${event.payload.reason}`,
              created_at: Date.now()
            });
            break;
          case "live_execution_resynced":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "info",
              message: "Live execution state repaired from recent exchange window.",
              created_at: Date.now()
            });
            break;
          case "live_mainnet_gate_updated":
            if (next.liveStatus) {
              next.liveStatus = {
                ...next.liveStatus,
                execution: {
                  ...next.liveStatus.execution,
                  mainnet_canary_enabled: event.payload.enabled
                },
                mainnet_canary: {
                  ...next.liveStatus.mainnet_canary,
                  enabled_by_server: event.payload.enabled
                }
              };
            }
            break;
          case "live_order_submitted":
          case "live_order_updated":
            if (next.liveStatus) {
              const execution = next.liveStatus.execution;
              const recentOrders = upsertLiveOrder(execution.recent_orders, event.payload, 50);
              next.liveStatus = {
                ...next.liveStatus,
                execution: {
                  ...execution,
                  active_order: isTerminalLiveOrder(event.payload.status) ? execution.active_order : event.payload,
                  recent_orders: recentOrders,
                  updated_at: event.payload.updated_at
                },
                updated_at: event.payload.updated_at
              };
            }
            break;
          case "live_fill_appended":
            if (next.liveStatus) {
              const execution = next.liveStatus.execution;
              next.liveStatus = {
                ...next.liveStatus,
                execution: {
                  ...execution,
                  recent_fills: upsertLiveFill(execution.recent_fills, event.payload, 100),
                  updated_at: event.payload.created_at
                },
                updated_at: event.payload.created_at
              };
            }
            break;
          case "live_flatten_started":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "info",
              message: `TESTNET flatten started for ${event.payload.symbol}.`,
              created_at: Date.now()
            });
            break;
          case "live_flatten_finished":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "info",
              message: event.payload.message,
              created_at: Date.now()
            });
            break;
          case "live_shadow_degraded":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "error",
              message: `Live shadow degraded: ${event.payload.reason}`,
              created_at: Date.now()
            });
            break;
          case "live_shadow_resynced":
            next.toasts = enqueueToast(next.toasts, {
              id: ++toastId,
              kind: "info",
              message: "Live shadow state resynced.",
              created_at: Date.now()
            });
            break;
          case "log_appended":
            next.recentLogs = keepTail([...next.recentLogs, event.payload], 400);
            break;
          case "system_metrics":
            next.systemMetrics = event.payload;
            break;
        }
      }
      return next;
    }),
  addToast: (message, kind = "info") =>
    set((state) => ({
      toasts: enqueueToast(state.toasts, {
        id: ++toastId,
        kind,
        message,
        created_at: Date.now()
      })
    })),
  dismissToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id)
    }))
}));

function upsertByOpenTime(candles: Candle[], candle: Candle): Candle[] {
  const next = candles.slice();
  const index = next.findIndex((item) => item.open_time === candle.open_time);
  if (index >= 0) {
    next[index] = candle;
  } else {
    next.push(candle);
    next.sort((left, right) => left.open_time - right.open_time);
  }
  return keepTail(next, 500);
}

function upsertAsoPoint(points: AsoPoint[], point: AsoPoint): AsoPoint[] {
  const next = points.slice();
  const index = next.findIndex((item) => item.open_time === point.open_time);
  if (index >= 0) {
    next[index] = point;
  } else {
    next.push(point);
    next.sort((left, right) => left.open_time - right.open_time);
  }
  return keepTail(next, 500);
}

function appendTrade(trades: Trade[], trade: Trade, limit: number): Trade[] {
  const next = trades.filter((item) => item.id !== trade.id);
  next.push(trade);
  next.sort((left, right) => left.timestamp - right.timestamp);
  return keepTail(next, limit);
}

function upsertLiveOrder(orders: LiveOrderRecord[], order: LiveOrderRecord, limit: number): LiveOrderRecord[] {
  const next = orders.filter((item) => item.id !== order.id);
  next.push(order);
  next.sort((left, right) => left.updated_at - right.updated_at);
  return keepTail(next, limit);
}

function upsertLiveFill(fills: LiveFillRecord[], fill: LiveFillRecord, limit: number): LiveFillRecord[] {
  const next = fills.filter((item) => item.id !== fill.id);
  next.push(fill);
  next.sort((left, right) => left.created_at - right.created_at);
  return keepTail(next, limit);
}

function isTerminalLiveOrder(status: LiveOrderRecord["status"]): boolean {
  return (
    status === "filled" ||
    status === "canceled" ||
    status === "rejected" ||
    status === "expired" ||
    status === "expired_in_match"
  );
}

function keepTail<T>(items: T[], limit: number): T[] {
  if (items.length <= limit) {
    return items;
  }
  return items.slice(items.length - limit);
}

function enqueueToast(toasts: ToastMessage[], toast: ToastMessage): ToastMessage[] {
  return keepTail([...toasts, toast], 4);
}
