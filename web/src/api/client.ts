import type {
  BootstrapPayload,
  CreateLiveCredentialRequest,
  LiveCredentialSummary,
  LiveCredentialValidationResult,
  LiveCancelResult,
  LiveExecutionRequest,
  LiveExecutionResult,
  LiveFillRecord,
  LiveFlattenResult,
  LiveModePreference,
  LiveOrderRecord,
  LiveOrderPreflightResult,
  LiveOrderPreview,
  LiveOrderType,
  LiveRiskProfile,
  LiveStartCheck,
  LiveStatusSnapshot,
  LogEvent,
  RuntimeStatus,
  Settings,
  SignalEvent,
  Trade,
  UpdateLiveCredentialRequest
} from "../types";

export type ApiErrorKind =
  | "validation"
  | "history"
  | "conflict"
  | "not_found"
  | "secure_store_unavailable"
  | "live"
  | "exchange"
  | "internal"
  | "unknown";

export class ApiClientError extends Error {
  readonly status: number;
  readonly kind: ApiErrorKind;

  constructor(message: string, status: number, kind: ApiErrorKind = "unknown") {
    super(message);
    this.name = "ApiClientError";
    this.status = status;
    this.kind = kind;
  }
}

async function request<T>(input: string, init?: RequestInit): Promise<T> {
  const response = await fetch(input, {
    headers: {
      "Content-Type": "application/json"
    },
    ...init
  });
  if (!response.ok) {
    throw await readApiError(response);
  }
  return (await response.json()) as T;
}

export async function readErrorMessage(response: Response): Promise<string> {
  return (await readApiError(response)).message;
}

export async function readApiError(response: Response): Promise<ApiClientError> {
  const body = await response.text();
  if (!body) {
    return new ApiClientError(`${response.status} ${response.statusText}`, response.status);
  }

  try {
    const parsed = JSON.parse(body) as { error?: string; kind?: ApiErrorKind };
    if (typeof parsed.error === "string" && parsed.error.trim().length > 0) {
      return new ApiClientError(parsed.error, response.status, parsed.kind ?? "unknown");
    }
  } catch {
    // Fall through to raw body when the server returns plain text.
  }

  return new ApiClientError(body, response.status);
}

export function getBootstrap(): Promise<BootstrapPayload> {
  return request("/api/bootstrap");
}

export function getSettings(): Promise<Settings> {
  return request("/api/settings");
}

export function putSettings(settings: Settings): Promise<BootstrapPayload> {
  return request("/api/settings", {
    method: "PUT",
    body: JSON.stringify(settings)
  });
}

export function startRuntime(): Promise<RuntimeStatus> {
  return request("/api/runtime/start", { method: "POST" });
}

export function stopRuntime(): Promise<RuntimeStatus> {
  return request("/api/runtime/stop", { method: "POST" });
}

export function closeAllPaper(): Promise<BootstrapPayload> {
  return request("/api/paper/close-all", { method: "POST" });
}

export function resetPaper(): Promise<BootstrapPayload> {
  return request("/api/paper/reset", { method: "POST" });
}

export function getTrades(limit = 50): Promise<Trade[]> {
  return request(`/api/trades?limit=${limit}`);
}

export function getSignals(limit = 50): Promise<SignalEvent[]> {
  return request(`/api/signals?limit=${limit}`);
}

export function getLogs(limit = 100): Promise<LogEvent[]> {
  return request(`/api/logs?limit=${limit}`);
}

export function getLiveStatus(): Promise<LiveStatusSnapshot> {
  return request("/api/live/status");
}

export function listLiveCredentials(): Promise<LiveCredentialSummary[]> {
  return request("/api/live/credentials");
}

export function createLiveCredential(payload: CreateLiveCredentialRequest): Promise<LiveCredentialSummary> {
  return request("/api/live/credentials", {
    method: "POST",
    body: JSON.stringify(payload)
  });
}

export function updateLiveCredential(
  credentialId: string,
  payload: UpdateLiveCredentialRequest
): Promise<LiveCredentialSummary> {
  return request(`/api/live/credentials/${credentialId}`, {
    method: "PUT",
    body: JSON.stringify(payload)
  });
}

export function deleteLiveCredential(credentialId: string): Promise<void> {
  return requestNoBody(`/api/live/credentials/${credentialId}`, { method: "DELETE" });
}

export function selectLiveCredential(credentialId: string): Promise<LiveStatusSnapshot> {
  return request(`/api/live/credentials/${credentialId}/select`, { method: "POST" });
}

export function validateLiveCredential(credentialId: string): Promise<LiveCredentialValidationResult> {
  return request(`/api/live/credentials/${credentialId}/validate`, { method: "POST" });
}

export function refreshLiveReadiness(): Promise<LiveStatusSnapshot> {
  return request("/api/live/readiness/refresh", { method: "POST" });
}

export function armLive(): Promise<LiveStatusSnapshot> {
  return request("/api/live/arm", { method: "POST" });
}

export function disarmLive(): Promise<LiveStatusSnapshot> {
  return request("/api/live/disarm", {
    method: "POST",
    body: JSON.stringify({ reason: "operator_disarm" })
  });
}

