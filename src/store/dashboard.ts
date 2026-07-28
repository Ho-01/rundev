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
  CharacterState,
  ClaudeUsageToday,
  DailySummary,
  FocusActivityToday,
  FocusActivityUpdate,
  KeyboardActivityToday,
  RunnerId,
  RunnerSelection
} from "../types/activity";

type DashboardStore = {
  summary: DailySummary | null;
  focus: FocusActivityToday | null;
  character: CharacterState | null;
  aiUsage: AiUsageToday | null;
  claudeUsage: ClaudeUsageToday | null;
  keyboard: KeyboardActivityToday | null;
  runner: RunnerSelection | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  connectCodex: () => Promise<void>;
  disconnectCodex: () => Promise<void>;
  connectClaude: () => Promise<void>;
  disconnectClaude: () => Promise<void>;
  selectRunner: (runnerId: RunnerId) => Promise<void>;
  setKeyboardActivity: (keyboard: KeyboardActivityToday) => void;
  setFocusActivity: (activity: FocusActivityUpdate) => void;
};

export const useDashboardStore = create<DashboardStore>((set) => ({
  summary: null,
  focus: null,
  character: null,
  aiUsage: null,
  claudeUsage: null,
  keyboard: null,
  runner: null,
  setKeyboardActivity: (keyboard) => set({ keyboard }),
  setFocusActivity: (activity) =>
    set((state) => ({
      summary: state.summary
        ? { ...state.summary, activeSeconds: activity.activeSeconds }
        : state.summary
    })),
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
