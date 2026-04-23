import { useEffect, useState, type PropsWithChildren } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { putSettings, startRuntime, stopRuntime } from "../api/client";
import { notifyCommandError, notifyCommandSuccess } from "../lib/commandFeedback";
import { useAppStore } from "../store/appStore";
import type { Settings } from "../types";
import { Panel } from "./Panel";

export function ControlPanel() {
  const queryClient = useQueryClient();
  const settings = useAppStore((state) => state.settings);
  const currentPosition = useAppStore((state) => state.currentPosition);
  const runtimeActivity = useAppStore((state) => state.runtimeStatus?.activity ?? null);
  const setSnapshot = useAppStore((state) => state.setSnapshot);
  const addToast = useAppStore((state) => state.addToast);
  const [draft, setDraft] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!settings) {
      return;
    }
    setDraft({
      active_symbol: settings.active_symbol,
      timeframe: settings.timeframe,
      aso_length: String(settings.aso_length),
      aso_mode: settings.aso_mode,
      leverage: String(settings.leverage),
      fee_rate: String(settings.fee_rate),
      fixed_notional: String(settings.fixed_notional)
    });
  }, [settings]);

  const startMutation = useMutation({
    mutationFn: startRuntime,
    onSuccess: () => {
      notifyCommandSuccess(addToast, "runtime_start");
      void queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
    onError: (error) => {
      notifyCommandError(addToast, "runtime_start", error);
    }
  });

  const stopMutation = useMutation({
    mutationFn: stopRuntime,
    onSuccess: () => {
      notifyCommandSuccess(addToast, "runtime_stop");
      void queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
    onError: (error) => {
      notifyCommandError(addToast, "runtime_stop", error);
    }
  });

  const applyMutation = useMutation({
    mutationFn: putSettings,
    onSuccess: (snapshot) => {
      setSnapshot(snapshot);
      notifyCommandSuccess(addToast, "settings_apply");
    },
    onError: (error) => {
      notifyCommandError(addToast, "settings_apply", error);
    }
  });

  if (!settings) {
    return null;
  }

  const nextSettings: Settings = {
    ...settings,
    active_symbol: draft.active_symbol as Settings["active_symbol"],
    timeframe: draft.timeframe as Settings["timeframe"],
    aso_length: Number.parseInt(draft.aso_length, 10) || settings.aso_length,
    aso_mode: draft.aso_mode as Settings["aso_mode"],
    leverage: Number.parseFloat(draft.leverage) || settings.leverage,
    fee_rate: Number.parseFloat(draft.fee_rate) || settings.fee_rate,
    fixed_notional: Number.parseFloat(draft.fixed_notional) || settings.fixed_notional
  };
  const commandBusy =
    runtimeActivity !== null || applyMutation.isPending || startMutation.isPending || stopMutation.isPending;

  return (
    <div className="grid-span-4">
      <Panel title="Control Panel">
        <div className="control-panel">
          <div className="field-grid">
            <Field label="Active Symbol">
              <select
                aria-label="Active Symbol"
                value={draft.active_symbol ?? settings.active_symbol}
                disabled={Boolean(currentPosition) || commandBusy}
                onChange={(event) => setDraft((current) => ({ ...current, active_symbol: event.target.value }))}
              >
                {settings.available_symbols.map((symbol) => (
                  <option key={symbol} value={symbol}>
                    {symbol}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Timeframe">
              <select
                aria-label="Timeframe"
                value={draft.timeframe ?? settings.timeframe}
                disabled={commandBusy}
                onChange={(event) => setDraft((current) => ({ ...current, timeframe: event.target.value }))}
              >
                {["1m", "5m", "15m", "1h"].map((timeframe) => (
                  <option key={timeframe} value={timeframe}>
                    {timeframe}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="ASO Length">
              <input
                aria-label="ASO Length"
                value={draft.aso_length ?? String(settings.aso_length)}
                disabled={commandBusy}
                onBlur={(event) => setDraft((current) => ({ ...current, aso_length: event.target.value.trim() }))}
                onChange={(event) => setDraft((current) => ({ ...current, aso_length: event.target.value }))}
              />
            </Field>
            <Field label="ASO Mode">
              <select
                aria-label="ASO Mode"
                value={draft.aso_mode ?? settings.aso_mode}
                disabled={commandBusy}
                onChange={(event) => setDraft((current) => ({ ...current, aso_mode: event.target.value }))}
              >
                {["intrabar", "group", "both"].map((mode) => (
                  <option key={mode} value={mode}>
                    {mode}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Leverage">
              <input
                aria-label="Leverage"
                value={draft.leverage ?? String(settings.leverage)}
                disabled={commandBusy}
                onBlur={(event) => setDraft((current) => ({ ...current, leverage: event.target.value.trim() }))}
                onChange={(event) => setDraft((current) => ({ ...current, leverage: event.target.value }))}
              />
            </Field>
            <Field label="Fee Rate">
              <input
                aria-label="Fee Rate"
                value={draft.fee_rate ?? String(settings.fee_rate)}
                disabled={commandBusy}
                onBlur={(event) => setDraft((current) => ({ ...current, fee_rate: event.target.value.trim() }))}
                onChange={(event) => setDraft((current) => ({ ...current, fee_rate: event.target.value }))}
              />
            </Field>
            <Field label="Fixed Notional">
              <input
                aria-label="Fixed Notional"
                value={draft.fixed_notional ?? String(settings.fixed_notional)}
                disabled={commandBusy}
                onBlur={(event) => setDraft((current) => ({ ...current, fixed_notional: event.target.value.trim() }))}
                onChange={(event) => setDraft((current) => ({ ...current, fixed_notional: event.target.value }))}
              />
            </Field>
            <Field label="Mode">
              <button disabled>{settings.live_mode_visible ? "LIVE LOCKED" : "PAPER ONLY"}</button>
            </Field>
          </div>
          <div className="action-row">
            <button onClick={() => applyMutation.mutate(nextSettings)} disabled={commandBusy}>
              Apply
            </button>
            <button onClick={() => startMutation.mutate()} disabled={commandBusy}>
              Start Runtime
            </button>
            <button onClick={() => stopMutation.mutate()} disabled={commandBusy}>
              Stop Runtime
            </button>
          </div>
          {runtimeActivity ? (
            <div className="muted">History sync/rebuild is in progress. Commands are temporarily limited.</div>
          ) : null}
        </div>
      </Panel>
    </div>
  );
}

function Field({ label, children }: PropsWithChildren<{ label: string }>) {
  return (
    <div className="field">
      <label>{label}</label>
      {children}
    </div>
  );
}
