import { invoke } from "@tauri-apps/api/core";
import type {
  AiUsageToday,
  AiActivityStatus,
  CharacterState,
  ClaudeConnectionPreview,
  ClaudeUsageToday,
  CodexAccountPreview,
  DailySummary,
  RunnerId,
  RunnerSelection
} from "../types/activity";

const previewSummary: DailySummary = {
  date: new Date().toISOString().slice(0, 10),
  activeSeconds: 0,
  xpEarned: 0,
  aiEvents: 0
};

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
  inputTokens: 0,
  outputTokens: 0,
  cachedTokens: 0,
  cacheWriteTokens: 0,
  sessionCount: 0,
  lastReceivedAt: null,
  status: "disconnected",
  error: null
};

const previewAiActivity: AiActivityStatus = {
  activeProviderCount: 0,
  codexActive: false,
  claudeActive: false,
  claudeActiveSessions: 0
};

function previewRunner(): RunnerSelection {
  const requested = new URLSearchParams(window.location.search).get("runner");
  const supported: RunnerId[] = [
    "coding-cat",
    "coding-fish",
    "coding-orange-cat",
    "coding-white-cat",
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
      aiActivity: {
        activeProviderCount: 2,
        codexActive: true,
        claudeActive: true,
        claudeActiveSessions: 2
      },
      runner: previewRunner()
    };
  }
  if (scenario === "connected") {
    return {
      summary: previewSummary,
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
      aiActivity: previewAiActivity,
      runner: previewRunner()
    };
  }
  return {
    summary: previewSummary,
    character: previewCharacter,
    aiUsage: previewAiUsage,
    claudeUsage: previewClaudeUsage,
    aiActivity: previewAiActivity,
    runner: previewRunner()
  };
}

export async function setRunnerSelection(runnerId: RunnerId) {
  if (!isTauri()) return;
  await invoke("set_runner_selection", { runnerId });
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

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export async function getDashboard() {
  if (!isTauri()) {
    return getPreviewDashboard();
  }

  const [summary, character, aiUsage, claudeUsage, aiActivity, runner] = await Promise.all([
    invoke<DailySummary>("get_daily_summary"),
    invoke<CharacterState>("get_character_state"),
    invoke<AiUsageToday>("get_ai_usage_today"),
    invoke<ClaudeUsageToday>("get_claude_usage_today"),
    invoke<AiActivityStatus>("get_ai_activity_status"),
    invoke<RunnerSelection>("get_runner_selection")
  ]);

  return { summary, character, aiUsage, claudeUsage, aiActivity, runner };
}
