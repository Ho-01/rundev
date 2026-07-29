import { Battery, Cpu, HardDrive, MemoryStick, Thermometer, Wifi } from "lucide-react";
import { buildStripItems } from "./systemStatus";
import type { SystemStats } from "../types/system";

type Props = {
  stats: SystemStats;
};

const icons = {
  cpu: Cpu,
  memory: MemoryStick,
  temperature: Thermometer,
  disk: HardDrive,
  battery: Battery,
  network: Wifi
} as const;

export function SystemStatusStrip({ stats }: Props) {
  const items = buildStripItems(stats);

  return (
    <aside className="system-strip" aria-label="시스템 상태">
      {items.map((item) => {
        const Icon = icons[item.id as keyof typeof icons] ?? Cpu;
        return (
          <button
            key={item.id}
            type="button"
            className={`system-strip-tile${item.muted ? " muted" : ""}`}
            aria-label={item.detail}
            title={item.detail}
          >
            <Icon size={14} strokeWidth={2.1} aria-hidden="true" />
            <strong>
              {item.display}
              {item.unit ? <span>{item.unit}</span> : null}
            </strong>
            <em>{item.label}</em>
          </button>
        );
      })}
    </aside>
  );
}
