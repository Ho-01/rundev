import { useEffect, useRef, useState, type FormEvent } from "react";
import { Settings } from "lucide-react";
import { getSalarySettings, salaryAvailable, saveSalarySettings,
  type SalarySettings, type SalarySnapshot } from "../services/salary";

const initial: SalarySettings = { monthlyWon: 3_000_000, payday: 25, startMinute: 540,
  endMinute: 1080, lunchStartMinute: 720, lunchEndMinute: 780, dailyMinutes: 480,
  weekdays: [0, 1, 2, 3, 4] };
const won = (value: number) => new Intl.NumberFormat("ko-KR", {
  style: "currency", currency: "KRW", maximumFractionDigits: 0
}).format(Math.floor(value));
const time = (minutes: number) => `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
const minutes = (value: string) => { const [h, m] = value.split(":").map(Number); return h * 60 + m; };
const workStatusLabels: Record<SalarySnapshot["workStatus"], string> = {
  working: "근무 중", lunch: "점심시간", beforeWork: "출근 전", finished: "퇴근", dayOff: "휴무"
};
const remainingTime = (seconds: number) => [Math.floor(seconds / 3600), Math.floor(seconds / 60) % 60, seconds % 60]
  .map((value) => String(value).padStart(2, "0")).join(":");

import { useSalarySnapshot } from "../hooks/useSalarySnapshot";

export function SalaryCard({ active = true, snapshot: sharedSnapshot }: { active?: boolean; snapshot?: SalarySnapshot | null }) {
  const [settings, setSettings] = useState<SalarySettings | null>(null);
  const ownSnapshot = useSalarySnapshot(active && !!settings && sharedSnapshot === undefined);
  const snapshot = settings ? (sharedSnapshot === undefined ? ownSnapshot : sharedSnapshot) : null;
  const [draft, setDraft] = useState(initial);
  const [editing, setEditing] = useState(false);
  const [ready, setReady] = useState(false);
  const [settingsAttempt, setSettingsAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [formError, setFormError] = useState("");
  const dialog = useRef<HTMLElement>(null);
  const settingsButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    let active = true;
    let pending = false;
    let loaded = false;
    let retryTimer: number | undefined;
    async function load() {
      if (!active || pending || loaded) return;
      window.clearTimeout(retryTimer);
      pending = true;
      try {
        const value = await getSalarySettings();
        if (active) {
          loaded = true;
          setSettings(value); setDraft(value ?? initial); setReady(true); setError("");
        }
      } catch {
        if (active) {
          setError("급여 설정을 불러오지 못했습니다. 자동으로 다시 시도하고 있어요.");
          retryTimer = window.setTimeout(() => void load(), 2_000);
        }
      } finally { pending = false; }
    }
    const retryOnFocus = () => void load();
    void load();
    window.addEventListener("focus", retryOnFocus);
    return () => { active = false; window.clearTimeout(retryTimer); window.removeEventListener("focus", retryOnFocus); };
  }, [settingsAttempt]);

  useEffect(() => {
    if (editing) dialog.current?.querySelector<HTMLInputElement>("input")?.focus();
  }, [editing]);

  function close() { if (!busy) { setEditing(false); settingsButton.current?.focus(); } }
  async function persist(value: SalarySettings | null) {
    setBusy(true); setFormError("");
    try {
      await saveSalarySettings(value);
      setSettings(value); setEditing(false); settingsButton.current?.focus();
    } catch (reason) { setFormError(String(reason)); }
    finally { setBusy(false); }
  }
  function submit(event: FormEvent) { event.preventDefault(); void persist(draft); }

  return <section className="salary-card" aria-label="실시간 월급">
    <header><span>실시간 월급 <small>예상</small></span><div className="salary-header-tools">
      {snapshot && <span className={`salary-state ${snapshot.workStatus}`}>● {workStatusLabels[snapshot.workStatus]}</span>}
      <button ref={settingsButton} type="button"
      aria-label="급여 설정" title="급여 설정"
      disabled={!ready} onClick={() => { setDraft(settings ?? initial); setFormError(""); setEditing(true); }}><Settings size={13} aria-hidden="true" /></button></div></header>
    {snapshot ? <>
      <div className="salary-total-row" title={`${snapshot.cycleStart} ~ ${snapshot.nextPayday} · 근무 ${snapshot.workdays}일`}>
        <div className="salary-cycle"><span className="salary-caption">이번 급여 주기 누적</span><strong className="salary-amount">{won(snapshot.cycleWon)}</strong></div>
        <div className="salary-today"><span className="salary-caption">오늘</span><strong>{won(snapshot.todayWon)}</strong></div>
      </div>
      <div className="salary-details"><span>초당 <b>+{snapshot.wonPerSecond.toFixed(2)}원</b></span>
        {snapshot.secondsUntilEnd != null && <span className="salary-countdown"><span>퇴근까지</span> <b>{remainingTime(snapshot.secondsUntilEnd)}</b></span>}
      </div>
    </> : <p className="salary-empty">{!ready ? "급여 설정을 불러오는 중…" : settings ? "예상 급여를 계산하고 있어요…" : "급여와 근무시간을 설정해 주세요."}</p>}
    {error && !editing && <p role="alert" className="error-message">{error}</p>}
    {!ready && error && <button className="salary-retry" type="button" onClick={() => {
      setError(""); setSettingsAttempt((attempt) => attempt + 1);
    }}>다시 시도</button>}
    {editing && <div className="dialog-backdrop" role="presentation">
      <section ref={dialog} className="account-dialog salary-dialog" role="dialog" aria-modal="true" aria-labelledby="salary-title"
        onKeyDown={(event) => {
          if (event.key === "Escape") { event.stopPropagation(); close(); }
          if (event.key === "Tab") {
            const controls = dialog.current?.querySelectorAll<HTMLElement>("input:not(:disabled), button:not(:disabled)");
            if (!controls?.length) return;
            const first = controls[0], last = controls[controls.length - 1];
            if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
            if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
          }
        }}>
        <h2 id="salary-title">급여 설정</h2>
        <p>입력한 금액은 이 기기에만 저장됩니다. 로컬 DB는 암호화되지 않습니다.</p>
        {!salaryAvailable() && <p role="status">설정 저장과 실시간 계산은 데스크톱 앱에서 사용할 수 있습니다.</p>}
        <form onSubmit={submit}>
          <fieldset disabled={busy} className="salary-fields">
            <label>월 급여 (원)<input type="number" min="1" max="1000000000000" step="1" required value={draft.monthlyWon}
              onChange={(e) => setDraft({ ...draft, monthlyWon: Number(e.target.value) })} /></label>
            <label>매월 급여일<input type="number" min="1" max="31" required value={draft.payday}
              onChange={(e) => setDraft({ ...draft, payday: Number(e.target.value) })} /></label>
            {([
              ["출근", "startMinute"], ["퇴근", "endMinute"],
              ["점심 시작", "lunchStartMinute"], ["점심 종료", "lunchEndMinute"]
            ] as const).map(([label, key]) => <label key={key}>{label}<input type="time" required value={time(draft[key])}
              onChange={(e) => { if (e.target.value) setDraft({ ...draft, [key]: minutes(e.target.value) }); }} /></label>)}
            <label>하루 총근무시간 (분)<input type="number" min="1" max="1439" required value={draft.dailyMinutes}
              onChange={(e) => setDraft({ ...draft, dailyMinutes: Number(e.target.value) })} /></label>
            <span className="salary-hours">{(draft.dailyMinutes / 60).toFixed(1)}시간 · 점심 제외</span>
            <div className="salary-weekdays" role="group" aria-label="근무 요일">{["월", "화", "수", "목", "금", "토", "일"].map((day, i) =>
              <label key={day}><input type="checkbox" checked={draft.weekdays.includes(i)} onChange={(e) => setDraft({ ...draft,
                weekdays: e.target.checked ? [...draft.weekdays, i] : draft.weekdays.filter((d) => d !== i) })} />{day}</label>)}</div>
          </fieldset>
          <p className="salary-help">직전 급여일~다음 급여일의 근무시간에 월 급여를 나눕니다. 급여일 0시에 초기화하며, 없는 날짜는 말일로 처리합니다. 점심이 없으면 시작·종료를 같게 입력하세요.</p>
          <p className="salary-help">공휴일·휴가·야간근무·세금은 반영하지 않는 예상액입니다. 설정 변경은 현재 주기 전체에 적용됩니다. 세전·세후 중 보고 싶은 금액을 입력하세요.</p>
          {formError && <p role="alert" className="error-message">{formError}</p>}
          <div className="dialog-actions">
            {settings && <button type="button" disabled={busy} onClick={() => void persist(null)}>설정 삭제</button>}
            <button type="button" disabled={busy} onClick={close}>닫기</button>
            <button type="submit" disabled={busy || !salaryAvailable()}>{busy ? "저장 중…" : "저장"}</button>
          </div>
        </form>
      </section>
    </div>}
  </section>;
}
