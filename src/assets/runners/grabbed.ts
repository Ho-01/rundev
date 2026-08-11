import type { RunnerId, RunnerSkinId } from "../../types/activity";

const grabbedFrames = import.meta.glob("./master/**/grabbed*.png", {
  eager: true,
  query: "?url",
  import: "default"
}) as Record<string, string>;

function frames(runnerId: RunnerId, skinId = "default") {
  const skinPrefix = skinId === "default" ? "" : `${skinId}/`;
  return ["grabbed.png", "grabbed-2.png", "grabbed-3.png"].map(
    (frame) => grabbedFrames[`./master/${runnerId}/${skinPrefix}${frame}`]
  );
}

export const grabbedRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": frames("coding-cat"),
  "coding-fish": frames("coding-fish"),
  "coding-orange-cat": frames("coding-orange-cat"),
  "coding-shrimp": frames("coding-shrimp"),
  "coding-vtuber": frames("coding-vtuber")
};

export function grabbedRunnerFrames(runnerId: RunnerId, skinId: RunnerSkinId = "default") {
  const skinFrames = frames(runnerId, skinId);
  return skinFrames.every(Boolean) ? skinFrames : grabbedRunnerFramesById[runnerId];
}
