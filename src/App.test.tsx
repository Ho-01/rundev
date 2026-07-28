import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { App } from "./App";

describe("App", () => {
  it("renders the starter dashboard", async () => {
    render(<App />);
    expect(await screen.findByText("RunDev")).toBeInTheDocument();
    expect(await screen.findByText("새싹 개발자")).toBeInTheDocument();
    expect(await screen.findByText("개발 활동")).toBeInTheDocument();
    expect(await screen.findByText("Claude Code")).toBeInTheDocument();
  });
});
