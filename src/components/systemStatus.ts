import type { SystemStats } from "../types/system";

function clampPercent(value: number | null | undefined) {
  if (value == null || Number.isNaN(value)) return null;
  return Math.max(0, Math.min(100, value));
}

export function formatRate(bps: number) {
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
}

function formatCompactRate(bps: number) {
  if (bps < 1024) return `${Math.round(bps)}`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(bps < 10_240 ? 1 : 0)}K`;
  return `${(bps / (1024 * 1024)).toFixed(bps < 10_485_760 ? 1 : 0)}M`;
}

function batteryLabel(stats: SystemStats) {
  if (stats.batteryPercent == null) return "배터리 없음";
  const state =
    stats.batteryState === "charging"
      ? "충전 중"
      : stats.batteryState === "full"
        ? "완충"
        : stats.batteryState === "discharging"
          ? "방전"
          : "상태 불명";
  return `배터리 ${Math.round(stats.batteryPercent)}% · ${state}`;
}

export type StripItem = {
  id: string;
  label: string;
  detail: string;
  display: string;
  unit: "" | "%" | "°";
  muted?: boolean;
};

export function buildStripItems(stats: SystemStats): StripItem[] {
  const cpu = clampPercent(stats.cpuPercent);
  const memory = clampPercent(stats.memoryPercent) ?? 0;
  const disk = clampPercent(stats.diskPercent);
  const battery = clampPercent(stats.batteryPercent);
  const temperature =
    stats.temperatureCelsius != null && Number.isFinite(stats.temperatureCelsius)
      ? stats.temperatureCelsius
      : null;
  const networkReady = stats.networkDownBps != null || stats.networkUpBps != null;
  const networkTotal = (stats.networkDownBps ?? 0) + (stats.networkUpBps ?? 0);

  return [
    {
      id: "cpu",
      label: "CPU",
      detail: cpu == null ? "측정 준비 중" : `CPU ${Math.round(cpu)}%`,
      display: cpu == null ? "—" : `${Math.round(cpu)}`,
      unit: cpu == null ? "" : "%"
    },
    {
      id: "memory",
      label: "메모리",
      detail: `메모리 ${Math.round(memory)}%`,
      display: `${Math.round(memory)}`,
      unit: "%"
    },
    {
      id: "temperature",
      label: "온도",
      detail:
        temperature == null
          ? "온도 센서 없음"
          : `온도 ${Math.round(temperature)}°C`,
      display: temperature == null ? "—" : `${Math.round(temperature)}`,
      unit: temperature == null ? "" : "°",
      muted: temperature == null
    },
    {
      id: "disk",
      label: "디스크",
      detail: disk == null ? "디스크 확인 불가" : `디스크 ${Math.round(disk)}%`,
      display: disk == null ? "—" : `${Math.round(disk)}`,
      unit: disk == null ? "" : "%"
    },
    {
      id: "battery",
      label: "배터리",
      detail: batteryLabel(stats),
      display: battery == null ? "—" : `${Math.round(battery)}`,
      unit: battery == null ? "" : "%",
      muted: battery == null
    },
    {
      id: "network",
      label: "네트워크",
      detail: networkReady
        ? `네트워크 ↓ ${formatRate(stats.networkDownBps ?? 0)} · ↑ ${formatRate(stats.networkUpBps ?? 0)}`
        : "네트워크 측정 준비 중",
      display: networkReady ? formatCompactRate(networkTotal) : "—",
      unit: ""
    }
  ];
}
