import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SalaryCard } from "./SalaryCard";
import { getSalarySettings, getSalarySnapshot, saveSalarySettings } from "../services/salary";

vi.mock("../services/salary", () => ({
  salaryAvailable: () => true,
  getSalarySettings: vi.fn(), getSalarySnapshot: vi.fn(), saveSalarySettings: vi.fn()
}));
const settings = { monthlyWon: 3000000, payday: 25, startMinute: 540, endMinute: 1080,
  lunchStartMinute: 720, lunchEndMinute: 780, dailyMinutes: 480, weekdays: [0,1,2,3,4] };

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getSalarySettings).mockResolvedValue(null);
  vi.mocked(saveSalarySettings).mockResolvedValue(undefined);
  vi.mocked(getSalarySnapshot).mockResolvedValue({ cycleStart: "2026-10-25", nextPayday: "2026-11-25",
    cycleWon: 123456.78, todayWon: 1234, wonPerSecond: 4.73, working: true, workStatus: "working" as const, secondsUntilEnd: 3600, workdays: 22 });
});
afterEach(cleanup);

describe("SalaryCard", () => {
  it.each([
    ["working", "근무 중", 7384, "퇴근까지 02:03:04"],
    ["lunch", "점심시간", 6 * 3600, "퇴근까지 06:00:00"],
    ["finished", "퇴근", null, null],
    ["beforeWork", "출근 전", 9 * 3600, "퇴근까지 09:00:00"],
    ["dayOff", "휴무", null, null]
  ] as const)("displays native %s state and the correct end-of-shift countdown", async (workStatus, label, secondsUntilEnd, countdown) => {
    vi.mocked(getSalarySettings).mockResolvedValue(settings);
    vi.mocked(getSalarySnapshot).mockResolvedValue({ cycleStart: "2026-10-25", nextPayday: "2026-11-25",
      cycleWon: 123456, todayWon: 1234, wonPerSecond: 4.73, working: workStatus === "working",
      workStatus, secondsUntilEnd, workdays: 22 });
    render(<SalaryCard />);
    expect(await screen.findByText(`● ${label}`)).toHaveClass(workStatus);
    if (countdown) expect(screen.getByText(countdown)).toBeInTheDocument();
    else expect(screen.queryByText(/^퇴근까지/)).not.toBeInTheDocument();
  });

  it("automatically recovers a failed initial read without losing saved settings", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(getSalarySettings).mockRejectedValueOnce("pool timed out").mockResolvedValue(settings);
      render(<SalaryCard />);
      await act(async () => {});
      expect(screen.getByRole("alert")).toHaveTextContent("자동으로 다시 시도");
      expect(screen.getByRole("button", { name: "급여 설정" })).toBeDisabled();
      expect(screen.queryByText("급여와 근무시간을 설정해 주세요.")).not.toBeInTheDocument();
      await act(async () => { vi.advanceTimersByTime(2_000); });
      expect(getSalarySettings).toHaveBeenCalledTimes(2);
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "급여 설정" })).toBeEnabled();
      expect(screen.getByText("₩123,456")).toBeInTheDocument();
      fireEvent.click(screen.getByRole("button", { name: "급여 설정" }));
      expect(screen.getByLabelText("월 급여 (원)")).toHaveValue(settings.monthlyWon);
      cleanup();
      await act(async () => { vi.advanceTimersByTime(5_000); });
      expect(getSalarySettings).toHaveBeenCalledTimes(2);
    } finally { vi.useRealTimers(); }
  });

  it("can retry a failed initial read immediately", async () => {
    vi.mocked(getSalarySettings).mockRejectedValueOnce("temporary IPC failure").mockResolvedValue(settings);
    render(<SalaryCard />);
    fireEvent.click(await screen.findByRole("button", { name: "다시 시도" }));
    expect(await screen.findByText("₩123,456")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(saveSalarySettings).not.toHaveBeenCalled();
  });

  it("saves explicit settings then displays the Rust snapshot", async () => {
    render(<SalaryCard />);
    await waitFor(() => expect(screen.getByRole("button", { name: "급여 설정" })).toBeEnabled());
    expect(getSalarySnapshot).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "급여 설정" }));
    fireEvent.change(screen.getByLabelText("월 급여 (원)"), { target: { value: "4000000" } });
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    await waitFor(() => expect(saveSalarySettings).toHaveBeenCalledWith({ ...settings, monthlyWon: 4000000 }));
    expect(await screen.findByText("₩123,456")).toBeInTheDocument();
    expect(screen.getByText(/근무 중/)).toBeInTheDocument();
  });

  it("keeps the form and inputs when native validation fails", async () => {
    vi.mocked(getSalarySettings).mockResolvedValue(settings);
    vi.mocked(saveSalarySettings).mockRejectedValue("하루 총근무시간을 확인하세요.");
    render(<SalaryCard />);
    await waitFor(() => expect(screen.getByRole("button", { name: "급여 설정" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "급여 설정" }));
    fireEvent.click(screen.getByRole("button", { name: "저장" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("하루 총근무시간");
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByLabelText("월 급여 (원)")).toHaveValue(3000000);
    const previousCalls = vi.mocked(getSalarySnapshot).mock.calls.length;
    fireEvent.focus(window);
    await waitFor(() => expect(vi.mocked(getSalarySnapshot).mock.calls.length).toBeGreaterThan(previousCalls));
    expect(screen.getByRole("alert")).toHaveTextContent("하루 총근무시간");
  });

  it("deletes stored settings and removes the amount", async () => {
    vi.mocked(getSalarySettings).mockResolvedValue(settings);
    render(<SalaryCard />);
    expect(await screen.findByText("₩123,456")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "급여 설정" }));
    fireEvent.click(screen.getByRole("button", { name: "설정 삭제" }));
    await waitFor(() => expect(saveSalarySettings).toHaveBeenCalledWith(null));
    await waitFor(() => expect(screen.queryByText("₩123,456")).not.toBeInTheDocument());
  });
});
