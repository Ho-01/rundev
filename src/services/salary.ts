import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type SalarySettings = {
  monthlyWon: number;
  payday: number;
  startMinute: number;
  endMinute: number;
  lunchStartMinute: number;
  lunchEndMinute: number;
  dailyMinutes: number;
  weekdays: number[];
};

export type SalarySnapshot = {
  cycleStart: string;
  nextPayday: string;
  cycleWon: number;
  todayWon: number;
  wonPerSecond: number;
  working: boolean;
  workStatus: "working" | "lunch" | "beforeWork" | "finished" | "dayOff";
  secondsUntilEnd: number | null;
  workdays: number;
};

export const salaryAvailable = () => "__TAURI_INTERNALS__" in window;
export const subscribeSalaryChanges = (onChange: () => void) => salaryAvailable()
  ? listen("salary-settings-changed", onChange) : Promise.resolve(() => {});
export type SalaryAnchor = { x: number; y: number; characterSize: number };
export const subscribeSalaryAnchor = (onChange: (anchor: SalaryAnchor) => void) => salaryAvailable()
  ? listen<SalaryAnchor>("character-salary-anchor", (event) => onChange(event.payload))
  : Promise.resolve(() => {});
export const getSalarySettings = () => salaryAvailable()
  ? invoke<SalarySettings | null>("get_salary_settings") : Promise.resolve(null);
export const getSalarySnapshot = () => salaryAvailable()
  ? invoke<SalarySnapshot | null>("get_salary_snapshot") : Promise.resolve(null);
export async function saveSalarySettings(settings: SalarySettings | null) {
  if (!salaryAvailable()) throw new Error("급여 설정은 데스크톱 앱에서 저장할 수 있습니다.");
  await invoke("save_salary_settings", { settings });
}
