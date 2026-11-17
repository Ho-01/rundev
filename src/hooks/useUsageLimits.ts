import { useEffect, useState } from "react";
import { getAccountUsageLimits, type ProviderLimits } from "../services/usageLimits";

export function useUsageLimits() {
  const [limits, setLimits] = useState<ProviderLimits[]>([]);
  useEffect(() => {
    let disposed = false;
    let pending = false;
    async function refresh() {
      if (pending) return;
      pending = true;
      try {
        const values = await getAccountUsageLimits();
        if (!disposed) setLimits(values);
      } catch {
        if (!disposed) setLimits((current) => (current.length ? current : (["codex", "claude", "cursor"] as const).map((provider) => ({
          provider, windows: [], lastSyncedAt: null
        }))).map((view) => ({
          ...view, status: "error", error: "한도 정보를 읽지 못했습니다."
        })));
      } finally { pending = false; }
    }
    void refresh();
    // Rust throttles provider requests to 60 seconds; this only reads its cache.
    const timer = window.setInterval(() => void refresh(), 5_000);
    return () => { disposed = true; window.clearInterval(timer); };
  }, []);
  return limits;
}
