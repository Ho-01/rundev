import {
  Battery,
  ChevronLeft,
  ChevronRight,
  Cpu,
  HardDrive,
  MemoryStick,
  Thermometer,
  Wifi
} from "lucide-react";
import { buildStripItems, formatRate } from "./systemStatus";
import type { SystemStats } from "../types/system";

type Props = {
  stats: SystemStats;
  expanded: boolean;
  onToggle: () => void;
};

const icons = {
  cpu: Cpu,
  memory: MemoryStick,
  temperature: Thermometer,
  disk: HardDrive,
  battery: Battery,
  network: Wifi
} as const;

function formatBytes(bytes: number | null | undefined) {
  if (bytes == null || !Number.isFinite(bytes)) return "—";
  const gib = bytes / 1024 ** 3;
  if (gib >= 100) return `${gib.toFixed(0)} GB`;
  return `${gib.toFixed(1)} GB`;
}

function formatPercent(value: number | null | undefined) {
  return value == null || !Number.isFinite(value) ? "—" : `${Math.round(value)}%`;
}

function batteryStateLabel(stats: SystemStats) {
  if (stats.batteryPercent == null) return "배터리 없음";
  if (stats.batteryState === "charging") return "충전 중";
  if (stats.batteryState === "full") return "완충";
  if (stats.batteryState === "discharging") return "사용 중";
  return "상태 확인 불가";
}

function DetailCard({
  icon: Icon,
  title,
  value,
  rows
}: {
  icon: typeof Cpu;
  title: string;
  value: string;
  rows: Array<[string, string]>;
}) {
  return (
    <article className="system-detail-card">
      <header>
        <Icon size={13} aria-hidden="true" />
        <strong>{title}</strong>
        <b>{value}</b>
      </header>
      <dl>
        {rows.map(([label, rowValue]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{rowValue}</dd>
          </div>
        ))}
      </dl>
    </article>
  );
}

export function SystemStatusStrip({ stats, expanded, onToggle }: Props) {
  const items = buildStripItems(stats);

  return (
    <aside
      className={`system-strip ${expanded ? "expanded" : "compact"}`}
      aria-label="장치 상태"
    >
      <button
        type="button"
        className="system-strip-toggle"
        aria-label={expanded ? "장치 상세 접기" : "장치 상세 펼치기"}
        aria-expanded={expanded}
        onClick={onToggle}
      >
        {expanded ? (
          <ChevronLeft size={15} aria-hidden="true" />
        ) : (
          <ChevronRight size={15} aria-hidden="true" />
        )}
      </button>

      {expanded ? (
        <div className="system-detail-list">
          <DetailCard
            icon={Cpu}
            title="CPU"
            value={formatPercent(stats.cpuPercent)}
            rows={[
              ["전체 사용률", formatPercent(stats.cpuPercent)],
              ["논리 코어", stats.logicalCpuCount ? `${stats.logicalCpuCount}개` : "—"]
            ]}
          />
          <DetailCard
            icon={MemoryStick}
            title="메모리"
            value={formatPercent(stats.memoryPercent)}
            rows={[
              ["전체", formatBytes(stats.memoryTotalBytes)],
              ["사용 중", formatBytes(stats.memoryUsedBytes)],
              ["사용 가능", formatBytes(stats.memoryAvailableBytes)]
            ]}
          />
          <DetailCard
            icon={Thermometer}
            title="온도"
            value={
              stats.temperatureCelsius == null
                ? "—"
                : `${Math.round(stats.temperatureCelsius)}°C`
            }
            rows={[
              [
                "현재",
                stats.temperatureCelsius == null
                  ? "센서 없음"
                  : `${Math.round(stats.temperatureCelsius)}°C`
              ],
              [
                "실행 후 최고",
                stats.temperatureMaxCelsius == null
                  ? "—"
                  : `${Math.round(stats.temperatureMaxCelsius)}°C`
              ]
            ]}
          />
          <DetailCard
            icon={HardDrive}
            title="저장 장치"
            value={formatPercent(stats.diskPercent)}
            rows={[
              ["전체", formatBytes(stats.diskTotalBytes)],
              ["사용 중", formatBytes(stats.diskUsedBytes)],
              ["사용 가능", formatBytes(stats.diskAvailableBytes)]
            ]}
          />
          <DetailCard
            icon={Battery}
            title="배터리"
            value={formatPercent(stats.batteryPercent)}
            rows={[["상태", batteryStateLabel(stats)]]}
          />
          <DetailCard
            icon={Wifi}
            title="네트워크"
            value={
              stats.networkDownBps == null && stats.networkUpBps == null
                ? "—"
                : formatRate((stats.networkDownBps ?? 0) + (stats.networkUpBps ?? 0))
            }
            rows={[
              [
                "다운로드",
                stats.networkDownBps == null ? "—" : formatRate(stats.networkDownBps)
              ],
              ["업로드", stats.networkUpBps == null ? "—" : formatRate(stats.networkUpBps)]
            ]}
          />
        </div>
      ) : (
        <div className="system-strip-items">
          {items.map((item) => {
            const Icon = icons[item.id as keyof typeof icons] ?? Cpu;
            return (
              <div
                key={item.id}
                className={`system-strip-tile${item.muted ? " muted" : ""}`}
                aria-label={item.detail}
                title={item.detail}
              >
                <Icon size={17} strokeWidth={2.1} aria-hidden="true" />
                <strong>
                  {item.display}
                  {item.unit ? <span>{item.unit}</span> : null}
                </strong>
                <em>{item.label}</em>
              </div>
            );
          })}
        </div>
      )}
    </aside>
  );
}
