import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUsageLimits } from "./useUsageLimits";
import { useSalarySnapshot } from "./useSalarySnapshot";
import { getAccountUsageLimits } from "../services/usageLimits";
import { getSalarySnapshot } from "../services/salary";

vi.mock("../services/usageLimits", () => ({ getAccountUsageLimits: vi.fn(async () => []) }));
vi.mock("../services/salary", () => ({
  getSalarySnapshot: vi.fn(async () => null), subscribeSalaryChanges: () => Promise.resolve(() => {})
}));
vi.mock("../services/windowVisibility", () => ({ subscribeWindowVisibility: () => () => {} }));

beforeEach(() => { vi.useFakeTimers(); vi.clearAllMocks(); });
afterEach(() => { vi.useRealTimers(); });

describe("view polling lifecycle", () => {
  it("stops AI cache and salary IPC while hidden, then refreshes immediately on reopen", async () => {
    const view = renderHook(({ active }) => ({ limits: useUsageLimits(active), salary: useSalarySnapshot(active) }),
      { initialProps: { active: true } });
    await act(async () => {});
    expect(getAccountUsageLimits).toHaveBeenCalledTimes(1);
    expect(getSalarySnapshot).toHaveBeenCalledTimes(1);
    view.rerender({ active: false });
    await act(async () => { await vi.advanceTimersByTimeAsync(60_000); });
    expect(getAccountUsageLimits).toHaveBeenCalledTimes(1);
    expect(getSalarySnapshot).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
    view.rerender({ active: true });
    await act(async () => {});
    expect(getAccountUsageLimits).toHaveBeenCalledTimes(2);
    expect(getSalarySnapshot).toHaveBeenCalledTimes(2);
    view.unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("does not start any polling for a view that was never opened", async () => {
    const view = renderHook(() => { useUsageLimits(false); useSalarySnapshot(false); });
    await act(async () => { await vi.advanceTimersByTimeAsync(60_000); });
    expect(getAccountUsageLimits).not.toHaveBeenCalled();
    expect(getSalarySnapshot).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
    view.unmount();
  });
});
