import { describe, expect, it } from "vitest";
import { grabbedRunnerFramesById } from "./grabbed";

describe("grabbed runner frames", () => {
  it("provides three held frames for every runner", () => {
    for (const frames of Object.values(grabbedRunnerFramesById)) {
      expect(frames).toHaveLength(3);
      expect(frames.every(Boolean)).toBe(true);
    }
  });
});
