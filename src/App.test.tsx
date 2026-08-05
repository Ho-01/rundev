import { act, fireEvent, render, screen, within } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { App } from "./App";
import { useDashboardStore } from "./store/dashboard";

describe("App", () => {
  it("renders the starter dashboard", async () => {
    localStorage.clear();
    render(<App />);
    expect(document.querySelector(".whip-crack-canvas")).toHaveAttribute("width", "340");
    expect(await screen.findByText("RunDev")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: /레벨 1 · 특성 열기/ })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "사용 가능한 특성 포인트 2개" })).toBeInTheDocument();
    expect(await screen.findByText("새싹 개발자")).toBeInTheDocument();
    expect(await screen.findByText("개발 활동")).toBeInTheDocument();
    expect(await screen.findByText("개발 도구 노려본 시간")).toBeInTheDocument();
    expect(await screen.findByText("마지막으로 본 도구")).toBeInTheDocument();
    expect(await screen.findByText("20분마다")).toBeInTheDocument();
    expect(await screen.findByText("2,000회마다")).toBeInTheDocument();
    expect(await screen.findAllByText(/^오늘 \d+회 달성$/)).toHaveLength(2);
    expect(await screen.findByText("Claude Code")).toBeInTheDocument();
    expect(await screen.findByText("AI 사용량")).toBeInTheDocument();
    expect((await screen.findAllByText("Cursor")).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByRole("progressbar", { name: "이번 주 AI 토큰 사용 XP" })).toHaveAttribute(
      "aria-valuemax",
      "210"
    );
    fireEvent.click(screen.getByRole("button", { name: "장치 상세 펼치기" }));
    expect(screen.getByRole("button", { name: "장치 상세 접기" })).toHaveAttribute(
      "aria-expanded",
      "true"
    );
    expect(document.querySelector(".whip-crack-canvas")).toHaveAttribute("width", "512");
    expect(screen.getByText("논리 코어")).toBeInTheDocument();
    expect(screen.getByText("실행 후 최고")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "장치 상세 접기" }));
    const claudeSummary = screen.getByText("Claude Code").closest("summary");
    expect(claudeSummary).not.toBeNull();
    const claudeDetails = claudeSummary!.parentElement!;
    expect(within(claudeDetails).getByText("최근 활동 세션")).not.toBeVisible();
    fireEvent.click(claudeSummary!);
    expect(within(claudeDetails).getByText("최근 활동 세션")).toBeVisible();
    expect(within(claudeDetails).getByText("이번 주 토큰 사용량")).toBeVisible();
    fireEvent.click(claudeSummary!);
    expect(within(claudeDetails).getByText("최근 활동 세션")).not.toBeVisible();
    expect(await screen.findByText("오늘 두드린 키보드")).toBeInTheDocument();
    expect(await screen.findByText("최근 20주 활동")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "RunDev 정보" }));
    expect(screen.getByRole("heading", { name: "RunDev 정보" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "진단 로그 폴더" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "개발자 변경" }));
    expect(screen.getByText("주황 고양이")).toBeInTheDocument();
    expect(screen.getByText("주황 새우")).toBeInTheDocument();
    expect(screen.getByText("핑크 버튜버")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "활동 통계" }));
    expect(await screen.findByRole("complementary", { name: "활동 통계" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /최근 20주 중/ })).toBeInTheDocument();
    expect(screen.getAllByText("XP 구성")).toHaveLength(2);
    expect(screen.getByLabelText("오늘 XP 출처별 구성")).toBeInTheDocument();
    expect(screen.getByLabelText("이번 주 XP 출처별 구성")).toBeInTheDocument();
    expect(screen.getByLabelText("요일")).toHaveTextContent("월화수목금토일");
    fireEvent.click(screen.getByRole("button", { name: "통계 접기" }));
    fireEvent.click(document.querySelector(".trait-launcher")!);
    expect(await screen.findByRole("dialog", { name: "개발자 특성" })).toBeInTheDocument();
  });

  it("offers permission recovery directly from the keyboard card", async () => {
    localStorage.clear();
    render(<App />);
    expect(await screen.findByText("RunDev")).toBeInTheDocument();

    act(() => {
      useDashboardStore.setState({
        keyboard: {
          localDate: "2026-07-30",
          pressCount: 0,
          rewardedMilestones: 0,
          xpEarned: 0,
          nextRewardAt: 2_000,
          pressesPerReward: 2_000,
          status: "permission-required",
          permissionRequired: true
        }
      });
    });

    expect(
      screen.getByText("새 설치 후에는 입력 권한을 다시 연결해야 할 수 있습니다.")
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "권한 다시 연결" })
    ).toBeInTheDocument();
  });
});
