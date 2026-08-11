import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview, type DragDropEvent } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { RunnerSelection } from "../types/activity";

export type CharacterWindowState = {
  visible: boolean;
  followPointer: boolean;
  roaming: boolean;
  moving: boolean;
  direction: number;
  size: number;
};

export type CharacterMotionState = { moving: boolean; direction: number };


function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export async function getCharacterWindowState(): Promise<CharacterWindowState> {
  if (!isTauri()) return { visible: false, followPointer: false, roaming: false, moving: false, direction: 1, size: 48 };
  return invoke<CharacterWindowState>("get_state");
}

export async function setCharacterWindowVisible(visible: boolean): Promise<CharacterWindowState> {
  if (!isTauri()) return { visible, followPointer: false, roaming: false, moving: false, direction: 1, size: 48 };
  return invoke<CharacterWindowState>("set_visible", { visible });
}

export async function toggleCharacterRoaming() {
  if (!isTauri()) return;
  await invoke("toggle_roaming");
}

export async function dragCharacterWindow() {
  if (!isTauri()) return;
  await getCurrentWindow().startDragging();
  await invoke("save_position");
}

export async function beginCharacterDrag(scale: number) {
  if (!isTauri()) return;
  await invoke("begin_character_drag", { scale });
}

export async function endCharacterDrag() {
  if (!isTauri()) return;
  await invoke("end_character_drag");
}

export async function resizeCharacterWindow(size: number): Promise<number> {
  if (!isTauri()) return size;
  return invoke<number>("resize_character_window", { size });
}

export async function finishCharacterResize(size: number): Promise<number> {
  if (!isTauri()) return size;
  return invoke<number>("finish_character_resize", { size });
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
  if (!isTauri()) return { runnerId: "coding-cat", skinId: "default" };
  return invoke<RunnerSelection>("get_runner_selection");
}

export async function subscribeCharacterWindowState(
  callback: (state: CharacterWindowState) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<CharacterWindowState>("character-window-state-changed", ({ payload }) => callback(payload));
}

export async function subscribeCharacterMotion(
  callback: (state: CharacterMotionState) => void
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<CharacterMotionState>("character-window-motion-changed", ({ payload }) => callback(payload));
}

export async function subscribeCharacterDragEnd(callback: () => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen("character-window-drag-ended", callback);
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
