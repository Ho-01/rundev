export type BatteryState = "charging" | "discharging" | "full" | "unknown";

export type SystemStats = {
  cpuPercent: number | null;
  logicalCpuCount: number;
  memoryPercent: number;
  memoryTotalBytes: number;
  memoryUsedBytes: number;
  memoryAvailableBytes: number;
  temperatureCelsius: number | null;
  temperatureMaxCelsius: number | null;
  batteryPercent: number | null;
  batteryState: BatteryState | null;
  diskPercent: number | null;
  diskTotalBytes: number | null;
  diskUsedBytes: number | null;
  diskAvailableBytes: number | null;
  networkDownBps: number | null;
  networkUpBps: number | null;
  sequence: number;
};

export const emptySystemStats = (): SystemStats => ({
  cpuPercent: null,
  logicalCpuCount: 0,
  memoryPercent: 0,
  memoryTotalBytes: 0,
  memoryUsedBytes: 0,
  memoryAvailableBytes: 0,
  temperatureCelsius: null,
  temperatureMaxCelsius: null,
  batteryPercent: null,
  batteryState: null,
  diskPercent: null,
  diskTotalBytes: null,
  diskUsedBytes: null,
  diskAvailableBytes: null,
  networkDownBps: null,
  networkUpBps: null,
  sequence: 0
});
