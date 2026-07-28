import { create } from "zustand";
import { getDashboard } from "../services/rundev";
import type { CharacterState, DailySummary } from "../types/activity";

type DashboardStore = {
  summary: DailySummary | null;
  character: CharacterState | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
};

export const useDashboardStore = create<DashboardStore>((set) => ({
  summary: null,
  character: null,
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
  }
}));
