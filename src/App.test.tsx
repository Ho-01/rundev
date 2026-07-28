import { fireEvent, render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { App } from "./App";

describe("App", () => {
  it("renders the starter dashboard", async () => {
    render(<App />);
    expect(await screen.findByText("RunDev")).toBeInTheDocument();
    expect(await screen.findByText("새싹 개발자")).toBeInTheDocument();
    expect(await screen.findByText("개발 활동")).toBeInTheDocument();
    expect(await screen.findByText("활성 AI")).toBeInTheDocument();
    expect(await screen.findByText("Claude Code")).toBeInTheDocument();
    expect(await screen.findByText("오늘 두드린 키보드")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "개발자 변경" }));
    expect(screen.getByText("주황 고양이")).toBeInTheDocument();
    expect(screen.getByText("주황 새우")).toBeInTheDocument();
    expect(screen.getByText("핑크 버튜버")).toBeInTheDocument();
  });
});
