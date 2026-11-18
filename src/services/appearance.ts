import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { resolveTheme, type AccentTheme, type AccentThemeId } from "../domain/themes";

const CACHE_KEY = "rundev.accentTheme";
const native = () => "__TAURI_INTERNALS__" in window;
let revision = 0;
let initialization: Promise<void> | undefined;
let listening = false;

/** A preview changes this view only; it never changes the persisted selection. */
export function previewTheme(theme: AccentTheme) {
  revision += 1;
  const rgb = [1, 3, 5].map(index => parseInt(theme.hex.slice(index, index + 2), 16));
  document.documentElement.style.setProperty("--rundev-accent", theme.hex);
  document.documentElement.style.setProperty("--rundev-accent-rgb", rgb.join(", "));
  document.documentElement.dataset.accentTheme = theme.id;
}

function rememberTheme(id: string) {
  const theme = resolveTheme(id);
  localStorage.setItem(CACHE_KEY, theme.id);
  previewTheme(theme);
}

export async function getAccentTheme(): Promise<AccentTheme> {
  return resolveTheme(native() ? await invoke<string>("get_accent_theme") : localStorage.getItem(CACHE_KEY));
}

export async function saveAccentTheme(id: AccentThemeId) {
  if (native()) await invoke("set_accent_theme", { theme: id });
  revision += 1;
  rememberTheme(id);
}

export function initializeAppearance() {
  if (initialization) return initialization;
  previewTheme(resolveTheme(localStorage.getItem(CACHE_KEY)));
  const observed = revision;
  initialization = (async () => {
    if (native() && !listening) {
      await listen<string>("accent-theme-changed", ({ payload }) => {
        revision += 1;
        rememberTheme(payload);
      });
      listening = true;
    }
    const theme = await getAccentTheme();
    if (observed === revision) rememberTheme(theme.id);
  })().catch(() => { initialization = undefined; });
  return initialization;
}
