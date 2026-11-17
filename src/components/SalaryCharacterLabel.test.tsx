import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SalaryCharacterLabel } from "./SalaryCharacterLabel";
import type { SalarySnapshot } from "../services/salary";

afterEach(cleanup);
const snapshot: SalarySnapshot = { cycleStart: "2026-10-25", nextPayday: "2026-11-25",
  cycleWon: 2000000, todayWon: 12345.9, wonPerSecond: 3.42, working: true, workStatus: "working" as const, secondsUntilEnd: 3600, workdays: 22 };

describe("character salary", () => {
  it("renders a repeating gain effect without a persistent rate label", () => {
    render(<SalaryCharacterLabel snapshot={snapshot} />);
    const gain = screen.getByLabelText("초당 3.42원 증가");
    expect(gain).toHaveTextContent("+₩3.42");
    expect(gain).toHaveClass("salary-character-gain");
    expect(screen.queryByText(/\/초/)).not.toBeInTheDocument();
    expect(screen.queryByText(/버는 중/)).not.toBeInTheDocument();
  });
  it("uses today's amount in the floating bubble", () => {
    render(<SalaryCharacterLabel snapshot={snapshot} bubble />);
    expect(screen.getByText("오늘 12,345원째 버는 중!")).toBeInTheDocument();
  });
  it("stops the rate and explains the pause outside working hours", () => {
    render(<SalaryCharacterLabel snapshot={{ ...snapshot, working: false }} bubble />);
    expect(screen.queryByLabelText(/초당 .*원 증가/)).not.toBeInTheDocument();
    expect(screen.getByText("오늘 12,345원 벌었어요! 지금은 쉬는 중")).toBeInTheDocument();
  });
  it("keeps the animation element when a new snapshot arrives each second", () => {
    const { rerender } = render(<SalaryCharacterLabel snapshot={snapshot} bubble />);
    const gain = screen.getByLabelText("초당 3.42원 증가");
    rerender(<SalaryCharacterLabel snapshot={{ ...snapshot, todayWon: 12349.32 }} bubble />);
    expect(screen.getByLabelText("초당 3.42원 증가")).toBe(gain);
    expect(screen.getByText("오늘 12,349원째 버는 중!")).toBeInTheDocument();
  });
  it("hides salary text when no settings or snapshot exist", () => {
    const { container } = render(<SalaryCharacterLabel snapshot={null} bubble />);
    expect(container).toBeEmptyDOMElement();
  });
});
