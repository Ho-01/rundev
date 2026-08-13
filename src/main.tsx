import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { CharacterWindow } from "./components/CharacterWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles.css";

const isTauri = "__TAURI_INTERNALS__" in window;
const currentWindowLabel = isTauri ? getCurrentWindow().label : null;
const isCharacterWindow = currentWindowLabel === "character";
if (isCharacterWindow) {
  document.documentElement.classList.add("character-body");
  document.body.classList.add("character-body");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isCharacterWindow ? <CharacterWindow /> : <App />}
  </StrictMode>
);
