import { describe, expect, it } from "vitest";
import {
  actionForPingSlot,
  pingSlotForDelta
} from "./pingMode";

describe("ping mode radial selection", () => {
  it("keeps only a stationary pointer on the center action", () => {
    expect(pingSlotForDelta(0, 0)).toBe("center");
    expect(actionForPingSlot("center")).toBe("basic-ping");
  });

  it("selects the dominant drag direction", () => {
    expect(pingSlotForDelta(0, -1)).toBe("up");
    expect(pingSlotForDelta(1, 0)).toBe("right");
    expect(pingSlotForDelta(3, -60)).toBe("up");
    expect(pingSlotForDelta(70, 12)).toBe("right");
    expect(pingSlotForDelta(-50, 8)).toBe("left");
    expect(pingSlotForDelta(4, 55)).toBe("down");
  });

  it("maps only the initial center and up slots", () => {
    expect(actionForPingSlot("up")).toBe("whip");
    expect(actionForPingSlot("right")).toBeNull();
    expect(actionForPingSlot("down")).toBeNull();
    expect(actionForPingSlot("left")).toBeNull();
  });
});
