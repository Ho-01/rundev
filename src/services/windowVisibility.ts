import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function subscribeWindowVisibility(onChange: (visible: boolean) => void) {
  let disposed = false;
  let revision = 0;
  let off = () => {};
  const notify = () => { if (!disposed) onChange(!document.hidden); };
  document.addEventListener("visibilitychange", notify);
  if ("__TAURI_INTERNALS__" in window) {
    void listen<boolean>("rundev-window-visible", ({ payload }) => {
      revision += 1;
      if (!disposed) onChange(payload);
    }).then(async (unlisten) => {
      if (disposed) { unlisten(); return; }
      off = unlisten;
      const observed = revision;
      const visible = await getCurrentWindow().isVisible();
      if (!disposed && observed === revision) onChange(visible);
    }).catch(() => {});
  }
  return () => { disposed = true; off(); document.removeEventListener("visibilitychange", notify); };
}
