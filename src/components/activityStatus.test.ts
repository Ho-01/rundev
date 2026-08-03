import { describe, expect, it, vi } from "vitest";
import {
  AWAY_MESSAGES,
  DEVELOPMENT_MESSAGES,
  buildActivityStatus
} from "./activityStatus";

describe("activity status", () => {
  it("keeps the app-name particle out of message candidates", () => {
    expect([...DEVELOPMENT_MESSAGES, ...AWAY_MESSAGES]).not.toEqual(
      expect.arrayContaining([expect.stringContaining("에서")])
    );
  });

  it("shows a development message for an active developer app", () => {
    vi.spyOn(Math, "random").mockReturnValue(0);
    expect(
      buildActivityStatus({
        activeSeconds: 10,
        focused: true,
        active: true,
        appName: "VS Code"
      })
    ).toEqual({
      tone: "development",
      appName: "VS Code",
      message: DEVELOPMENT_MESSAGES[0]
    });
  });

  it("shows a separate away message for an active non-developer app", () => {
    vi.spyOn(Math, "random").mockReturnValue(0);
    expect(
      buildActivityStatus({
        activeSeconds: 10,
        focused: false,
        active: true,
        appName: "Safari"
      })
    ).toEqual({ tone: "away", appName: "Safari", message: AWAY_MESSAGES[0] });
  });

  it("falls back to waiting while idle", () => {
    expect(
      buildActivityStatus({
        activeSeconds: 10,
        focused: false,
        active: false,
        appName: "Safari"
      })
    ).toEqual({ tone: "idle", appName: null, message: "개발 활동 대기 중" });
  });
});
