import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeDialog } from "./ThemeDialog";
import { accentThemes } from "../domain/themes";
import { getAccentTheme, previewTheme, saveAccentTheme } from "../services/appearance";

vi.mock("../services/appearance", () => ({
  getAccentTheme: vi.fn(), previewTheme: vi.fn(), saveAccentTheme: vi.fn()
}));

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getAccentTheme).mockResolvedValue(accentThemes[0]);
  vi.mocked(saveAccentTheme).mockResolvedValue(undefined);
});

describe("ThemeDialog", () => {
  it("previews all candidates, persists only on apply, and keeps the applied color on close", async () => {
    const close = vi.fn();
    const view = render(<ThemeDialog onClose={close} />);
    await waitFor(() => expect(screen.getByRole("radio", { name: "퍼리윙클" })).toBeEnabled());
    expect(screen.getAllByRole("radio")).toHaveLength(13);
    fireEvent.click(screen.getByRole("radio", { name: "퍼리윙클" }));
    expect(previewTheme).toHaveBeenLastCalledWith(accentThemes[1]);
    expect(saveAccentTheme).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "적용" }));
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    expect(saveAccentTheme).toHaveBeenCalledWith("periwinkle");
    vi.mocked(previewTheme).mockClear();
    view.unmount();
    expect(previewTheme).not.toHaveBeenCalled();
  });

  it("restores the saved color when preview is cancelled or the window unmounts", async () => {
    const view = render(<ThemeDialog onClose={() => {}} />);
    await waitFor(() => expect(screen.getByRole("radio", { name: "코랄" })).toBeEnabled());
    fireEvent.click(screen.getByRole("radio", { name: "코랄" }));
    expect(previewTheme).toHaveBeenLastCalledWith(accentThemes[10]);
    view.unmount();
    expect(previewTheme).toHaveBeenLastCalledWith(accentThemes[0]);
    expect(saveAccentTheme).not.toHaveBeenCalled();
  });

  it("keeps the dialog open on failed saves and supports keyboard selection and Escape", async () => {
    vi.mocked(saveAccentTheme).mockRejectedValue(new Error("disk busy"));
    const close = vi.fn();
    render(<ThemeDialog onClose={close} />);
    const first = screen.getByRole("radio", { name: "기본 그린" });
    await waitFor(() => expect(first).toHaveFocus());
    fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(screen.getByRole("radio", { name: "퍼리윙클" })).toHaveAttribute("aria-checked", "true");
    fireEvent.click(screen.getByRole("button", { name: "적용" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("저장하지 못했습니다");
    expect(close).not.toHaveBeenCalled();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(close).toHaveBeenCalledOnce();
  });

  it("blocks overwriting unread settings and recovers through retry", async () => {
    vi.mocked(getAccentTheme).mockRejectedValueOnce(new Error("not ready")).mockResolvedValue(accentThemes[2]);
    render(<ThemeDialog onClose={() => {}} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("읽지 못했습니다");
    expect(screen.getByRole("button", { name: "적용" })).toBeDisabled();
    expect(saveAccentTheme).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "다시 시도" }));
    await waitFor(() => expect(screen.getByRole("radio", { name: "라벤더" })).toHaveAttribute("aria-checked", "true"));
    expect(screen.getByRole("button", { name: "적용" })).toBeEnabled();
  });
});
