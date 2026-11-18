import { useEffect, useState } from "react";
import { getSalarySnapshot, subscribeSalaryChanges, type SalarySnapshot } from "../services/salary";
import { useWindowVisibility } from "./useWindowVisibility";

export function useSalarySnapshot(enabled = true) {
  const [snapshot, setSnapshot] = useState<SalarySnapshot | null>(null);
  const visible = useWindowVisibility();
  useEffect(() => {
    if (!enabled || !visible) return;
    let active = true;
    let pending = false;
    let unlisten = () => {};
    const refresh = async () => {
      if (pending || document.hidden) return;
      pending = true;
      try {
        const value = await getSalarySnapshot();
        if (active) setSnapshot(value);
      } catch { if (active) setSnapshot(null); }
      finally { pending = false; }
    };
    void refresh();
    void subscribeSalaryChanges(() => void refresh()).then((off) => {
      if (active) unlisten = off; else off();
    }).catch(() => {});
    const timer = window.setInterval(() => void refresh(), 1000);
    document.addEventListener("visibilitychange", refresh);
    window.addEventListener("focus", refresh);
    return () => { active = false; unlisten(); window.clearInterval(timer);
      document.removeEventListener("visibilitychange", refresh); window.removeEventListener("focus", refresh); };
  }, [enabled, visible]);
  return snapshot;
}
