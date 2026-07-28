export type DailySummary = {
  date: string;
  activeSeconds: number;
  xpEarned: number;
  aiEvents: number;
};

export type FocusAppUsage = {
  appName: string;
  activeSeconds: number;
};

export type FocusActivityToday = {
  lastAppName: string | null;
  apps: FocusAppUsage[];
};

export type ActivityHistoryDay = {
  date: string;
  activeSeconds: number;
  intensity: 0 | 1 | 2 | 3 | 4;
};

export type CharacterState = {
  level: number;
  totalXp: number;
  currentForm: string;
  xpIntoLevel: number;
  xpForNextLevel: number;
};

export type AiUsageToday = {
  provider: string;
  totalTokens: number | null;
  source: string | null;
  lastSyncedAt: string | null;
  status: "disconnected" | "syncing" | "connected" | "delayed" | "error";
  error: string | null;
  accountLabel: string | null;
  environment: string | null;
  latestAvailableDate: string | null;
  latestAvailableTokens: number | null;
};

export type CodexAccountPreview = {
  accountLabel: string;
  authType: string;
  planType: string | null;
  environment: string;
};

export type ClaudeConnectionPreview = {
  settingsPath: string;
  hasConflicts: boolean;
};

export type ClaudeUsageToday = {
  provider: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cachedTokens: number;
  cacheWriteTokens: number;
  sessionCount: number;
  lastReceivedAt: string | null;
  status: "disconnected" | "waiting" | "connected" | "error";
  error: string | null;
};

export type KeyboardActivityToday = {
  localDate: string;
  pressCount: number;
  rewardedMilestones: number;
  xpEarned: number;
  nextRewardAt: number;
  status: "starting" | "active" | "permission-required" | "error" | "unavailable";
  permissionRequired: boolean;
};

export type FocusActivityUpdate = {
  activeSeconds: number;
  focused: boolean;
};

export type RunnerId =
  | "coding-cat"
  | "coding-fish"
  | "coding-orange-cat"
  | "coding-shrimp"
  | "coding-vtuber";

export type RunnerSelection = {
  runnerId: RunnerId;
};
