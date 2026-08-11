import type { RunnerId, RunnerSkinId } from "../../types/activity";

const feedingFrames = import.meta.glob("./master/**/feed-*.png", {
  eager: true,
  query: "?url",
  import: "default"
}) as Record<string, string>;

function frames(runnerId: RunnerId, skinId = "default") {
  const skinPrefix = skinId === "default" ? "" : `${skinId}/`;
  return ["ready", "bite", "chew", "swallow"].map(
    (frame) => feedingFrames[`./master/${runnerId}/${skinPrefix}feed-${frame}.png`]
  );
}

export const feedingRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": frames("coding-cat"),
  "coding-fish": frames("coding-fish"),
  "coding-orange-cat": frames("coding-orange-cat"),
  "coding-shrimp": frames("coding-shrimp"),
  "coding-vtuber": frames("coding-vtuber")
};

export function feedingRunnerFrames(runnerId: RunnerId, skinId: RunnerSkinId = "default") {
  const skinFrames = frames(runnerId, skinId);
  return skinFrames.every(Boolean) ? skinFrames : feedingRunnerFramesById[runnerId];
}
