import type { RunnerId } from "../../types/activity";

const masterFrames = import.meta.glob("./master/*/*.png", {
  eager: true,
  query: "?url",
  import: "default"
}) as Record<string, string>;

function frames(runnerId: RunnerId) {
  return [1, 2, 3, 4].map((frame) =>
    masterFrames[`./master/${runnerId}/${String(frame).padStart(2, "0")}.png`]
  );
}

export const desktopRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": frames("coding-cat"),
  "coding-fish": frames("coding-fish"),
  "coding-orange-cat": frames("coding-orange-cat"),
  "coding-shrimp": frames("coding-shrimp"),
  "coding-vtuber": frames("coding-vtuber")
};
