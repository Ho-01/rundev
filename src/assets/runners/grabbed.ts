import type { RunnerId } from "../../types/activity";

const grabbedFrames = import.meta.glob("./master/*/grabbed*.png", {
  eager: true,
  query: "?url",
  import: "default"
}) as Record<string, string>;

function frames(runnerId: RunnerId) {
  return ["grabbed.png", "grabbed-2.png", "grabbed-3.png"].map(
    (frame) => grabbedFrames[`./master/${runnerId}/${frame}`]
  );
}

export const grabbedRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": frames("coding-cat"),
  "coding-fish": frames("coding-fish"),
  "coding-orange-cat": frames("coding-orange-cat"),
  "coding-shrimp": frames("coding-shrimp"),
  "coding-vtuber": frames("coding-vtuber")
};
