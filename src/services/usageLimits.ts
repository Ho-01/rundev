import { invoke } from "@tauri-apps/api/core";

export type UsageWindow = {
  label: string;
  usedPercent: number;
  resetsAt: number | null;
  windowMinutes: number;
};

export type ProviderLimits = {
  provider: "codex" | "claude" | "cursor";
  status: "disconnected" | "syncing" | "connected" | "unavailable" | "error";
  windows: UsageWindow[];
  lastSyncedAt: string | null;
  error: string | null;
};

export async function getAccountUsageLimits(): Promise<ProviderLimits[]> {
  if (!("__TAURI_INTERNALS__" in window)) return (["codex", "claude", "cursor"] as const).map((provider) => ({
    provider, status: "disconnected", windows: [], lastSyncedAt: null, error: null
  }));
  return invoke<ProviderLimits[]>("get_account_usage_limits");
}
