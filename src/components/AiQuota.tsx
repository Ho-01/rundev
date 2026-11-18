import { useEffect, useState } from "react";
import type { ProviderLimits } from "../services/usageLimits";
import { useWindowVisibility } from "../hooks/useWindowVisibility";

export function resetRemaining(resetsAt: number | null, now: number) {
  if (resetsAt == null) return "초기화 시각 없음";
  const minutes = Math.ceil((resetsAt * 1000 - now) / 60_000);
  if (minutes <= 0) return "초기화 확인 중";
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor((minutes % 1440) / 60);
  const mins = minutes % 60;
  return `${days ? `${days}일 ` : ""}${hours ? `${hours}시간 ` : ""}${!days || !hours ? `${mins}분 ` : ""}후 초기화`;
}

function useClock() {
  const [now, setNow] = useState(Date.now());
  const visible = useWindowVisibility();
  useEffect(() => {
    if (!visible) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, [visible]);
  return now;
}

export function AiQuotaSummary({ quota }: { quota: ProviderLimits | undefined }) {
  const now = useClock();
  const selected = quota?.windows.reduce<ProviderLimits["windows"][number] | undefined>(
    (current, value) => !current || value.usedPercent > current.usedPercent ? value : current, undefined);
  const status = quota?.status;
  return <div className="provider-summary-value">
    <div className="provider-summary-today">
      <span>{selected?.label ?? "한도"}</span>
      <b className={status === "error" ? "needs-attention" : ""}>
        {status === "disconnected" ? "연동하기" : selected ? `${Math.round(selected.usedPercent)}%`
          : status === "error" ? "확인 필요" : status === "unavailable" ? "정보 없음" : "조회 중"}
      </b>
    </div>
    <span className="provider-summary-week">
      {status === "error" ? (selected ? "갱신 실패 · 마지막 수신값" : "한도 조회 실패") : selected ? resetRemaining(selected.resetsAt, now)
        : status === "disconnected" ? "연결 후 확인" : "계정 사용 한도"}
    </span>
  </div>;
}

export function AiQuotaDetails({ quota }: { quota: ProviderLimits | undefined }) {
  const now = useClock();
  if (!quota || quota.status === "disconnected") return null;
  return <div className="provider-quota-details">
    <dl>{quota.windows.map((w, index) => <div key={`${w.label}-${index}`}>
      <dt>{w.label} 사용률</dt><dd>{Math.round(w.usedPercent)}% · {resetRemaining(w.resetsAt, now)}</dd>
    </div>)}</dl>
    {quota.lastSyncedAt && <p>한도 갱신 {new Date(quota.lastSyncedAt).toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" })}</p>}
    {quota.error && <p className="provider-inline-error">{quota.error}</p>}
    {quota.status === "unavailable" && <p>이 계정에서 한도 정보를 제공하지 않습니다.</p>}
  </div>;
}
