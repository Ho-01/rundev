import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { accentThemes } from "../domain/themes";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

beforeEach(() => {
  vi.resetModules(); vi.resetAllMocks(); localStorage.clear();
  document.documentElement.removeAttribute("style");
  delete document.documentElement.dataset.accentTheme;
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("appearance preference", () => {
  it("previews locally and restores the saved choice after a new view starts", async () => {
    const appearance = await import("./appearance");
    await appearance.saveAccentTheme("lavender");
    appearance.previewTheme(accentThemes[10]);
    expect(document.documentElement.style.getPropertyValue("--rundev-accent")).toBe("#F29CA5");
    expect(localStorage.getItem("rundev.accentTheme")).toBe("lavender");
    await appearance.initializeAppearance();
    expect(document.documentElement.dataset.accentTheme).toBe("lavender");
  });

  it("loads Rust's saved setting and applies cross-window changes", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    let changed: ((event: {payload:string}) => void) | undefined;
    vi.mocked(listen).mockImplementation(async (_event, handler) => { changed = handler as typeof changed; return () => {}; });
    vi.mocked(invoke).mockResolvedValue("sky-blue");
    const appearance = await import("./appearance");
    await appearance.initializeAppearance();
    expect(document.documentElement.dataset.accentTheme).toBe("sky-blue");
    changed?.({payload:"coral"});
    expect(document.documentElement.dataset.accentTheme).toBe("coral");
    expect(document.documentElement.style.getPropertyValue("--rundev-accent-rgb")).toBe("242, 156, 165");
    vi.mocked(invoke).mockRejectedValueOnce(new Error("disk busy"));
    await expect(appearance.saveAccentTheme("periwinkle")).rejects.toThrow("disk busy");
    expect(localStorage.getItem("rundev.accentTheme")).toBe("coral");
  });
});
