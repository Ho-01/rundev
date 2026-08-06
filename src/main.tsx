import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { CharacterWindow } from "./components/CharacterWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles.css";

const isCharacterWindow = "__TAURI_INTERNALS__" in window && getCurrentWindow().label === "character";
if (isCharacterWindow) document.body.classList.add("character-body");

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isCharacterWindow ? <CharacterWindow /> : <App />}
  </StrictMode>
);