export function liveStartCheck(): Promise<LiveStartCheck> {
  return request("/api/live/start-check", { method: "POST" });
}

export function setLiveModePreference(modePreference: LiveModePreference): Promise<LiveStatusSnapshot> {
  return request("/api/live/mode", {
    method: "POST",
    body: JSON.stringify({ mode_preference: modePreference })
  });
}

export function startLiveShadow(): Promise<LiveStatusSnapshot> {
  return request("/api/live/shadow/start", { method: "POST" });
}

export function stopLiveShadow(): Promise<LiveStatusSnapshot> {
  return request("/api/live/shadow/stop", { method: "POST" });
}

export function refreshLiveShadow(): Promise<LiveStatusSnapshot> {
  return request("/api/live/shadow/refresh", { method: "POST" });
}

export function getLiveIntentPreview(orderType: LiveOrderType = "MARKET", limitPrice?: string): Promise<LiveOrderPreview> {
  const params = new URLSearchParams({ order_type: orderType });
  if (limitPrice && limitPrice.trim().length > 0) {
    params.set("limit_price", limitPrice.trim());
  }
  return request(`/api/live/intent/preview?${params.toString()}`);
}

export function runLivePreflight(): Promise<LiveOrderPreflightResult> {
  return request("/api/live/preflight", { method: "POST" });
}

export function startLiveAuto(): Promise<LiveStatusSnapshot> {
  return request("/api/live/auto/start", {
    method: "POST",
    body: JSON.stringify({ confirm_testnet_auto: true })
  });
}

export function stopLiveAuto(): Promise<LiveStatusSnapshot> {
  return request("/api/live/auto/stop", { method: "POST" });
}

export function engageLiveKillSwitch(reason = "operator_engaged"): Promise<LiveStatusSnapshot> {
  return request("/api/live/kill-switch/engage", {
    method: "POST",
    body: JSON.stringify({ reason })
  });
}

export function releaseLiveKillSwitch(reason = "operator_released"): Promise<LiveStatusSnapshot> {
  return request("/api/live/kill-switch/release", {
    method: "POST",
    body: JSON.stringify({ reason })
  });
}

export function configureLiveRiskProfile(payload: LiveRiskProfile): Promise<LiveStatusSnapshot> {
  return request("/api/live/risk-profile", {
    method: "PUT",
    body: JSON.stringify(payload)
  });
}

export function getLivePreflights(limit = 50): Promise<LiveOrderPreflightResult[]> {
  return request(`/api/live/preflights?limit=${limit}`);
}

export function executeLivePreview(payload: LiveExecutionRequest): Promise<LiveExecutionResult> {
  return request("/api/live/execute", {
    method: "POST",
    body: JSON.stringify(payload)
  });
}

export function cancelLiveOrder(orderRef: string, confirmTestnet = true): Promise<LiveCancelResult> {
  return request(`/api/live/orders/${encodeURIComponent(orderRef)}/cancel`, {
    method: "POST",
    body: JSON.stringify({ order_ref: orderRef, confirm_testnet: confirmTestnet })
  });
}

export function cancelLiveOrderWithPayload(
  orderRef: string,
  payload: { confirm_testnet: boolean; confirm_mainnet_canary?: boolean; confirmation_text?: string | null }
): Promise<LiveCancelResult> {
  return request(`/api/live/orders/${encodeURIComponent(orderRef)}/cancel`, {
    method: "POST",
    body: JSON.stringify({ order_ref: orderRef, ...payload })
  });
}

export function cancelAllLiveOrders(confirmTestnet = true): Promise<LiveCancelResult[]> {
  return request("/api/live/cancel-all", {
    method: "POST",
    body: JSON.stringify({ confirm_testnet: confirmTestnet })
  });
}

export function cancelAllLiveOrdersWithPayload(payload: {
  confirm_testnet: boolean;
  confirm_mainnet_canary?: boolean;
  confirmation_text?: string | null;
}): Promise<LiveCancelResult[]> {
  return request("/api/live/cancel-all", {
    method: "POST",
    body: JSON.stringify(payload)
  });
}

export function flattenLivePosition(confirmTestnet = true): Promise<LiveFlattenResult> {
  return request("/api/live/flatten", {
    method: "POST",
    body: JSON.stringify({ confirm_testnet: confirmTestnet })
  });
}

export function flattenLivePositionWithPayload(payload: {
  confirm_testnet: boolean;
  confirm_mainnet_canary?: boolean;
  confirmation_text?: string | null;
}): Promise<LiveFlattenResult> {
  return request("/api/live/flatten", {
    method: "POST",
    body: JSON.stringify(payload)
  });
}

export function getLiveOrders(limit = 50): Promise<LiveOrderRecord[]> {
  return request(`/api/live/orders?limit=${limit}`);
}

export function getLiveFills(limit = 100): Promise<LiveFillRecord[]> {
  return request(`/api/live/fills?limit=${limit}`);
}

async function requestNoBody(input: string, init?: RequestInit): Promise<void> {
  const response = await fetch(input, {
    headers: {
      "Content-Type": "application/json"
    },
    ...init
  });
  if (!response.ok) {
    throw await readApiError(response);
  }
}
