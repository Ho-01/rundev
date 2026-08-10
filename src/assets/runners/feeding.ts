import type { RunnerId } from "../../types/activity";

const feedingFrames = import.meta.glob("./master/*/feed-*.png", {
  eager: true,
  query: "?url",
  import: "default"
}) as Record<string, string>;

function frames(runnerId: RunnerId) {
  return ["ready", "bite", "chew", "swallow"].map(
    (frame) => feedingFrames[`./master/${runnerId}/feed-${frame}.png`]
  );
}

export const feedingRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": frames("coding-cat"),
  "coding-fish": frames("coding-fish"),
  "coding-orange-cat": frames("coding-orange-cat"),
  "coding-shrimp": frames("coding-shrimp"),
  "coding-vtuber": frames("coding-vtuber")
};
