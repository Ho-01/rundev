import { describe, expect, it } from "vitest";
import { roamingRunnerFramesById } from "./roaming";

describe("roaming runner frames", () => {
  it("keeps every runner on a four-frame movement cycle", () => {
    for (const frames of Object.values(roamingRunnerFramesById)) {
      expect(frames).toHaveLength(4);
      expect(frames.every(Boolean)).toBe(true);
    }
  });
});
