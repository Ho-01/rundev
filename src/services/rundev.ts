import type { SystemStats } from "../types/system";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import type {
  ActivityHistoryDay,
  AiUsageToday,
  CharacterState,
  ClaudeConnectionPreview,
  ClaudeUsageToday,
  CodexAccountPreview,
  CursorAccountPreview,
  CursorUsage,
  DailySummary,
  FocusActivityToday,
  FocusActivityUpdate,
  KeyboardActivityToday,
  RunnerId,
  RunnerSelection,
  WhipStats
} from "../types/activity";

const WHIP_COOLDOWN_MS = 100;

let previewWhipCount = 0;

function previewWhipStats(): WhipStats {
  return {
    localDate: new Date().toLocaleDateString("en-CA"),
    whipCount: previewWhipCount
  };
}

const previewSummary: DailySummary = {
  date: new Date().toISOString().slice(0, 10),
  activeSeconds: 0,
  xpEarned: 0,
  aiEvents: 0
};

const previewFocus: FocusActivityToday = {
  lastAppName: null,
  apps: []
};

function previewActivityHistory(): ActivityHistoryDay[] {
  const today = new Date();
  return Array.from({ length: 140 }, (_, index) => {
    const date = new Date(today);
    date.setDate(today.getDate() - (139 - index));
    return {
      date: date.toLocaleDateString("en-CA"),
      activeSeconds: 0,
      intensity: 0 as const
    };
  });
}

const previewCharacter: CharacterState = {
  level: 1,
  totalXp: 0,
  currentForm: "sprout",
  xpIntoLevel: 0,
  xpForNextLevel: 100
};

const previewAiUsage: AiUsageToday = {
  provider: "codex",
  totalTokens: null,
  weekTokens: null,
  source: null,
  lastSyncedAt: null,
  status: "disconnected",
  error: null,
  accountLabel: null,
  environment: null,
  latestAvailableDate: null,
  latestAvailableTokens: null
};

const previewClaudeUsage: ClaudeUsageToday = {
  provider: "claude",
  totalTokens: 0,
  weekTokens: 0,
  inputTokens: 0,
  outputTokens: 0,
  cachedTokens: 0,
  cacheWriteTokens: 0,
  sessionCount: 0,
  lastReceivedAt: null,
  status: "disconnected",
  error: null
};

const previewCursorUsage: CursorUsage = {
  provider: "cursor",
  status: "disconnected",
  accountLabel: null,
  usedMicrousd: null,
  limitMicrousd: null,
  remainingMicrousd: null,
  usedRequests: null,
  limitRequests: null,
  remainingRequests: null,
  todayRequests: null,
  autoPercent: null,
  apiPercent: null,
  todayMicrousd: null,
  totalTokens: null,
  weekTokens: null,
  cycleEndsAt: null,
  lastSyncedAt: null,
  errorCode: null
};

const previewKeyboard: KeyboardActivityToday = {
  localDate: new Date().toISOString().slice(0, 10),
  pressCount: 0,
  rewardedMilestones: 0,
  xpEarned: 0,
  nextRewardAt: 2_000,
  status: "active",
  permissionRequired: false
};

function previewRunner(): RunnerSelection {
  const requested = new URLSearchParams(window.location.search).get("runner");
  const supported: RunnerId[] = [
    "coding-cat",
    "coding-fish",
    "coding-orange-cat",
    "coding-shrimp",
    "coding-vtuber"
  ];
  return {
    runnerId: supported.includes(requested as RunnerId)
      ? (requested as RunnerId)
      : "coding-cat"
  };
}

