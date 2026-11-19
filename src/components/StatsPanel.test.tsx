import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { StatsPanel } from "./StatsPanel";
import { getActivityStats } from "../services/rundev";
import type { ActivityStats } from "../types/activity";
vi.mock("../services/rundev", () => ({ getActivityStats: vi.fn() }));
afterEach(() => { cleanup(); vi.resetAllMocks(); });
const stats: ActivityStats = { period: "history", activeSeconds: 3600, xpEarned: 50, keyboardPresses: 1000,
  hourly: [], xpSources: [{id:"focus",amount:50}], apps: [{appName:"VS Code",activeSeconds:3600}] };
const panel = () => render(<StatsPanel dailyStats={null} weeklyStats={null} activityHistory={[
  {date:"2026-11-18",activeSeconds:3600,intensity:3}
]} activeHistoryDays={1} onClose={() => {}} />);
it("loads the 20-week totals, XP sources and apps only on demand and reuses them across tabs", async () => {
  vi.mocked(getActivityStats).mockResolvedValue(stats);
  panel();
  expect(getActivityStats).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("tab",{name:"20주"}));
  expect(await screen.findByText("최근 20주 사용한 개발 도구")).toBeInTheDocument();
  expect(screen.getByText("VS Code")).toBeInTheDocument();
  expect(screen.getByRole("img",{name:"최근 20주 XP 구성. 집중 50 XP"})).toBeInTheDocument();
  expect(screen.getByRole("img",{name:"최근 20주 중 1일 개발 활동"})).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab",{name:"이번 주"}));
  fireEvent.click(screen.getByRole("tab",{name:"20주"}));
  expect(getActivityStats).toHaveBeenCalledExactlyOnceWith("history");
});
it("shows a retry instead of zero totals when the history request fails", async () => {
  vi.mocked(getActivityStats).mockRejectedValueOnce(new Error("offline")).mockResolvedValueOnce(stats);
  panel();
  fireEvent.click(screen.getByRole("tab",{name:"20주"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("20주 통계를 불러오지 못했습니다.");
  fireEvent.click(screen.getByRole("button",{name:"다시 시도"}));
  await waitFor(()=>expect(screen.getByText("VS Code")).toBeInTheDocument());
});
