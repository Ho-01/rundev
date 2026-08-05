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

export type XpCouponPreview = {
  couponId: string;
  multiplier: number;
  durationMinutes: number;
  redeemBefore: string;
};

export type XpBoostStatus = {
  active: boolean;
  multiplier: number | null;
  startsAt: string | null;
  endsAt: string | null;
};

export type AiUsageToday = {
  provider: string;
  totalTokens: number | null;
  weekTokens: number | null;
  source: string | null;
  lastSyncedAt: string | null;
  status: "disconnected" | "syncing" | "connected" | "delayed" | "error";
  error: string | null;
  accountLabel: string | null;
  environment: string | null;
  latestAvailableDate: string | null;
  latestAvailableTokens: number | null;
};

export type AiWeeklyXp = {
  weekStartedOn: string;
  earnedXp: number;
  maxXp: number;
  codexXp: number;
  claudeXp: number;
  cursorXp: number;
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
  weekTokens: number;
  inputTokens: number;
  outputTokens: number;
  cachedTokens: number;
  cacheWriteTokens: number;
  sessionCount: number;
  lastReceivedAt: string | null;
  status: "disconnected" | "waiting" | "connected" | "error";
  error: string | null;
};

export type CursorAccountPreview = {
  accountLabel: string;
  planType: string | null;
};

export type CursorUsage = {
  provider: "cursor";
  status:
    | "disconnected"
    | "syncing"
    | "connected"
    | "stale"
    | "rateLimited"
    | "reauthRequired"
    | "unsupportedSchema"
    | "error";
  accountLabel: string | null;
  usedMicrousd: number | null;
  limitMicrousd: number | null;
  remainingMicrousd: number | null;
  usedRequests: number | null;
  limitRequests: number | null;
  remainingRequests: number | null;
  todayRequests: number | null;
  autoPercent: number | null;
  apiPercent: number | null;
  todayMicrousd: number | null;
  totalTokens: number | null;
  weekTokens: number | null;
  cycleEndsAt: string | null;
  lastSyncedAt: string | null;
  errorCode: string | null;
};

export type KeyboardActivityToday = {
  localDate: string;
  pressCount: number;
  rewardedMilestones: number;
  xpEarned: number;
  nextRewardAt: number;
  pressesPerReward: number;
  status: "starting" | "active" | "permission-required" | "error" | "unavailable";
  permissionRequired: boolean;
};

export type TraitId = "focus-ready" | "hot-keyboard" | "reload" | "context-runner";
export type TraitProgress = {
  availablePoints: number;
  earnedPoints: number;
  spentPoints: number;
  traits: Array<{ id: TraitId; level: number; maxLevel: number; effectPercent: number }>;
};
export type ActivityStats = {
  period: "day" | "week";
  activeSeconds: number;
  xpEarned: number;
  keyboardPresses: number;
  xpSources: Array<{
    id: "focus" | "keyboard" | "ai" | "boost" | "trait" | "other";
    amount: number;
  }>;
  hourly: Array<{ date: string; hour: number; activeSeconds: number; xpEarned: number }>;
};

export type FocusActivityUpdate = {
  activeSeconds: number;
  focused: boolean;
  active: boolean;
  appName: string | null;
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

export type WhipStats = {
  localDate: string;
  whipCount: number;
};
