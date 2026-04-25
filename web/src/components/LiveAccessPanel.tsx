import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  armLive,
  cancelAllLiveOrders,
  cancelAllLiveOrdersWithPayload,
  cancelLiveOrder,
  cancelLiveOrderWithPayload,
  configureLiveRiskProfile,
  createLiveCredential,
  deleteLiveCredential,
  disarmLive,
  engageLiveKillSwitch,
  executeLivePreview,
  flattenLivePosition,
  flattenLivePositionWithPayload,
  exportMainnetAutoEvidence,
  getLiveIntentPreview,
  getLatestMainnetAutoLessons,
  getMainnetAutoStatus,
  listLiveCredentials,
  listMainnetAutoDecisions,
  liveStartCheck,
  refreshLiveReadiness,
  refreshLiveShadow,
  releaseLiveKillSwitch,
  runLivePreflight,
  selectLiveCredential,
  setLiveModePreference,
  startLiveAuto,
  startMainnetAutoDryRun,
  startMainnetAutoLiveBlocked,
  startLiveShadow,
  stopLiveAuto,
  stopMainnetAutoDryRun,
  stopLiveShadow,
  updateLiveCredential,
  validateLiveCredential
} from "../api/client";
import { notifyCommandError, notifyCommandSuccess } from "../lib/commandFeedback";
import { formatNumber, formatTime } from "../lib/format";
import { useAppStore } from "../store/appStore";
import type {
  LiveCredentialSummary,
  LiveEnvironment,
  LiveModePreference,
  LiveOrderRecord,
  LiveOrderPreflightResult,
  LiveOrderPreview,
  LiveOrderType,
  LiveRiskProfile,
  LiveStatusSnapshot
} from "../types";
import { Panel } from "./Panel";

const emptyDraft = {
  alias: "",
  environment: "testnet" as LiveEnvironment,
  api_key: "",
  api_secret: ""
};

