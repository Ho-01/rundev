import type { RunnerId, RunnerSkinId } from "../../types/activity";

const masterFrames = import.meta.glob("./master/**/*.png", {
  eager: true,
  query: "?url",
  import: "default"
}) as Record<string, string>;

function frames(runnerId: RunnerId, skinId = "default") {
  const skinPrefix = skinId === "default" ? "" : `${skinId}/`;
  return [1, 2, 3, 4].map((frame) =>
    masterFrames[`./master/${runnerId}/${skinPrefix}${String(frame).padStart(2, "0")}.png`]
  );
}

export const desktopRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": frames("coding-cat"),
  "coding-fish": frames("coding-fish"),
  "coding-orange-cat": frames("coding-orange-cat"),
  "coding-shrimp": frames("coding-shrimp"),
  "coding-vtuber": frames("coding-vtuber")
};

export function desktopRunnerFrames(runnerId: RunnerId, skinId: RunnerSkinId = "default") {
  const skinFrames = frames(runnerId, skinId);
  return skinFrames.every(Boolean) ? skinFrames : desktopRunnerFramesById[runnerId];
}
