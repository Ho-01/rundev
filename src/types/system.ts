export type BatteryState = "charging" | "discharging" | "full" | "unknown";

export type SystemStats = {
  cpuPercent: number | null;
  memoryPercent: number;
  temperatureCelsius: number | null;
  batteryPercent: number | null;
  batteryState: BatteryState | null;
  diskPercent: number | null;
  networkDownBps: number | null;
  networkUpBps: number | null;
  sequence: number;
};

export const emptySystemStats = (): SystemStats => ({
  cpuPercent: null,
  memoryPercent: 0,
  temperatureCelsius: null,
  batteryPercent: null,
  batteryState: null,
  diskPercent: null,
  networkDownBps: null,
  networkUpBps: null,
  sequence: 0
});
