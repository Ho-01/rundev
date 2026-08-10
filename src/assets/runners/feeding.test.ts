import { describe, expect, it } from "vitest";
import { feedingRunnerFramesById } from "./feeding";

describe("feeding runner frames", () => {
  it("provides ready, bite, chew, and swallow frames for every runner", () => {
    for (const frames of Object.values(feedingRunnerFramesById)) {
      expect(frames).toHaveLength(4);
      expect(frames.every(Boolean)).toBe(true);
    }
  });
});
