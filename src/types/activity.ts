export type DailySummary = {
  date: string;
  activeSeconds: number;
  xpEarned: number;
  aiEvents: number;
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
