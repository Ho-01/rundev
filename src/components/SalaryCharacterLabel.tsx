import { useEffect, useState, type CSSProperties } from "react";
import { subscribeSalaryAnchor, type SalaryAnchor, type SalarySnapshot } from "../services/salary";
import { useSalarySnapshot } from "../hooks/useSalarySnapshot";

export function SalaryCharacterLabel({ snapshot, bubble = false }: {
  snapshot: SalarySnapshot | null;
  bubble?: boolean;
}) {
  if (!snapshot) return null;
  const rate = snapshot.working ? snapshot.wonPerSecond : 0;
  const today = new Intl.NumberFormat("ko-KR").format(Math.floor(snapshot.todayWon));
  const amount = new Intl.NumberFormat("ko-KR", { maximumFractionDigits: 2 }).format(rate);
  return <div className={`salary-character-label${bubble ? " with-bubble" : ""}`}>
    {bubble && <span className="salary-character-bubble">{snapshot.working
      ? `오늘 ${today}원째 버는 중!` : `오늘 ${today}원 벌었어요! 지금은 쉬는 중`}</span>}
    {snapshot.working && rate > 0 && <span key={amount} className="salary-character-gain"
      aria-label={`초당 ${amount}원 증가`}>+₩{amount}</span>}
  </div>;
}

export function CharacterSalaryWindow() {
  const snapshot = useSalarySnapshot();
  const [anchor, setAnchor] = useState<SalaryAnchor | null>(null);
  useEffect(() => {
    let active = true;
    let unlisten = () => {};
    void subscribeSalaryAnchor((value) => { if (active) setAnchor(value); }).then((off) => {
      if (active) unlisten = off; else off();
    }).catch(() => {});
    return () => { active = false; unlisten(); };
  }, []);
  const bubbleScale = anchor ? Math.max(0.75, Math.min(4, anchor.characterSize / 48)) : 1;
  return <main className="character-salary-window"
    data-bubble-below={anchor != null && anchor.y < anchor.characterSize / 2 + 85 * bubbleScale}
    style={anchor ? {
      "--salary-anchor-x": `${anchor.x}px`, "--salary-anchor-y": `${anchor.y}px`,
      "--salary-character-size": `${anchor.characterSize}px`, "--salary-bubble-scale": bubbleScale
    } as CSSProperties : undefined}>
    {anchor && <SalaryCharacterLabel snapshot={snapshot} bubble />}
  </main>;
}
