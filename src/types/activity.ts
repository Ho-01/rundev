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
