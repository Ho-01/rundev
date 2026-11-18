import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles.css";
import { initializeAppearance } from "./services/appearance";

void initializeAppearance();

const isTauri = "__TAURI_INTERNALS__" in window;
const currentWindowLabel = isTauri ? getCurrentWindow().label : null;
const isCharacterWindow = currentWindowLabel === "character";
const isSalaryWindow = currentWindowLabel === "character-salary";
// Import only the active window's tree. The salary companion does not load the
// dashboard, charts, physics engine or character sprite catalog.
const View = lazy(() => isSalaryWindow
  ? import("./components/CharacterSalaryWindow").then((module) => ({ default: module.CharacterSalaryWindow }))
  : isCharacterWindow
    ? import("./components/CharacterWindow").then((module) => ({ default: module.CharacterWindow }))
    : import("./App").then((module) => ({ default: module.App })));
if (isCharacterWindow || isSalaryWindow) {
  document.documentElement.classList.add("character-body");
  document.body.classList.add("character-body");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Suspense fallback={null}><View /></Suspense>
  </StrictMode>
);
