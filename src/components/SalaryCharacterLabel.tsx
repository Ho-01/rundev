import type { SalarySnapshot } from "../services/salary";

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
