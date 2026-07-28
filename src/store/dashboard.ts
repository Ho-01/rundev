import { create } from "zustand";
import {
  connectClaude,
  connectCodexAccount,
  disconnectClaude,
  disconnectCodexAccount,
  getDashboard,
  setRunnerSelection
} from "../services/rundev";
import type {
  AiUsageToday,
  AiActivityStatus,
  CharacterState,
  ClaudeUsageToday,
  DailySummary,
  RunnerId,
  RunnerSelection
} from "../types/activity";

type DashboardStore = {
  summary: DailySummary | null;
  character: CharacterState | null;
  aiUsage: AiUsageToday | null;
  claudeUsage: ClaudeUsageToday | null;
  aiActivity: AiActivityStatus | null;
  runner: RunnerSelection | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  connectCodex: () => Promise<void>;
  disconnectCodex: () => Promise<void>;
  connectClaude: () => Promise<void>;
  disconnectClaude: () => Promise<void>;
  selectRunner: (runnerId: RunnerId) => Promise<void>;
};

export const useDashboardStore = create<DashboardStore>((set) => ({
  summary: null,
  character: null,
  aiUsage: null,
  claudeUsage: null,
  aiActivity: null,
  runner: null,
  loading: false,
  error: null,
  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const data = await getDashboard();
      set({ ...data, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : String(error)
      });
    }
  },
  connectCodex: async () => {
    set({ loading: true, error: null });
    try {
      await connectCodexAccount();
      const data = await getDashboard();
      set({ ...data, loading: false });
    } catch (error) {
      const data = await getDashboard();
      set({
        ...data,
        loading: false,
        error: error instanceof Error ? error.message : String(error)
      });
    }
  },
  disconnectCodex: async () => {
    set({ loading: true, error: null });
    await disconnectCodexAccount();
    const data = await getDashboard();
    set({ ...data, loading: false });
  },
  connectClaude: async () => {
    set({ loading: true, error: null });
    try {
      await connectClaude();
      const data = await getDashboard();
      set({ ...data, loading: false });
    } catch (error) {
      const data = await getDashboard();
      set({
        ...data,
        loading: false,
        error: error instanceof Error ? error.message : String(error)
      });
    }
  },
  disconnectClaude: async () => {
    set({ loading: true, error: null });
    await disconnectClaude();
    const data = await getDashboard();
    set({ ...data, loading: false });
  },
  selectRunner: async (runnerId) => {
    set({ loading: true, error: null });
    await setRunnerSelection(runnerId);
    const data = await getDashboard();
    set({ ...data, loading: false });
  }
}));