function getPreviewDashboard() {
  const scenario = new URLSearchParams(window.location.search).get("preview");
  if (scenario === "active") {
    return {
      summary: { ...previewSummary, activeSeconds: 5_460, xpEarned: 84, aiEvents: 27 },
      focus: {
        lastAppName: "VS Code",
        apps: [
          { appName: "VS Code", activeSeconds: 3_720 },
          { appName: "Windows Terminal", activeSeconds: 1_260 },
          { appName: "Cursor", activeSeconds: 480 }
        ]
      },
      activityHistory: previewActivityHistory().map((day, index) =>
        index > 126 && index % 3 !== 0
          ? {
              ...day,
              activeSeconds: ((index % 4) + 1) * 1_800,
              intensity: ((index % 4) + 1) as 1 | 2 | 3 | 4
            }
          : day
      ),
      character: {
        ...previewCharacter,
        level: 4,
        totalXp: 372,
        currentForm: "focused",
        xpIntoLevel: 72
      },
      aiUsage: {
        ...previewAiUsage,
        totalTokens: 128_420,
        source: "codex-account",
        lastSyncedAt: "2026-07-28T16:49:00+09:00",
        status: "connected" as const,
        accountLabel: "Codex 계정"
      },
      claudeUsage: {
        ...previewClaudeUsage,
        totalTokens: 86_310,
        inputTokens: 31_200,
        outputTokens: 18_110,
        cachedTokens: 37_000,
        sessionCount: 3,
        lastReceivedAt: "2026-07-28T16:51:00+09:00",
        status: "connected" as const
      },
      cursorUsage: {
        ...previewCursorUsage,
        status: "connected" as const,
        accountLabel: "p***@example.com",
        usedMicrousd: 31_200_000,
        limitMicrousd: 70_000_000,
        remainingMicrousd: 38_800_000,
        autoPercent: 36.7,
        apiPercent: 12.4,
        todayMicrousd: 2_850_000,
        totalTokens: 241_820,
        cycleEndsAt: "2026-08-04T00:00:00Z",
        lastSyncedAt: "2026-07-29T16:51:00+09:00"
      },
      keyboard: {
        ...previewKeyboard,
        pressCount: 8_421,
        rewardedMilestones: 4,
        xpEarned: 40,
        nextRewardAt: 10_000
      },
      runner: previewRunner()
    };
  }
  if (scenario === "connected") {
    return {
      summary: previewSummary,
      focus: previewFocus,
      activityHistory: previewActivityHistory(),
      character: previewCharacter,
      aiUsage: {
        ...previewAiUsage,
        status: "syncing" as const,
        accountLabel: "Codex 계정"
      },
      claudeUsage: {
        ...previewClaudeUsage,
        status: "waiting" as const
      },
      cursorUsage: previewCursorUsage,
      keyboard: previewKeyboard,
      runner: previewRunner()
    };
  }
  return {
    summary: previewSummary,
    focus: previewFocus,
    activityHistory: previewActivityHistory(),
    character: previewCharacter,
    aiUsage: previewAiUsage,
    claudeUsage: previewClaudeUsage,
    cursorUsage: previewCursorUsage,
    keyboard: previewKeyboard,
    runner: previewRunner()
  };
}

export async function setRunnerSelection(runnerId: RunnerId) {
  if (!isTauri()) return;
  await invoke("set_runner_selection", { runnerId });
}

export async function openKeyboardPermissionSettings() {
  if (!isTauri()) return;
  await invoke("open_keyboard_permission_settings");
}

export async function resetKeyboardPermissionAndRelaunch() {
  if (!isTauri()) return;
  await invoke("reset_keyboard_permission");
  await relaunch();
}

export async function openDiagnosticsFolder() {
  if (!isTauri()) return;
  await invoke("open_diagnostics_folder");
}

export async function subscribeKeyboardActivity(
  onActivity: (activity: KeyboardActivityToday) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<KeyboardActivityToday>("keyboard-activity-updated", (event) => {
    onActivity(event.payload);
  });
}

export async function subscribeFocusActivity(
  onActivity: (activity: FocusActivityUpdate) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<FocusActivityUpdate>("focus-activity-updated", (event) => {
    onActivity(event.payload);
  });
}

export async function previewCodexAccount() {
  if (!isTauri()) {
    return {
      accountLabel: "preview@example.com",
      authType: "ChatGPT",
      planType: "plus",
      environment: "기본 Codex 환경"
    } satisfies CodexAccountPreview;
  }
  return invoke<CodexAccountPreview>("preview_codex_account");
}

export async function connectCodexAccount() {
  if (!isTauri()) return;
  await invoke("connect_codex_account");
}

export async function disconnectCodexAccount() {
  if (!isTauri()) return;
  await invoke("set_codex_usage_enabled", { enabled: false });
}