export function LiveAccessPanel() {
  const queryClient = useQueryClient();
  const liveStatus = useAppStore((state) => state.liveStatus);
  const setLiveStatus = useAppStore((state) => state.setLiveStatus);
  const addToast = useAppStore((state) => state.addToast);
  const [draft, setDraft] = useState(emptyDraft);
  const [selectedId, setSelectedId] = useState<string>("");
  const [replaceSecrets, setReplaceSecrets] = useState(false);
  const [orderType, setOrderType] = useState<LiveOrderType>("MARKET");
  const [limitPrice, setLimitPrice] = useState("");
  const [mainnetConfirmText, setMainnetConfirmText] = useState("");

  const credentialsQuery = useQuery({
    queryKey: ["live-credentials"],
    queryFn: listLiveCredentials
  });

  useEffect(() => {
    if (!selectedId && liveStatus?.active_credential) {
      setSelectedId(liveStatus.active_credential.id);
    }
  }, [liveStatus?.active_credential, selectedId]);

  const selectedCredential =
    credentialsQuery.data?.find((credential) => credential.id === selectedId) ??
    (liveStatus?.active_credential?.id === selectedId ? liveStatus.active_credential : null);

  useEffect(() => {
    if (!selectedCredential) {
      return;
    }
    setDraft((current) => ({
      ...current,
      alias: selectedCredential.alias,
      environment: selectedCredential.environment
    }));
  }, [selectedCredential]);
  const selectedCredentialIsEnv = selectedCredential?.source === "env";

  const saveMutation = useMutation({
    mutationFn: async () => {
      if (selectedCredential) {
        return updateLiveCredential(selectedCredential.id, {
          alias: draft.alias,
          environment: draft.environment,
          api_key: replaceSecrets ? draft.api_key : undefined,
          api_secret: replaceSecrets ? draft.api_secret : undefined
        });
      }
      return createLiveCredential(draft);
    },
    onSuccess: async (credential) => {
      notifyCommandSuccess(addToast, "live_credential_save");
      setSelectedId(credential.id);
      setDraft((current) => ({ ...current, api_key: "", api_secret: "" }));
      setReplaceSecrets(false);
      await queryClient.invalidateQueries({ queryKey: ["live-credentials"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_credential_save", error)
  });

  const deleteMutation = useMutation({
    mutationFn: () => deleteLiveCredential(selectedId),
    onSuccess: async () => {
      notifyCommandSuccess(addToast, "live_credential_delete");
      setSelectedId("");
      setDraft(emptyDraft);
      await queryClient.invalidateQueries({ queryKey: ["live-credentials"] });
      await queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_credential_delete", error)
  });

  const selectMutation = useMutation({
    mutationFn: () => selectLiveCredential(selectedId),
    onSuccess: (status) => {
      setLiveStatus(status);
      notifyCommandSuccess(addToast, "live_mode");
    },
    onError: (error) => notifyCommandError(addToast, "live_mode", error)
  });

  const validateMutation = useMutation({
    mutationFn: () => validateLiveCredential(selectedId),
    onSuccess: async (result) => {
      if (result.status === "valid") {
        notifyCommandSuccess(addToast, "live_credential_validate");
      } else {
        addToast(result.message ?? "Live credential validation failed.", "error");
      }
      await queryClient.invalidateQueries({ queryKey: ["live-credentials"] });
      await queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_credential_validate", error)
  });

  const refreshMutation = useLiveStatusMutation(refreshLiveReadiness, "live_readiness_refresh", setLiveStatus, addToast);
  const armMutation = useLiveStatusMutation(armLive, "live_arm", setLiveStatus, addToast);
  const disarmMutation = useLiveStatusMutation(disarmLive, "live_disarm", setLiveStatus, addToast);
  const shadowStartMutation = useLiveStatusMutation(startLiveShadow, "live_shadow_start", setLiveStatus, addToast);
  const shadowStopMutation = useLiveStatusMutation(stopLiveShadow, "live_shadow_stop", setLiveStatus, addToast);
  const shadowRefreshMutation = useLiveStatusMutation(refreshLiveShadow, "live_shadow_refresh", setLiveStatus, addToast);
  const modeMutation = useLiveStatusMutation(
    (mode: LiveModePreference) => setLiveModePreference(mode),
    "live_mode",
    setLiveStatus,
    addToast
  );
  const previewMutation = useMutation({
    mutationFn: () => getLiveIntentPreview(orderType, orderType === "LIMIT" ? limitPrice : undefined),
    onSuccess: (preview) => {
      mergeIntentPreview(setLiveStatus, preview);
      notifyCommandSuccess(addToast, "live_intent_preview");
    },
    onError: (error) => notifyCommandError(addToast, "live_intent_preview", error)
  });
  const preflightMutation = useMutation({
    mutationFn: runLivePreflight,
    onSuccess: (result) => {
      mergePreflightResult(setLiveStatus, result);
      if (result.accepted) {
        addToast(result.message, "info");
      } else {
        addToast(result.message, "error");
      }
    },
    onError: (error) => notifyCommandError(addToast, "live_preflight", error)
  });
  const startCheckMutation = useMutation({
    mutationFn: liveStartCheck,
    onSuccess: (result) => {
      addToast(result.message, result.allowed ? "info" : "error");
    },
    onError: (error) => notifyCommandError(addToast, "live_start_check", error)
  });
  const killEngageMutation = useLiveStatusMutation(
    () => engageLiveKillSwitch("operator_engaged"),
    "live_kill_switch_engage",
    setLiveStatus,
    addToast
  );
  const killReleaseMutation = useLiveStatusMutation(
    () => releaseLiveKillSwitch("operator_released"),
    "live_kill_switch_release",
    setLiveStatus,
    addToast
  );
  const autoStartMutation = useLiveStatusMutation(startLiveAuto, "live_auto_start", setLiveStatus, addToast);
  const autoStopMutation = useLiveStatusMutation(stopLiveAuto, "live_auto_stop", setLiveStatus, addToast);
  const mainnetAutoQuery = useQuery({
    queryKey: ["mainnet-auto-status"],
    queryFn: getMainnetAutoStatus
  });
  const mainnetAutoDecisionsQuery = useQuery({
    queryKey: ["mainnet-auto-decisions"],
    queryFn: () => listMainnetAutoDecisions(5)
  });
  const mainnetAutoLessonsQuery = useQuery({
    queryKey: ["mainnet-auto-lessons"],
    queryFn: getLatestMainnetAutoLessons
  });
  const mainnetAutoDryRunStartMutation = useMutation({
    mutationFn: startMainnetAutoDryRun,
    onSuccess: async () => {
      notifyCommandSuccess(addToast, "live_start_check");
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-status"] });
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-decisions"] });
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-lessons"] });
      await queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_start_check", error)
  });
  const mainnetAutoDryRunStopMutation = useMutation({
    mutationFn: stopMainnetAutoDryRun,
    onSuccess: async () => {
      notifyCommandSuccess(addToast, "live_auto_stop");
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-status"] });
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-decisions"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_auto_stop", error)
  });
  const mainnetAutoLiveStartMutation = useMutation({
    mutationFn: startMainnetAutoLiveBlocked,
    onSuccess: async (status) => {
      addToast(`MAINNET live auto blocked: ${status.current_blockers.join(", ") || "server gates incomplete"}`, "error");
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-status"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_start_check", error)
  });
  const mainnetAutoEvidenceMutation = useMutation({
    mutationFn: exportMainnetAutoEvidence,
    onSuccess: async (result) => {
      addToast(`MAINNET auto evidence exported: ${result.path}`, "info");
      await queryClient.invalidateQueries({ queryKey: ["mainnet-auto-status"] });
    },
    onError: (error) => notifyCommandError(addToast, "live_start_check", error)
  });
  const riskProfileMutation = useLiveStatusMutation(
    () => configureLiveRiskProfile(defaultRiskProfile(liveStatus)),
    "live_risk_profile",
    setLiveStatus,
    addToast
  );
  const executeMutation = useMutation({
    mutationFn: () => {
      const current = useAppStore.getState().liveStatus;
      const intent = current?.intent_preview?.intent;
      if (current?.environment === "mainnet") {
        const required = current.mainnet_canary.required_confirmation;
        if (!required || mainnetConfirmText !== required) {
          throw new Error(`MAINNET canary requires exact confirmation: ${required ?? "unavailable"}`);
        }
        return executeLivePreview({
          intent_id: intent?.id ?? null,
          confirm_testnet: false,
          confirm_mainnet_canary: true,
          confirmation_text: mainnetConfirmText
        });
      }
      const confirmed = window.confirm(
        "Submit the displayed preview as a real TESTNET Binance Futures order? This is not mainnet, but it is an actual testnet exchange order."
      );
      if (!confirmed) {
        throw new Error("TESTNET execution cancelled by operator.");
      }
      return executeLivePreview({ intent_id: intent?.id ?? null, confirm_testnet: true });
    },
    onSuccess: (result) => {
      if (result.order) {
        mergeLiveOrder(setLiveStatus, result.order);
      }
      if (result.accepted) {
        notifyCommandSuccess(addToast, "live_execute");
      } else {
        addToast(result.message, "error");
      }
    },
    onError: (error) => notifyCommandError(addToast, "live_execute", error)
  });
  const cancelMutation = useMutation({
    mutationFn: (orderRef: string) => {
      const current = useAppStore.getState().liveStatus;
      if (current?.environment === "mainnet") {
        const order = current.execution.recent_orders.find((item) => item.id === orderRef || item.client_order_id === orderRef);
        const required = order ? `CANCEL MAINNET ${order.symbol} ${order.client_order_id}` : "";
        if (!required || mainnetConfirmText !== required) {
          throw new Error(`MAINNET canary cancel requires exact confirmation: ${required || "unavailable"}`);
        }
        return cancelLiveOrderWithPayload(orderRef, {
          confirm_testnet: false,
          confirm_mainnet_canary: true,
          confirmation_text: mainnetConfirmText
        });
      }
      const confirmed = window.confirm("Cancel this TESTNET Binance Futures order?");
      if (!confirmed) {
        throw new Error("TESTNET cancel cancelled by operator.");
      }
      return cancelLiveOrder(orderRef, true);
    },
    onSuccess: (result) => {
      if (result.order) {
        mergeLiveOrder(setLiveStatus, result.order);
      }
      if (result.accepted) {
        notifyCommandSuccess(addToast, "live_cancel");
      } else {
        addToast(result.message, "error");
      }
    },
    onError: (error) => notifyCommandError(addToast, "live_cancel", error)
  });
  const cancelAllMutation = useMutation({
    mutationFn: () => {
      const current = useAppStore.getState().liveStatus;
      if (current?.environment === "mainnet") {
        const symbol = useAppStore.getState().activeSymbol;
        const required = symbol ? `CANCEL ALL MAINNET ${symbol}` : "";
        if (!required || mainnetConfirmText !== required) {
          throw new Error(`MAINNET canary cancel-all requires exact confirmation: ${required || "unavailable"}`);
        }
        return cancelAllLiveOrdersWithPayload({
          confirm_testnet: false,
          confirm_mainnet_canary: true,
          confirmation_text: mainnetConfirmText
        });
      }
      const confirmed = window.confirm("Cancel all open TESTNET orders for the active symbol?");
      if (!confirmed) {
        throw new Error("TESTNET cancel-all cancelled by operator.");
      }
      return cancelAllLiveOrders(true);
    },
    onSuccess: (results) => {
      for (const result of results) {
        if (result.order) {
          mergeLiveOrder(setLiveStatus, result.order);
        }
      }
      notifyCommandSuccess(addToast, "live_cancel_all");
    },
    onError: (error) => notifyCommandError(addToast, "live_cancel_all", error)
  });
  const flattenMutation = useMutation({
    mutationFn: () => {
      const current = useAppStore.getState().liveStatus;
      if (current?.environment === "mainnet") {
        const symbol = useAppStore.getState().activeSymbol;
        const required = symbol ? `FLATTEN MAINNET ${symbol}` : "";
        if (!required || mainnetConfirmText !== required) {
          throw new Error(`MAINNET canary flatten requires exact confirmation: ${required || "unavailable"}`);
        }
        return flattenLivePositionWithPayload({
          confirm_testnet: false,
          confirm_mainnet_canary: true,
          confirmation_text: mainnetConfirmText
        });
      }
      const confirmed = window.confirm(
        "Flatten the active-symbol TESTNET position? This cancels open active-symbol orders first, then submits a reduce-only MARKET close if safe."
      );
      if (!confirmed) {
        throw new Error("TESTNET flatten cancelled by operator.");
      }
      return flattenLivePosition(true);
    },
    onSuccess: (result) => {
      if (result.flatten_order) {
        mergeLiveOrder(setLiveStatus, result.flatten_order);
      }
      if (result.accepted) {
        notifyCommandSuccess(addToast, "live_flatten");
      } else {
        addToast(result.message, "error");
      }
    },
    onError: (error) => notifyCommandError(addToast, "live_flatten", error)
  });

  if (!liveStatus?.feature_visible) {
    return null;
  }

  const busy =
    saveMutation.isPending ||
    deleteMutation.isPending ||
    selectMutation.isPending ||
    validateMutation.isPending ||
    refreshMutation.isPending ||
    armMutation.isPending ||
    disarmMutation.isPending ||
    shadowStartMutation.isPending ||
    shadowStopMutation.isPending ||
    shadowRefreshMutation.isPending ||
    previewMutation.isPending ||
    preflightMutation.isPending ||
    killEngageMutation.isPending ||
    killReleaseMutation.isPending ||
    autoStartMutation.isPending ||
    autoStopMutation.isPending ||
    mainnetAutoDryRunStartMutation.isPending ||
    mainnetAutoDryRunStopMutation.isPending ||
    mainnetAutoLiveStartMutation.isPending ||
    mainnetAutoEvidenceMutation.isPending ||
    riskProfileMutation.isPending ||
    executeMutation.isPending ||
    cancelMutation.isPending ||
    cancelAllMutation.isPending ||
    flattenMutation.isPending ||
    modeMutation.isPending ||
    startCheckMutation.isPending;
  const credentials = credentialsQuery.data ?? [];
  const openOrder = [...liveStatus.execution.recent_orders].reverse().find((order) => !isTerminalOrder(order));
  const liveBlockers = collectBlockingReasons(liveStatus);
  const liveWarnings = collectWarnings(liveStatus);
  const lastOrder =
    liveStatus.execution.recent_orders.length > 0
      ? liveStatus.execution.recent_orders[liveStatus.execution.recent_orders.length - 1]
      : null;
  const lastFill =
    liveStatus.execution.recent_fills.length > 0
      ? liveStatus.execution.recent_fills[liveStatus.execution.recent_fills.length - 1]
      : null;
  const mainnetAuto = mainnetAutoQuery.data ?? liveStatus.mainnet_auto ?? defaultMainnetAutoStatus();

  return (
    <div className="grid-span-6">
      <Panel title="LIVE ACCESS">
        <div className="live-access">
          <div className="status-strip">
            <button
              type="button"
              disabled={busy || liveStatus.mode_preference === "paper"}
              onClick={() => modeMutation.mutate("paper")}
            >
              PAPER MODE
            </button>
            <button
              type="button"
              disabled={busy || liveStatus.mode_preference === "live_read_only"}
              onClick={() => modeMutation.mutate("live_read_only")}
            >
              LIVE READ-ONLY
            </button>
          </div>

          <div className="operator-summary" aria-label="Live safety summary">
            <OperatorCard label="Safety Posture" value={safeDefaultMetric(liveStatus)} />
            <OperatorCard label="Execution" value={executionMetric(liveStatus)} />
            <OperatorCard label="Mainnet Auto" value="MAINNET AUTO BLOCKED" />
            <OperatorCard label="Mainnet Canary" value={mainnetCanaryMetric(liveStatus)} />
            <OperatorCard
              label="Position / Order"
              value={`${positionMetric(liveStatus)} · ${openOrder ? "OPEN ORDER PRESENT" : "NO OPEN ORDER"}`}
            />
            <OperatorCard label="Latest Order Truth" value={lastOrder ? orderOutcomeLabel(lastOrder) : "NO LIVE ORDER SUBMITTED"} />
          </div>

          <div className="blocker-strip" aria-label="Current safety blockers">
            <strong>{liveBlockers.length > 0 ? "Execution blocked by" : "Execution blockers"}</strong>
            <span>{liveBlockers.length > 0 ? liveBlockers.join(", ") : "NONE"}</span>
            {liveWarnings.length > 0 ? <span>Warnings: {liveWarnings.join(", ")}</span> : null}
          </div>

          <section className="live-section">
            <div className="live-section__header">
              <div>
                <h3>Credential</h3>
                <p>Masked metadata only. Env credentials cannot be edited or revealed here.</p>
              </div>
              <span>{liveStatus.active_credential ? credentialLabel(liveStatus.active_credential) : "NO ACTIVE CREDENTIAL"}</span>
            </div>
            <div className="field-grid">
              <Field label="Credential">
                <select
                  aria-label="Live Credential"
                  value={selectedId}
                  onChange={(event) => setSelectedId(event.target.value)}
                  disabled={busy}
                >
                  <option value="">New credential</option>
                  {credentials.map((credential) => (
                    <option key={credential.id} value={credential.id}>
                      {credentialLabel(credential)}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Alias">
                <input
                  aria-label="Live Alias"
                  value={draft.alias}
                  disabled={busy || selectedCredentialIsEnv}
                  onChange={(event) => setDraft((current) => ({ ...current, alias: event.target.value }))}
                />
              </Field>
              <Field label="Environment">
                <select
                  aria-label="Live Environment"
                  value={draft.environment}
                  disabled={busy || selectedCredentialIsEnv}
                  onChange={(event) =>
                    setDraft((current) => ({ ...current, environment: event.target.value as LiveEnvironment }))
                  }
                >
                  <option value="testnet">testnet</option>
                  <option value="mainnet">mainnet</option>
                </select>
              </Field>
              <Field label={selectedCredentialIsEnv ? "API Key" : selectedCredential ? "Replace Secret" : "API Key"}>
                {selectedCredential && !replaceSecrets && !selectedCredentialIsEnv ? (
                  <button type="button" disabled={busy} onClick={() => setReplaceSecrets(true)}>
                    REPLACE STORED SECRET
                  </button>
                ) : (
                  <input
                    aria-label="Live API Key"
                    value={draft.api_key}
                    disabled={busy || selectedCredentialIsEnv}
                    autoComplete="off"
                    onChange={(event) => setDraft((current) => ({ ...current, api_key: event.target.value }))}
                  />
                )}
              </Field>
              <Field label="API Secret">
                <input
                  aria-label="Live API Secret"
                  value={draft.api_secret}
                  disabled={busy || selectedCredentialIsEnv || (selectedCredential !== null && !replaceSecrets)}
                  type="password"
                  autoComplete="off"
                  onChange={(event) => setDraft((current) => ({ ...current, api_secret: event.target.value }))}
                />
              </Field>
            </div>
            <div className="action-row">
              <button
                type="button"
                disabled={
                  busy ||
                  selectedCredentialIsEnv ||
                  !draft.alias ||
                  (!selectedCredential && (!draft.api_key || !draft.api_secret)) ||
                  (selectedCredential !== null && replaceSecrets && (!draft.api_key || !draft.api_secret))
                }
                onClick={() => saveMutation.mutate()}
              >
                {selectedCredential ? "Update Credential" : "Create Credential"}
              </button>
              <button type="button" disabled={busy || !selectedId} onClick={() => selectMutation.mutate()}>
                Select Active
              </button>
              <button type="button" disabled={busy || !selectedId} onClick={() => validateMutation.mutate()}>
                Validate
              </button>
              <button type="button" disabled={busy || !selectedId || selectedCredentialIsEnv} onClick={() => deleteMutation.mutate()}>
                Delete
              </button>
            </div>
          </section>

          <section className="live-section">
            <div className="live-section__header">
              <div>
                <h3>Readiness / Shadow / Account</h3>
                <p>Read-only account truth and shadow state. Missing or stale data keeps execution blocked.</p>
              </div>
              <span>{shadowMetric(liveStatus)}</span>
            </div>
            <div className="metric-grid metric-grid--compact">
              <Metric label="Live State" value={stateLabel(liveStatus.state)} />
              <Metric label="Environment" value={liveStatus.environment.toUpperCase()} />
              <Metric label="Armed" value={liveStatus.armed ? "ARMED READ-ONLY" : "DISARMED"} />
              <Metric label="Shadow" value={shadowMetric(liveStatus)} />
              <Metric label="Account" value={accountMetric(liveStatus)} />
              <Metric label="Rules" value={liveStatus.symbol_rules ? `${liveStatus.symbol_rules.symbol} RULES FRESH` : "RULES MISSING"} />
            </div>
            <div className="action-row">
              <button type="button" disabled={busy} onClick={() => refreshMutation.mutate(undefined)}>
                Refresh Readiness
              </button>
              <button type="button" disabled={busy || !liveStatus.readiness.can_arm} onClick={() => armMutation.mutate(undefined)}>
                Arm Read-Only
              </button>
              <button type="button" disabled={busy || !liveStatus.armed} onClick={() => disarmMutation.mutate(undefined)}>
                Disarm
              </button>
              <button type="button" disabled={busy} onClick={() => startCheckMutation.mutate()}>
                Start Live Check
              </button>
              <button type="button" disabled={busy || !selectedId} onClick={() => shadowStartMutation.mutate(undefined)}>
                Start Shadow Sync
              </button>
              <button type="button" disabled={busy} onClick={() => shadowStopMutation.mutate(undefined)}>
                Stop Shadow Sync
              </button>
              <button type="button" disabled={busy} onClick={() => shadowRefreshMutation.mutate(undefined)}>
                Refresh Shadow
              </button>
            </div>
          </section>

          <section className="live-section">
            <div className="live-section__header">
              <div>
                <h3>Preview / Preflight</h3>
                <p>Preflight is validation only. It never means an order was placed.</p>
              </div>
              <span>{preflightMetric(liveStatus)}</span>
            </div>
            <div className="field-grid">
              <Field label="Intent Type">
                <select
                  aria-label="Live Intent Type"
                  value={orderType}
                  disabled={busy}
                  onChange={(event) => setOrderType(event.target.value as LiveOrderType)}
                >
                  <option value="MARKET">MARKET</option>
                  <option value="LIMIT">LIMIT</option>
                </select>
              </Field>
              <Field label="Limit Price">
                <input
                  aria-label="Live Limit Price"
                  value={limitPrice}
                  disabled={busy || orderType !== "LIMIT"}
                  inputMode="decimal"
                  onChange={(event) => setLimitPrice(event.target.value)}
                  placeholder={orderType === "LIMIT" ? "Required for LIMIT" : "n/a for MARKET"}
                />
              </Field>
              <Field label="Mainnet Confirmation">
                <input
                  aria-label="Mainnet Canary Confirmation"
                  value={mainnetConfirmText}
                  disabled={busy || liveStatus.environment !== "mainnet"}
                  autoComplete="off"
                  onChange={(event) => setMainnetConfirmText(event.target.value)}
                  placeholder={
                    liveStatus.environment === "mainnet"
                      ? (liveStatus.mainnet_canary.required_confirmation ?? "Build a preview to see required text")
                      : "n/a for TESTNET"
                  }
                />
                <small>
                  {liveStatus.environment === "mainnet"
                    ? `Required: ${liveStatus.mainnet_canary.required_confirmation ?? "unavailable until preview is ready"}`
                    : "MAINNET canary controls stay inactive in TESTNET."}
                </small>
              </Field>
            </div>
            <div className="action-row">
              <button type="button" disabled={busy || (orderType === "LIMIT" && !limitPrice.trim())} onClick={() => previewMutation.mutate()}>
                Build Preview
              </button>
              <button type="button" disabled={busy} onClick={() => preflightMutation.mutate()}>
                Run Preflight
              </button>
            </div>
            <div className="muted">{intentSummary(liveStatus.intent_preview)}</div>
            <div className="muted">{referencePriceSummary(liveStatus.intent_preview)} · {marketabilitySummary(liveStatus.intent_preview)}</div>
          </section>

          <section className="live-section live-section--safety">
            <div className="live-section__header">
              <div>
                <h3>Safety / Canary Controls</h3>
                <p>Mainnet auto is blocked. Mainnet canary remains manual, session-only, and exact-confirmation gated.</p>
              </div>
              <span>{liveStatus.kill_switch.engaged ? "KILL SWITCH ENGAGED" : "SAFE DEFAULT"}</span>
            </div>
            <div className="metric-grid metric-grid--compact">
              <Metric label="Kill Switch" value={liveStatus.kill_switch.engaged ? "KILL SWITCH ENGAGED" : "KILL SWITCH RELEASED"} />
              <Metric label="Auto Executor" value={autoMetric(liveStatus)} />
              <Metric label="Risk Profile" value={riskMetric(liveStatus)} />
              <Metric label="Mainnet Canary" value={mainnetCanaryMetric(liveStatus)} />
            </div>
            <div className="action-row">
              <button type="button" disabled={busy || liveStatus.kill_switch.engaged} onClick={() => killEngageMutation.mutate(undefined)}>
                Engage Kill Switch
              </button>
              <button type="button" disabled={busy || !liveStatus.kill_switch.engaged} onClick={() => killReleaseMutation.mutate(undefined)}>
                Release Kill Switch
              </button>
              <button type="button" disabled={busy || liveStatus.risk_profile.configured} onClick={() => riskProfileMutation.mutate(undefined)}>
                Configure Conservative Risk Profile
              </button>
              <button
                type="button"
                disabled={busy || liveStatus.environment !== "testnet" || liveStatus.auto_executor.state === "running"}
                onClick={() => autoStartMutation.mutate(undefined)}
              >
                Start TESTNET Auto
              </button>
              <button
                type="button"
                disabled={busy || liveStatus.auto_executor.state !== "running"}
                onClick={() => autoStopMutation.mutate(undefined)}
              >
                Stop TESTNET Auto
              </button>
            </div>
          </section>

          <section className="live-section live-section--safety">
            <div className="live-section__header">
              <div>
                <h3>Mainnet Auto Dry-Run</h3>
                <p>Infrastructure only. Dry-run records decisions and lessons; live auto remains server-blocked by default.</p>
              </div>
              <span>{mainnetAutoMetric(mainnetAuto)}</span>
            </div>
            <div className="metric-grid metric-grid--compact">
              <Metric label="Mainnet Auto" value={mainnetAutoMetric(mainnetAuto)} />
              <Metric label="Mode" value={mainnetAuto.mode.toUpperCase()} />
              <Metric
                label="Live Config"
                value={mainnetAuto.config.enable_live_execution ? "LIVE CONFIG ENABLED" : "LIVE CONFIG BLOCKED"}
              />
              <Metric
                label="Risk Budget"
                value={`max order ${mainnetAuto.risk_budget.max_notional_per_order} · max leverage ${mainnetAuto.risk_budget.max_leverage}`}
              />
              <Metric
                label="Margin Policy"
                value={`actual ${mainnetAuto.margin_policy.actual_margin_type.toUpperCase()} · allowed ${mainnetAuto.margin_policy.allowed_margin_type.toUpperCase()}${mainnetAuto.margin_policy.blocker ? ` · ${mainnetAuto.margin_policy.blocker}` : ""}`}
              />
              <Metric
                label="ASO Policy"
                value={`${mainnetAuto.position_policy.policy.replaceAll("_", " ").toUpperCase()} · desired ${mainnetAuto.position_policy.desired_side.toUpperCase()} · action ${mainnetAuto.position_policy.last_action.replaceAll("_", " ").toUpperCase()}`}
              />
              <Metric
                label="Watchdog"
                value={mainnetAuto.last_watchdog_stop_reason ?? "NO WATCHDOG STOP"}
              />
              <Metric
                label="Lessons"
                value={mainnetAutoLessonsQuery.data?.recommendation ?? mainnetAuto.latest_lessons_recommendation ?? "NO LESSON REPORT"}
              />
            </div>
            <div className="blocker-strip" aria-label="Mainnet auto blockers">
              <strong>MAINNET auto blockers</strong>
              <span>
                {mainnetAuto.current_blockers.length > 0
                  ? mainnetAuto.current_blockers.join(", ")
                  : "NONE FOR DRY-RUN"}
              </span>
              <span>Live auto start is intentionally not an easy UI action; backend gates still block it by default.</span>
            </div>
            <div className="action-row">
              <button type="button" disabled={busy} onClick={() => mainnetAutoDryRunStartMutation.mutate()}>
                Start MAINNET Auto Dry-Run
              </button>
              <button
                type="button"
                disabled={busy || mainnetAuto.state !== "dry_run_running"}
                onClick={() => mainnetAutoDryRunStopMutation.mutate()}
              >
                Stop Dry-Run
              </button>
              <button type="button" disabled={busy} onClick={() => mainnetAutoEvidenceMutation.mutate()}>
                Export Auto Evidence
              </button>
              <button type="button" disabled={busy} onClick={() => mainnetAutoLiveStartMutation.mutate()}>
                Verify Live Auto Blocked
              </button>
            </div>
            <div className="muted">
              Last decision: {mainnetAutoDecisionsQuery.data?.at(-1)?.outcome ?? mainnetAuto.last_decision_outcome ?? "none"} · no UI state here submits an order.
            </div>
          </section>

          <section className="live-section">
            <div className="live-section__header">
              <div>
                <h3>Orders / Fills</h3>
                <p>ACK is request acceptance, not a fill. Canceled orders stay distinct from fills.</p>
              </div>
              <span>{lastFill ? "FILL RECORDED" : "NO FILL"}</span>
            </div>
            <div className="metric-grid metric-grid--compact">
              <Metric label="Last Order" value={lastOrder ? orderOutcomeLabel(lastOrder) : "NO LIVE ORDER SUBMITTED"} />
              <Metric label="Last Fill" value={lastFill ? `${lastFill.side} ${lastFill.symbol} qty ${lastFill.quantity}` : "NO FILL RECORDED"} />
            </div>
            <div className="action-row">
              <button
                type="button"
                disabled={busy || !liveStatus.execution.can_submit || !liveStatus.intent_preview?.intent}
                onClick={() => executeMutation.mutate()}
              >
                {liveStatus.environment === "mainnet" ? "Execute MAINNET Canary Preview" : "Execute TESTNET Preview"}
              </button>
              <button type="button" disabled={busy || !openOrder} onClick={() => openOrder && cancelMutation.mutate(openOrder.id)}>
                {liveStatus.environment === "mainnet" ? "Cancel Open MAINNET Canary Order" : "Cancel Open TESTNET Order"}
              </button>
              <button type="button" disabled={busy || !openOrder} onClick={() => cancelAllMutation.mutate()}>
                Cancel All Active-Symbol Orders
              </button>
              <button type="button" disabled={busy} onClick={() => flattenMutation.mutate()}>
                {liveStatus.environment === "mainnet" ? "Flatten MAINNET Canary Position" : "Flatten TESTNET Position"}
              </button>
            </div>
          </section>

          <details className="live-section live-section--details">
            <summary>Advanced safety and exchange details</summary>
            <StatusLists status={liveStatus} activeCredential={liveStatus.active_credential} />
          </details>
        </div>
      </Panel>
    </div>
  );
}

function useLiveStatusMutation<TArg>(
  mutationFn: (arg: TArg) => Promise<LiveStatusSnapshot>,
  command: Parameters<typeof notifyCommandSuccess>[1],
  setLiveStatus: (status: LiveStatusSnapshot) => void,
  addToast: (message: string, kind?: "info" | "error") => void
) {
  return useMutation({
    mutationFn,
    onSuccess: (status) => {
      setLiveStatus(status);
      notifyCommandSuccess(addToast, command);
    },
    onError: (error) => notifyCommandError(addToast, command, error)
  });
}

function mergeIntentPreview(setLiveStatus: (status: LiveStatusSnapshot) => void, preview: LiveOrderPreview) {
  const current = useAppStore.getState().liveStatus;
  if (!current) {
    return;
  }
  setLiveStatus({
    ...current,
    state: preview.intent && preview.blocking_reasons.length === 0 ? "preflight_ready" : "preflight_blocked",
    intent_preview: preview,
    updated_at: preview.built_at
  });
}

function mergePreflightResult(setLiveStatus: (status: LiveStatusSnapshot) => void, result: LiveOrderPreflightResult) {
  const current = useAppStore.getState().liveStatus;
  if (!current) {
    return;
  }
  const recent = [...current.recent_preflights.filter((item) => item.id !== result.id), result].slice(-50);
  setLiveStatus({
    ...current,
    recent_preflights: recent,
    updated_at: result.created_at
  });
}

function mergeLiveOrder(setLiveStatus: (status: LiveStatusSnapshot) => void, order: LiveOrderRecord) {
  const current = useAppStore.getState().liveStatus;
  if (!current) {
    return;
  }
  const recentOrders = [...current.execution.recent_orders.filter((item) => item.id !== order.id), order].slice(-50);
  setLiveStatus({
    ...current,
    execution: {
      ...current.execution,
      active_order: isTerminalOrder(order) ? current.execution.active_order : order,
      recent_orders: recentOrders,
      updated_at: order.updated_at
    },
    updated_at: order.updated_at
  });
}

function credentialLabel(credential: LiveCredentialSummary): string {
  const sourceLabel =
    credential.source === "env" ? `ENV ${credential.environment.toUpperCase()}` : credential.environment;
  return `${sourceLabel} · ${credential.alias} · ${credential.api_key_hint}`;
}

function OperatorCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="operator-card">
      <div className="operator-card__label">{label}</div>
      <div className="operator-card__value">{value}</div>
    </div>
  );
}

function collectBlockingReasons(status: LiveStatusSnapshot): string[] {
  return unique([
    ...status.readiness.blocking_reasons,
    ...status.reconciliation.blocking_reasons,
    ...(status.intent_preview?.blocking_reasons ?? []),
    ...status.execution.blocking_reasons,
    ...status.auto_executor.blocking_reasons,
    ...status.mainnet_canary.blocking_reasons
  ]);
}

function collectWarnings(status: LiveStatusSnapshot): string[] {
  return unique([...status.readiness.warnings, ...status.reconciliation.warnings, ...status.execution.warnings]);
}

function safeDefaultMetric(status: LiveStatusSnapshot): string {
  if (status.kill_switch.engaged) {
    return "KILL SWITCH ENGAGED";
  }
  if (status.environment === "mainnet" && !status.mainnet_canary.manual_execution_enabled) {
    return "SAFE DEFAULT";
  }
  if (status.environment === "testnet" && status.execution.can_submit) {
    return "TESTNET READY";
  }
  return "EXECUTION BLOCKED";
}

function positionMetric(status: LiveStatusSnapshot): string {
  const activePosition = status.account_snapshot?.positions.find(
    (position) => position.symbol === status.symbol_rules?.symbol && position.position_amt !== 0
  );
  return activePosition ? `${activePosition.symbol} POSITION ${activePosition.position_amt}` : "NO OPEN POSITION";
}

function accountMetric(status: LiveStatusSnapshot): string {
  const account = status.account_snapshot;
  if (!account) {
    return "ACCOUNT MISSING";
  }
  const mode = account.position_mode ?? "mode unknown";
  const multi = account.multi_assets_margin === true ? "MULTI-ASSET" : "SINGLE-ASSET";
  return `${mode.toUpperCase()} · ${multi} · available ${formatNumber(account.available_balance)}`;
}

function orderOutcomeLabel(order: LiveOrderRecord): string {
  if (order.status === "canceled") {
    return "ORDER CANCELED · NO FILL CLAIMED";
  }
  if (order.status === "filled") {
    return `ORDER FILLED · qty ${order.executed_qty}`;
  }
  if (order.status === "accepted") {
    return "ACK ACCEPTED · WAITING RECONCILIATION";
  }
  return `${order.status.replaceAll("_", " ").toUpperCase()} · filled ${order.executed_qty}`;
}

function StatusLists({
  status,
  activeCredential
}: {
  status: NonNullable<ReturnType<typeof useAppStore.getState>["liveStatus"]>;
  activeCredential: LiveCredentialSummary | null;
}) {
  const blockingReasons = collectBlockingReasons(status);
  const warnings = collectWarnings(status);
  const shadow = status.reconciliation.shadow;
  const preview = status.intent_preview;
  const lastPreflight =
    status.recent_preflights.length > 0 ? status.recent_preflights[status.recent_preflights.length - 1] : null;
  const lastOrder =
    status.execution.recent_orders.length > 0
      ? status.execution.recent_orders[status.execution.recent_orders.length - 1]
      : null;
  const lastFill =
    status.execution.recent_fills.length > 0
      ? status.execution.recent_fills[status.execution.recent_fills.length - 1]
      : null;

  return (
    <div className="live-access__details">
      <div className="list">
        <div className="list-item">
          <strong>ACTIVE CREDENTIAL</strong>
          <span>
            {activeCredential
              ? `${credentialLabel(activeCredential)} · ${activeCredential.validation_status}`
              : "NONE"}
          </span>
          <span>last validated {formatTime(activeCredential?.last_validated_at ?? null)}</span>
        </div>
        <div className="list-item">
          <strong>BLOCKING REASONS</strong>
          <span>{blockingReasons.length > 0 ? blockingReasons.join(", ") : "NONE"}</span>
        </div>
        <div className="list-item">
          <strong>WARNINGS</strong>
          <span>{warnings.length > 0 ? warnings.join(", ") : "NONE"}</span>
        </div>
        <div className="list-item">
          <strong>KILL SWITCH / RISK</strong>
          <span>
            {status.kill_switch.engaged
              ? `KILL SWITCH ENGAGED · ${status.kill_switch.reason ?? "no reason provided"}`
              : "KILL SWITCH CLEAR"}
          </span>
          <span>
            {status.risk_profile.configured
              ? `${status.risk_profile.profile_name ?? "configured"} · max order ${status.risk_profile.limits.max_notional_per_order} · max leverage ${status.risk_profile.limits.max_leverage}`
              : "Explicit operator risk profile is required before MAINNET canary readiness."}
          </span>
        </div>
        <div className="list-item">
          <strong>AUTO EXECUTOR</strong>
          <span>
            {status.auto_executor.state.toUpperCase()} · {status.auto_executor.environment.toUpperCase()} ·{" "}
            {status.auto_executor.order_type}
          </span>
          <span>
            {status.auto_executor.last_message ??
              "TESTNET auto consumes closed-candle signals only and suppresses duplicate candle intents."}
          </span>
        </div>
        <div className="list-item">
          <strong>MAINNET CANARY</strong>
          <span>{mainnetCanaryMetric(status)}</span>
          <span>
            {status.mainnet_canary.required_confirmation
              ? `Exact confirmation required: ${status.mainnet_canary.required_confirmation}`
              : "Mainnet is disabled by default and requires server canary enablement plus a configured risk profile."}
          </span>
        </div>
        <div className="list-item">
          <strong>SYMBOL RULES</strong>
          <span>
            {status.symbol_rules
              ? `${status.symbol_rules.symbol} ${status.symbol_rules.status} tick ${status.symbol_rules.filters.tick_size ?? "n/a"} step ${status.symbol_rules.filters.step_size ?? "n/a"} min notional ${status.symbol_rules.filters.min_notional ?? "n/a"}`
              : "MISSING"}
          </span>
        </div>
        <div className="list-item">
          <strong>SHADOW STREAM</strong>
          <span>
            {status.reconciliation.stream.state.toUpperCase()}
            {status.reconciliation.stream.stale ? " · STALE" : " · FRESH"}
            {status.reconciliation.stream.detail ? ` · ${status.reconciliation.stream.detail}` : ""}
          </span>
          <span>
            last event {formatTime(status.reconciliation.stream.last_event_time)} · rest sync{" "}
            {formatTime(status.reconciliation.stream.last_rest_sync_at)}
          </span>
        </div>
        <div className="list-item">
          <strong>SHADOW ACCOUNT</strong>
          <span>
            {shadow
              ? `${shadow.environment.toUpperCase()} · balances ${shadow.balances.length} · positions ${shadow.positions.length} · open orders ${shadow.open_orders.length}`
              : "MISSING"}
          </span>
          <span>{shadow ? (shadow.ambiguous ? "AMBIGUOUS SHADOW STATE" : "SHADOW STATE COHERENT") : "no shadow snapshot"}</span>
        </div>
        <div className="list-item">
          <strong>ACCOUNT SNAPSHOT</strong>
          <span>
            {status.account_snapshot
              ? `available ${formatNumber(status.account_snapshot.available_balance)} · positions ${status.account_snapshot.positions.length} · position mode ${status.account_snapshot.position_mode ?? "unknown"} · multi-assets ${status.account_snapshot.multi_assets_margin === true ? "on" : "off"}`
              : "MISSING"}
          </span>
          <span>
            {status.account_snapshot?.account_mode_checked_at
              ? `account mode checked ${formatTime(status.account_snapshot.account_mode_checked_at)}`
              : "dedicated position/multi-assets checks have not completed"}
          </span>
        </div>
        <div className="list-item">
          <strong>INTENT PREVIEW</strong>
          <span>{intentSummary(preview)}</span>
          <span>{preview ? preview.message : "Build preview before preflight."}</span>
        </div>
        <div className="list-item">
          <strong>REFERENCE PRICE</strong>
          <span>{referencePriceSummary(preview)}</span>
          <span>{marketabilitySummary(preview)}</span>
        </div>
        <div className="list-item">
          <strong>LAST PREFLIGHT</strong>
          <span>{preflightSummary(lastPreflight)}</span>
          <span>{lastPreflight ? preflightMessage(lastPreflight) : "No preflight result yet."}</span>
        </div>
        <div className="list-item">
          <strong>LIVE ORDER STATE</strong>
          <span>{lastOrder ? orderSummary(lastOrder) : "NO LIVE ORDER SUBMITTED"}</span>
          <span>{lastOrder?.last_error ?? "Submissions use ACK; user-data stream and recent-window REST repair define final truth."}</span>
        </div>
        <div className="list-item">
          <strong>LIVE FILL STATE</strong>
          <span>{lastFill ? `${lastFill.side} ${lastFill.symbol} qty ${lastFill.quantity} @ ${lastFill.price}` : "NO FILL RECORDED"}</span>
          <span>{lastFill ? `trade ${lastFill.trade_id ?? "n/a"} · fee ${lastFill.commission ?? "0"} ${lastFill.commission_asset ?? ""}` : "fills are not inferred locally"}</span>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: React.PropsWithChildren<{ label: string }>) {
  return (
    <div className="field">
      <label>{label}</label>
      {children}
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

function stateLabel(state: string): string {
  switch (state) {
    case "ready_read_only":
      return "READY READ-ONLY";
    case "armed_read_only":
      return "ARMED READ-ONLY";
    case "start_blocked":
      return "START BLOCKED";
    case "shadow_running":
      return "LIVE SHADOW ACTIVE";
    case "shadow_degraded":
      return "SHADOW DEGRADED";
    case "preflight_ready":
      return "PREFLIGHT READY";
    case "preflight_blocked":
      return "PREFLIGHT BLOCKED";
    case "testnet_execution_ready":
      return "TESTNET EXECUTION READY";
    case "testnet_auto_ready":
      return "TESTNET AUTO READY";
    case "testnet_auto_running":
      return "TESTNET AUTO RUNNING";
    case "testnet_submit_pending":
      return "ORDER SUBMIT PENDING";
    case "testnet_order_open":
      return "WORKING";
    case "testnet_partially_filled":
      return "PARTIALLY FILLED";
    case "testnet_filled":
      return "FILLED";
    case "testnet_cancel_pending":
      return "CANCEL PENDING";
    case "execution_degraded":
      return "EXECUTION DEGRADED";
    case "execution_blocked":
      return "EXECUTION BLOCKED";
    case "mainnet_execution_blocked":
      return "MAINNET EXECUTION BLOCKED";
    case "mainnet_canary_ready":
      return "MAINNET CANARY READY";
    case "mainnet_manual_execution_enabled":
      return "MAINNET MANUAL EXECUTION ENABLED";
    case "kill_switch_engaged":
      return "KILL SWITCH ENGAGED";
    case "execution_not_implemented":
      return "EXECUTION NOT IMPLEMENTED";
    default:
      return state.replaceAll("_", " ").toUpperCase();
  }
}

function executionMetric(status: LiveStatusSnapshot): string {
  if (status.environment === "mainnet") {
    if (status.mainnet_canary.manual_execution_enabled) {
      return "MAINNET MANUAL EXECUTION ENABLED";
    }
    if (status.mainnet_canary.canary_ready) {
      return "MAINNET CANARY READY";
    }
    return "MAINNET EXECUTION BLOCKED";
  }
  if (status.execution.can_submit) {
    return "TESTNET EXECUTION READY";
  }
  return stateLabel(status.execution.state);
}

function autoMetric(status: LiveStatusSnapshot): string {
  if (status.auto_executor.state === "running") {
    return "TESTNET AUTO RUNNING";
  }
  if (status.auto_executor.state === "ready") {
    return "TESTNET AUTO READY";
  }
  if (status.auto_executor.state === "blocked") {
    return `AUTO BLOCKED · ${status.auto_executor.blocking_reasons.join(", ") || "reason unavailable"}`;
  }
  if (status.auto_executor.state === "degraded") {
    return "AUTO DEGRADED";
  }
  return "AUTO STOPPED";
}

function mainnetAutoMetric(status: LiveStatusSnapshot["mainnet_auto"]): string {
  if (status.state === "dry_run_running") {
    return "MAINNET AUTO DRY-RUN RUNNING";
  }
  if (status.state === "watchdog_stopped") {
    return `WATCHDOG STOPPED · ${status.last_watchdog_stop_reason ?? "reason unavailable"}`;
  }
  if (status.state === "live_running") {
    return "MAINNET AUTO LIVE RUNNING";
  }
  if (!status.config.enable_live_execution) {
    return "MAINNET AUTO BLOCKED BY SERVER CONFIG";
  }
  return `MAINNET AUTO ${status.state.replaceAll("_", " ").toUpperCase()}`;
}

function defaultMainnetAutoStatus(): LiveStatusSnapshot["mainnet_auto"] {
  return {
    state: "disabled",
    mode: "dry_run",
    config: {
      enable_live_execution: false,
      mode: "dry_run",
      max_runtime_minutes: 15,
      max_orders: 1,
      max_fills: 1,
      max_notional: "80",
      max_daily_loss: "5",
      require_flat_start: true,
      require_flat_stop: true,
      require_manual_canary_evidence: true,
      evidence_required: true,
      lesson_report_required: true,
      allowed_margin_type: "isolated",
      position_policy: "crossover_only",
      aso_delta_threshold: "5",
      aso_zone_threshold: "55"
    },
    risk_budget: {
      configured: true,
      budget_id: "mainnet-auto-dry-run-default",
      max_notional_per_order: "80",
      max_total_session_notional: "80",
      max_open_notional: "80",
      max_orders_per_session: 1,
      max_fills_per_session: 1,
      max_consecutive_losses: 1,
      max_consecutive_rejections: 1,
      max_daily_realized_loss: "5",
      max_position_age_seconds: 300,
      max_runtime_minutes: 15,
      max_leverage: "5",
      require_flat_start: true,
      require_flat_stop: true,
      allowed_symbols: ["BTCUSDT"],
      allowed_order_types: ["LIMIT"],
      require_fresh_reference_price: true,
      require_fresh_shadow: true,
      require_fresh_user_data_stream: true,
      require_evidence_logging: true,
      require_lessons_report: true,
      updated_at: 0
    },
    watchdog: {
      running: false,
      last_check_at: null,
      last_stop_reason: null,
      last_message: "Mainnet auto watchdog is idle."
    },
    margin_policy: {
      allowed_margin_type: "isolated",
      actual_margin_type: "unknown",
      allowed: false,
      blocker: "margin_type_unknown",
      warning: null
    },
    position_policy: {
      policy: "crossover_only",
      aso_delta_threshold: "5",
      aso_zone_threshold: "55",
      last_bulls: null,
      last_bears: null,
      last_delta: null,
      last_zone: null,
      desired_side: "none",
      current_side: "none",
      last_action: "no_trade",
      last_blocker: null,
      last_reason: "not_evaluated"
    },
    session_id: null,
    started_at: null,
    expires_at: null,
    stopped_at: null,
    last_heartbeat_at: null,
    last_signal_id: null,
    last_signal_open_time: null,
    last_order_id: null,
    last_decision_id: null,
    last_decision_outcome: null,
    last_watchdog_stop_reason: null,
    blocking_reasons: ["mainnet_auto_config_disabled"],
    current_blockers: ["mainnet_auto_config_disabled"],
    latest_lessons_recommendation: "live_not_allowed",
    live_orders_submitted: 0,
    dry_run_orders_submitted: 0,
    evidence_path: null,
    updated_at: 0
  };
}

function riskMetric(status: LiveStatusSnapshot): string {
  if (!status.risk_profile.configured) {
    return "RISK PROFILE REQUIRED";
  }
  return `${status.risk_profile.profile_name ?? "CONFIGURED"} · max ${status.risk_profile.limits.max_notional_per_order}`;
}

function mainnetCanaryMetric(status: LiveStatusSnapshot): string {
  if (!status.mainnet_canary.enabled_by_server) {
    return "MAINNET CANARY DISABLED BY SERVER";
  }
  if (!status.mainnet_canary.risk_profile_configured) {
    return "MAINNET CANARY NEEDS RISK PROFILE";
  }
  if (status.mainnet_canary.manual_execution_enabled) {
    return "MAINNET MANUAL EXECUTION ENABLED";
  }
  if (status.mainnet_canary.canary_ready) {
    return "MAINNET CANARY READY";
  }
  return `MAINNET CANARY BLOCKED · ${status.mainnet_canary.blocking_reasons.join(", ") || "gates incomplete"}`;
}

function shadowMetric(status: LiveStatusSnapshot): string {
  const stream = status.reconciliation.stream;
  if (stream.stale) {
    return `${stream.state.toUpperCase()} STALE`;
  }
  return stream.state.toUpperCase();
}

function preflightMetric(status: LiveStatusSnapshot): string {
  const preview = status.intent_preview;
  if (!preview) {
    return "NOT BUILT";
  }
  if (preview.intent && preview.blocking_reasons.length === 0) {
    return "PREFLIGHT READY";
  }
  return "PREFLIGHT BLOCKED";
}

function intentSummary(preview: LiveOrderPreview | null): string {
  if (!preview?.intent) {
    return preview?.blocking_reasons.length ? `BLOCKED · ${preview.blocking_reasons.join(", ")}` : "NOT BUILT";
  }
  const intent = preview.intent;
  const price = intent.price ? ` @ ${intent.price}` : "";
  const notes = intent.validation_notes.length > 0 ? ` · ${intent.validation_notes.join("; ")}` : "";
  const executionScope =
    intent.environment === "mainnet" ? "MAINNET CANARY GATED" : "TESTNET EXECUTION GATED";
  return `${intent.side} ${intent.order_type} ${intent.symbol} qty ${intent.quantity}${price} · ${executionScope} · ${intent.can_preflight ? "CAN PREFLIGHT" : "PREFLIGHT BLOCKED"} · ${intent.can_execute_now ? "CAN EXECUTE IF GATES PASS" : "EXECUTION BLOCKED"}${notes}`;
}

function referencePriceSummary(preview: LiveOrderPreview | null): string {
  const reference = preview?.reference_price;
  if (!reference) {
    return "NOT RESOLVED";
  }
  if (!reference.price) {
    return `UNAVAILABLE · ${reference.failure_reason ?? reference.blocking_reason ?? "reason unavailable"}`;
  }
  const stale = reference.stale ? "STALE" : "FRESH";
  return `${reference.price} · ${reference.source ?? "unknown source"} · ${stale} · age ${reference.age_ms ?? "n/a"}ms`;
}

function marketabilitySummary(preview: LiveOrderPreview | null): string {
  const check = preview?.marketability_check;
  if (!check) {
    return "No marketability check recorded.";
  }
  const result =
    check.marketable_after_rounding === true
      ? "MARKETABLE"
      : check.marketable_after_rounding === false
        ? "NON-MARKETABLE"
        : "UNKNOWN";
  return `${result} · rounded ${check.rounded_order_price ?? "n/a"} vs reference ${check.reference_price ?? "n/a"}`;
}

function preflightSummary(result: LiveOrderPreflightResult | null): string {
  if (!result) {
    return "NONE";
  }
  if (result.local_blocking_reason) {
    return `PREFLIGHT BLOCKED · ${result.local_blocking_reason}`;
  }
  return result.accepted ? "PREFLIGHT PASSED" : "PREFLIGHT FAILED";
}

function preflightMessage(result: LiveOrderPreflightResult): string {
  if (result.message.toLowerCase().includes("no order was placed")) {
    return result.message;
  }
  return `${result.message} No order was placed.`;
}

function orderSummary(order: LiveOrderRecord): string {
  const price = order.price ? ` @ ${order.price}` : "";
  const response = order.response_type ? ` · response ${order.response_type}` : "";
  const expire = order.expire_reason ? ` · expire ${order.expire_reason}` : "";
  return `${order.side} ${order.order_type} ${order.symbol} qty ${order.quantity}${price} · ${order.status.toUpperCase()} · filled ${order.executed_qty}${response}${expire}`;
}

function isTerminalOrder(order: LiveOrderRecord): boolean {
  return (
    order.status === "filled" ||
    order.status === "canceled" ||
    order.status === "rejected" ||
    order.status === "expired" ||
    order.status === "expired_in_match"
  );
}

function unique<T>(items: T[]): T[] {
  return Array.from(new Set(items));
}

function defaultRiskProfile(status: LiveStatusSnapshot | null): LiveRiskProfile {
  return {
    configured: true,
    profile_name: status?.environment === "mainnet" ? "mainnet-canary-conservative" : "testnet-conservative",
    limits: {
      max_notional_per_order: "50",
      max_open_notional_active_symbol: "50",
      max_leverage: "3",
      max_orders_per_session: 5,
      max_fills_per_session: 10,
      max_consecutive_rejections: 2,
      max_daily_realized_loss: "25"
    },
    updated_at: Date.now()
  };
}
