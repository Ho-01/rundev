import { create } from "zustand";
import {
  connectCodexAccount,
  disconnectCodexAccount,
  getDashboard
} from "../services/rundev";
import type { AiUsageToday, CharacterState, DailySummary } from "../types/activity";

type DashboardStore = {
  summary: DailySummary | null;
  character: CharacterState | null;
  aiUsage: AiUsageToday | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  connectCodex: () => Promise<void>;
  disconnectCodex: () => Promise<void>;
};

export const useDashboardStore = create<DashboardStore>((set) => ({
  summary: null,
  character: null,
  aiUsage: null,
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
  }
}));
