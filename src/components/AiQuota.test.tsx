import { act, render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { AiQuotaSummary, AiQuotaDetails, resetRemaining } from "./AiQuota";
import type { ProviderLimits } from "../services/usageLimits";

const quota: ProviderLimits = {
  provider: "claude", status: "connected", lastSyncedAt: "2026-11-17T05:00:00Z", error: null,
  windows: [
    { label: "5시간", usedPercent: 5, resetsAt: 1794893399, windowMinutes: 300 },
    { label: "주간", usedPercent: 56, resetsAt: 1794959999, windowMinutes: 10080 }
  ]
};

describe("AI account quota", () => {
  it("summarizes the most-used window while keeping every window available in details", () => {
    render(<><AiQuotaSummary quota={quota} /><AiQuotaDetails quota={quota} /></>);
    expect(screen.getByText("56%")).toBeInTheDocument();
    expect(screen.getByText("주간 사용률")).toBeInTheDocument();
    expect(screen.getByText(/^5% ·/)).toBeInTheDocument();
  });

  it("distinguishes a real zero from missing, disconnected and failed data", () => {
    const view = render(<AiQuotaSummary quota={{ ...quota, windows: [{ ...quota.windows[0], usedPercent: 0 }] }} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
    view.rerender(<AiQuotaSummary quota={{ ...quota, status: "unavailable", windows: [] }} />);
    expect(screen.getByText("정보 없음")).toBeInTheDocument();
    expect(screen.queryByText("0%")).not.toBeInTheDocument();
    view.rerender(<AiQuotaSummary quota={{ ...quota, status: "disconnected", windows: [] }} />);
    expect(screen.getByText("연동하기")).toBeInTheDocument();
    view.rerender(<AiQuotaSummary quota={{ ...quota, status: "error", error: "조회 실패" }} />);
    expect(screen.getByText("56%")).toBeInTheDocument();
    expect(screen.getByText("갱신 실패 · 마지막 수신값")).toBeInTheDocument();
    view.rerender(<AiQuotaSummary quota={{ ...quota, status: "error", windows: [] }} />);
    expect(screen.getByText("확인 필요")).toBeInTheDocument();
    expect(screen.queryByText("갱신 실패 · 마지막 수신값")).not.toBeInTheDocument();
  });

  it("updates the reset countdown without additional provider requests", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-11-17T05:00:00Z"));
      const reset = Date.now() / 1000 + 90;
      render(<AiQuotaSummary quota={{ ...quota, windows: [{ ...quota.windows[0], resetsAt: reset }] }} />);
      expect(screen.getByText("2분 후 초기화")).toBeInTheDocument();
      act(() => { vi.advanceTimersByTime(60_000); });
      expect(screen.getByText("1분 후 초기화")).toBeInTheDocument();
      act(() => { vi.advanceTimersByTime(30_000); });
      expect(screen.getByText("초기화 확인 중")).toBeInTheDocument();
    } finally { vi.useRealTimers(); }
  });

  it("does not invent a reset time", () => {
    expect(resetRemaining(null, Date.now())).toBe("초기화 시각 없음");
  });
});
