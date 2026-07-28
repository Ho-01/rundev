import { describe, expect, it } from "vitest";
import { getLevelTier, getNextLevelTier } from "./level";

describe("developer level tiers", () => {
  it("changes tiers at the documented level boundaries", () => {
    expect(getLevelTier(1).id).toBe("sprout");
    expect(getLevelTier(4).id).toBe("sprout");
    expect(getLevelTier(5).id).toBe("rookie");
    expect(getLevelTier(10).id).toBe("maker");
    expect(getLevelTier(20).id).toBe("debugger");
    expect(getLevelTier(50).id).toBe("systems");
    expect(getLevelTier(70).id).toBe("architect");
    expect(getLevelTier(250).id).toBe("legend");
    expect(getLevelTier(999).id).toBe("legend");
  });

  it("finds the next growth tier until legend", () => {
    expect(getNextLevelTier(1)?.minLevel).toBe(5);
    expect(getNextLevelTier(9)?.minLevel).toBe(10);
    expect(getNextLevelTier(70)?.minLevel).toBe(100);
    expect(getNextLevelTier(250)).toBeNull();
  });
});
