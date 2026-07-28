import { invoke } from "@tauri-apps/api/core";
import type { CharacterState, DailySummary } from "../types/activity";

const previewSummary: DailySummary = {
  date: new Date().toISOString().slice(0, 10),
  activeSeconds: 0,
  xpEarned: 0,
  aiEvents: 0
};

const previewCharacter: CharacterState = {
  level: 1,
  totalXp: 0,
  currentForm: "sprout",
  xpIntoLevel: 0,
  xpForNextLevel: 100
};

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export async function getDashboard() {
  if (!isTauri()) {
    return { summary: previewSummary, character: previewCharacter };
  }

  const [summary, character] = await Promise.all([
    invoke<DailySummary>("get_daily_summary"),
    invoke<CharacterState>("get_character_state")
  ]);

  return { summary, character };
}
