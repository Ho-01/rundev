import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview, type DragDropEvent } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { RunnerSelection } from "../types/activity";

export type CharacterWindowState = { visible: boolean; followPointer: boolean };

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export async function getCharacterWindowState(): Promise<CharacterWindowState> {
  if (!isTauri()) return { visible: false, followPointer: false };
  return invoke<CharacterWindowState>("get_state");
}

export async function setCharacterWindowVisible(visible: boolean): Promise<CharacterWindowState> {
  if (!isTauri()) return { visible, followPointer: false };
  return invoke<CharacterWindowState>("set_visible", { visible });
}

export async function dragCharacterWindow() {
  if (!isTauri()) return;
  await getCurrentWindow().startDragging();
  await invoke("save_position");
}

export async function beginCharacterFileDrop() {
  if (!isTauri()) return;
  await invoke("begin_character_file_drop");
}

export async function endCharacterFileDrop() {
  if (!isTauri()) return;
  await invoke("end_character_file_drop");
}

export async function trashDroppedFiles(paths: string[]) {
  if (!isTauri()) return paths.length;
  return invoke<number>("trash_dropped_files", { paths });
}

export async function showCharacterContextMenu() {
  if (!isTauri()) return;
  await invoke("show_context_menu");
}

export async function getCharacterRunner(): Promise<RunnerSelection> {
  if (!isTauri()) return { runnerId: "coding-cat" };
  return invoke<RunnerSelection>("get_runner_selection");
}

export async function subscribeCharacterWindowState(
  callback: (state: CharacterWindowState) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<CharacterWindowState>("character-window-state-changed", ({ payload }) => callback(payload));
}

export async function subscribeRunnerSelection(
  callback: (runner: RunnerSelection) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<RunnerSelection>("runner-selection-changed", ({ payload }) => callback(payload));
}

export async function subscribeTypingPulse(callback: () => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen("keyboard-typing-pulse", callback);
}

export async function subscribeCharacterFileDrop(
  callback: (event: DragDropEvent) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return getCurrentWebview().onDragDropEvent(({ payload }) => callback(payload));
}
