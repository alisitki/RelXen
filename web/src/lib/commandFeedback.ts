import { toErrorMessage } from "./errors";
import type { ToastKind } from "../types";

type AddToast = (message: string, kind?: ToastKind) => void;

export type CommandKind =
  | "settings_apply"
  | "runtime_start"
  | "runtime_stop"
  | "paper_close_all"
  | "paper_reset"
  | "bootstrap_reload"
  | "live_credential_save"
  | "live_credential_delete"
  | "live_credential_validate"
  | "live_readiness_refresh"
  | "live_arm"
  | "live_disarm"
  | "live_start_check"
  | "live_mode"
  | "live_shadow_start"
  | "live_shadow_stop"
  | "live_shadow_refresh"
  | "live_intent_preview"
  | "live_preflight"
  | "live_execute"
  | "live_cancel"
  | "live_cancel_all"
  | "live_flatten"
  | "live_kill_switch_engage"
  | "live_kill_switch_release"
  | "live_auto_start"
  | "live_auto_stop"
  | "live_risk_profile";

export function notifyCommandSuccess(addToast: AddToast, command: CommandKind): void {
  addToast(successMessage(command), "info");
}

export function notifyCommandError(addToast: AddToast, command: CommandKind, error: unknown): void {
  addToast(toErrorMessage(error, failureMessage(command)), "error");
}

function successMessage(command: CommandKind): string {
  switch (command) {
    case "settings_apply":
      return "Settings applied.";
    case "runtime_start":
      return "Runtime started.";
    case "runtime_stop":
      return "Runtime stopped.";
    case "paper_close_all":
      return "Paper position closed.";
    case "paper_reset":
      return "Paper account reset.";
    case "bootstrap_reload":
      return "Snapshot reloaded after resync.";
    case "live_credential_save":
      return "Live credential metadata saved.";
    case "live_credential_delete":
      return "Live credential deleted.";
    case "live_credential_validate":
      return "Live credential validated.";
    case "live_readiness_refresh":
      return "Live readiness refreshed.";
    case "live_arm":
      return "Live read-only mode armed.";
    case "live_disarm":
      return "Live mode disarmed.";
    case "live_start_check":
      return "Live start check completed.";
    case "live_mode":
      return "Execution preference updated.";
    case "live_shadow_start":
      return "Live shadow sync started.";
    case "live_shadow_stop":
      return "Live shadow sync stopped.";
    case "live_shadow_refresh":
      return "Live shadow state refreshed.";
    case "live_intent_preview":
      return "Live order intent preview built.";
    case "live_preflight":
      return "Live preflight completed.";
    case "live_execute":
      return "TESTNET order submission accepted.";
    case "live_cancel":
      return "TESTNET cancel submitted.";
    case "live_cancel_all":
      return "TESTNET cancel-all submitted.";
    case "live_flatten":
      return "TESTNET flatten submitted.";
    case "live_kill_switch_engage":
      return "Live kill switch engaged.";
    case "live_kill_switch_release":
      return "Live kill switch released.";
    case "live_auto_start":
      return "TESTNET auto executor started.";
    case "live_auto_stop":
      return "TESTNET auto executor stopped.";
    case "live_risk_profile":
      return "Live risk profile configured.";
  }
}

function failureMessage(command: CommandKind): string {
  switch (command) {
    case "settings_apply":
      return "Failed to apply settings.";
    case "runtime_start":
      return "Failed to start runtime.";
    case "runtime_stop":
      return "Failed to stop runtime.";
    case "paper_close_all":
      return "Failed to close paper position.";
    case "paper_reset":
      return "Failed to reset paper account.";
    case "bootstrap_reload":
      return "Failed to reload bootstrap snapshot.";
    case "live_credential_save":
      return "Failed to save live credential.";
    case "live_credential_delete":
      return "Failed to delete live credential.";
    case "live_credential_validate":
      return "Failed to validate live credential.";
    case "live_readiness_refresh":
      return "Failed to refresh live readiness.";
    case "live_arm":
      return "Failed to arm live mode.";
    case "live_disarm":
      return "Failed to disarm live mode.";
    case "live_start_check":
      return "Live start remains blocked.";
    case "live_mode":
      return "Failed to update execution preference.";
    case "live_shadow_start":
      return "Failed to start live shadow sync.";
    case "live_shadow_stop":
      return "Failed to stop live shadow sync.";
    case "live_shadow_refresh":
      return "Failed to refresh live shadow state.";
    case "live_intent_preview":
      return "Failed to build live order intent preview.";
    case "live_preflight":
      return "Failed to run live preflight.";
    case "live_execute":
      return "Failed to submit TESTNET order.";
    case "live_cancel":
      return "Failed to cancel TESTNET order.";
    case "live_cancel_all":
      return "Failed to cancel TESTNET open orders.";
    case "live_flatten":
      return "Failed to flatten TESTNET position.";
    case "live_kill_switch_engage":
      return "Failed to engage live kill switch.";
    case "live_kill_switch_release":
      return "Failed to release live kill switch.";
    case "live_auto_start":
      return "Failed to start TESTNET auto executor.";
    case "live_auto_stop":
      return "Failed to stop TESTNET auto executor.";
    case "live_risk_profile":
      return "Failed to configure live risk profile.";
  }
}
