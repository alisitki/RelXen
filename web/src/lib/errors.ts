import { ApiClientError } from "../api/client";

export function toErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiClientError) {
    if (error.kind === "history") {
      return "History rebuild failed. Runtime kept the last valid state.";
    }
    if (error.kind === "internal") {
      return fallback;
    }
    if (error.message.trim().length > 0) {
      return error.message;
    }
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }
  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }
  return fallback;
}
