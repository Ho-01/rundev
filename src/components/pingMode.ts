export type PingSlot = "center" | "up" | "right" | "down" | "left";
export type PingActionId = "basic-ping" | "whip";

export type PingSlotDefinition = {
  slot: PingSlot;
  actionId: PingActionId | null;
  label: string;
};

export const DEFAULT_PING_SLOTS: readonly PingSlotDefinition[] = [
  { slot: "center", actionId: "basic-ping", label: "기본 핑" },
  { slot: "up", actionId: "whip", label: "채찍" },
  { slot: "right", actionId: null, label: "비어 있음" },
  { slot: "down", actionId: null, label: "비어 있음" },
  { slot: "left", actionId: null, label: "비어 있음" }
];

export const PING_DRAG_THRESHOLD = 10;

export function pingSlotForDelta(deltaX: number, deltaY: number): PingSlot {
  if (deltaX === 0 && deltaY === 0) return "center";
  if (Math.abs(deltaX) > Math.abs(deltaY)) return deltaX > 0 ? "right" : "left";
  return deltaY > 0 ? "down" : "up";
}

export function actionForPingSlot(slot: PingSlot): PingActionId | null {
  return DEFAULT_PING_SLOTS.find((definition) => definition.slot === slot)?.actionId ?? null;
}
