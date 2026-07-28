import { invoke } from "@tauri-apps/api/core";
import type {
  AiUsageToday,
  CharacterState,
  ClaudeConnectionPreview,
  ClaudeUsageToday,
  CodexAccountPreview,
  DailySummary
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
  lastReceivedAt: null,
  status: "disconnected",
  error: null
};

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
    return {
      summary: previewSummary,
      character: previewCharacter,
      aiUsage: previewAiUsage,
      claudeUsage: previewClaudeUsage
    };
  }

  const [summary, character, aiUsage, claudeUsage] = await Promise.all([
    invoke<DailySummary>("get_daily_summary"),
    invoke<CharacterState>("get_character_state"),
    invoke<AiUsageToday>("get_ai_usage_today"),
    invoke<ClaudeUsageToday>("get_claude_usage_today")
  ]);

  return { summary, character, aiUsage, claudeUsage };
}
