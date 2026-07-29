import { describe, expect, it } from "vitest";
import { buildStripItems } from "./systemStatus";
import { emptySystemStats } from "../types/system";

describe("system status strip", () => {
  it("orders temperature between memory and disk", () => {
    const items = buildStripItems({
      ...emptySystemStats(),
      memoryPercent: 40,
      temperatureCelsius: 61,
      diskPercent: 55,
      sequence: 2
    });
    expect(items.map((item) => item.id)).toEqual([
      "cpu",
      "memory",
      "temperature",
      "disk",
      "battery",
      "network"
    ]);
    expect(items.find((item) => item.id === "disk")?.label).toBe("디스크");
    expect(items.find((item) => item.id === "temperature")?.display).toBe("61");
  });

  it("keeps battery and temperature muted when unavailable", () => {
    const items = buildStripItems({
      ...emptySystemStats(),
      memoryPercent: 40,
      diskPercent: 55,
      sequence: 2
    });
    expect(items.find((item) => item.id === "battery")?.muted).toBe(true);
    expect(items.find((item) => item.id === "temperature")?.display).toBe("—");
  });
});
