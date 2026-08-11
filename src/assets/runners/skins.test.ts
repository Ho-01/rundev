import { describe, expect, it } from "vitest";
import { desktopRunnerFrames } from "./desktop";
import { feedingRunnerFrames } from "./feeding";
import { grabbedRunnerFrames } from "./grabbed";
import { runnerFrames } from "./index";
import { roamingRunnerFrames } from "./roaming";

describe("pool party pink vtuber skin", () => {
  it("provides every visual state without falling back to the base skin", () => {
    const runnerId = "coding-vtuber" as const;
    const skinId = "pool-party" as const;

    expect(runnerFrames(runnerId, skinId)).toHaveLength(4);
    expect(desktopRunnerFrames(runnerId, skinId)).toHaveLength(4);
    expect(feedingRunnerFrames(runnerId, skinId)).toHaveLength(4);
    expect(grabbedRunnerFrames(runnerId, skinId)).toHaveLength(3);
    expect(roamingRunnerFrames(runnerId, skinId)).toHaveLength(4);

    for (const frames of [
      runnerFrames(runnerId, skinId),
      desktopRunnerFrames(runnerId, skinId),
      feedingRunnerFrames(runnerId, skinId),
      grabbedRunnerFrames(runnerId, skinId),
      roamingRunnerFrames(runnerId, skinId)
    ]) {
      expect(frames.every(Boolean)).toBe(true);
    }
  });
});
