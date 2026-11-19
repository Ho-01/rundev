import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { TraitDialog } from "./TraitDialog";
import type { TraitProgress } from "../types/activity";
afterEach(cleanup);
const progress: TraitProgress = { availablePoints: 3, earnedPoints: 4, spentPoints: 1, nextFocusRewardXp: 10, nextKeyboardRewardXp: 10,
  traits: ["focus-ready", "hot-keyboard", "reload"].map((id, i) => ({ id: id as "focus-ready" | "hot-keyboard" | "reload", level: 1, maxLevel: 20,
    effectValue: i === 2 ? 2 : 5, nextEffectValue: i === 2 ? 4 : 10, effectUnit: i === 2 ? "xp-per-active-day" : "percent",
    exampleRewardValue: i === 2 ? 2 : 10.5, nextExampleRewardValue: i === 2 ? 4 : 11, upgradeCost: 1 })) };
it("shows 16 nodes and keeps all 13 coming-soon nodes free of upgrade actions", () => {
  const upgrade = vi.fn();render(<TraitDialog progress={progress} onUpgrade={upgrade} onClose={() => {}} />);
  expect(document.querySelectorAll("[data-trait-node]")).toHaveLength(16);
  for (let i = 1; i <= 13; i++) {
    fireEvent.click(screen.getByRole("button", { name: `확장 노드 ${i} 선택` }));
    const detail = screen.getByRole("region", { name: "선택한 특성 상세" });
    expect(within(detail).getByText("확장 노드")).toBeInTheDocument();
    expect(within(detail).getByText("Coming soon")).toBeInTheDocument();
    expect(within(detail).queryByRole("button")).not.toBeInTheDocument();
  }
  expect(upgrade).not.toHaveBeenCalled();
});
it("shows the selected native reward values and serializes upgrades", async () => {
  let finish!: () => void;
  const upgrade = vi.fn(() => new Promise<void>(resolve => { finish = resolve; }));
  render(<TraitDialog progress={progress} onUpgrade={upgrade} onClose={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "뜨거운 키보드 선택" }));
  expect(screen.getByText("10.5 → 11.0 XP")).toBeInTheDocument();
  const button=screen.getByRole("button", { name: "뜨거운 키보드 강화" });
  fireEvent.click(button);fireEvent.click(button);
  expect(upgrade).toHaveBeenCalledExactlyOnceWith("hot-keyboard");expect(button).toBeDisabled();
  await act(async () => finish());await waitFor(() => expect(button).toBeEnabled());
});
it("reports an upgrade failure and preserves the ability to retry", async () => {
  render(<TraitDialog progress={progress} onUpgrade={vi.fn().mockRejectedValue(new Error("failed"))} onClose={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "몰입 준비 강화" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("강화하지 못했습니다.");
  expect(screen.getByRole("button", { name: "몰입 준비 강화" })).toBeEnabled();
});
it("blocks spending when points are missing or the trait is maxed and closes with Escape", () => {
  const close=vi.fn();const data={...progress,availablePoints:0,traits:progress.traits.map(t=>({...t,level:20}))};
  render(<TraitDialog progress={data} onUpgrade={vi.fn()} onClose={close} />);
  expect(screen.getByRole("button", { name:"몰입 준비 강화" })).toBeDisabled();
  fireEvent.keyDown(screen.getByRole("dialog"),{key:"Escape"});expect(close).toHaveBeenCalledOnce();
});