export async function grantCursorUsageConsent() {
  if (!isTauri()) return;
  await invoke("grant_cursor_usage_consent");
}

export async function previewCursorAccount() {
  if (!isTauri()) {
    return {
      accountLabel: "p***@example.com",
      planType: "pro"
    } satisfies CursorAccountPreview;
  }
  return invoke<CursorAccountPreview>("preview_cursor_account");
}

export async function connectCursorAccount() {
  if (!isTauri()) return;
  await invoke("connect_cursor_account");
}

export async function disconnectCursorAccount(revokeConsent = false) {
  if (!isTauri()) return;
  await invoke("disconnect_cursor_account", { revokeConsent });
}

export async function refreshCursorUsage() {
  if (!isTauri()) return;
  await invoke("refresh_cursor_usage");
}

export async function subscribeUsageRefreshed(
  onRefresh: () => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen("usage-refreshed", onRefresh);
}

export async function previewClaudeConnection() {
  if (!isTauri()) {
    return {
      settingsPath: "~/.claude/settings.json",
      hasConflicts: false
    } satisfies ClaudeConnectionPreview;
  }
  return invoke<ClaudeConnectionPreview>("preview_claude_connection");
}

export async function connectClaude() {
  if (!isTauri()) return;
  await invoke("connect_claude");
}

export async function disconnectClaude() {
  if (!isTauri()) return;
  await invoke("disconnect_claude");
}

export async function getSystemStats() {
  if (!isTauri()) {
    return {
      cpuPercent: 12,
      logicalCpuCount: 12,
      memoryPercent: 48,
      memoryTotalBytes: 34_359_738_368,
      memoryUsedBytes: 16_492_674_416,
      memoryAvailableBytes: 17_867_063_952,
      batteryPercent: 76,
      batteryState: "discharging" as const,
      temperatureCelsius: 54,
      temperatureMaxCelsius: 67,
      diskPercent: 61,
      diskTotalBytes: 1_000_204_886_016,
      diskUsedBytes: 610_124_980_470,
      diskAvailableBytes: 390_079_905_546,
      networkDownBps: 120_000,
      networkUpBps: 18_000,
      sequence: 1
    };
  }
  return invoke<SystemStats>("get_system_stats");
}

export async function setSystemPanelExpanded(expanded: boolean) {
  if (!isTauri()) return;
  await invoke("set_system_panel_expanded", { expanded });
}

export async function subscribeSystemStats(
  onStats: (stats: SystemStats) => void
): Promise<UnlistenFn> {
  if (!isTauri()) {
    onStats(await getSystemStats());
    return () => {};
  }

  const unlisten = await listen<SystemStats>(
    "system-stats-updated",
    (event) => {
      onStats(event.payload);
    }
  );
  try {
    onStats(await getSystemStats());
  } catch {
    // Snapshot can fail during early startup; events will fill in.
  }
  return unlisten;
}

export async function getWhipStats() {
  if (!isTauri()) {
    return previewWhipStats();
  }
  return invoke<WhipStats>("get_whip_stats");
}

export async function recordWhip() {
  if (!isTauri()) {
    previewWhipCount += 1;
    return previewWhipStats();
  }
  return invoke<WhipStats>("record_whip");
}

export { WHIP_COOLDOWN_MS };

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export async function getDashboard() {
  if (!isTauri()) {
    return getPreviewDashboard();
  }

  const [summary, focus, activityHistory, character, aiUsage, claudeUsage, cursorUsage, keyboard, runner] =
    await Promise.all([
    invoke<DailySummary>("get_daily_summary"),
    invoke<FocusActivityToday>("get_focus_activity_today"),
    invoke<ActivityHistoryDay[]>("get_activity_history"),
    invoke<CharacterState>("get_character_state"),
    invoke<AiUsageToday>("get_ai_usage_today"),
    invoke<ClaudeUsageToday>("get_claude_usage_today"),
    invoke<CursorUsage>("get_cursor_usage"),
    invoke<KeyboardActivityToday>("get_keyboard_activity_today"),
    invoke<RunnerSelection>("get_runner_selection")
  ]);

  return {
    summary,
    focus,
    activityHistory,
    character,
    aiUsage,
    claudeUsage,
    cursorUsage,
    keyboard,
    runner
  };
}
