import { useEffect, useState, type KeyboardEvent, type ReactNode } from "react";
import { X } from "lucide-react";
import type { ActivityStats, ActivityHistoryDay } from "../types/activity";
import { getActivityStats } from "../services/rundev";
const formatTokens = (tokens: number) => new Intl.NumberFormat("ko-KR").format(tokens);
function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours === 0) return `${minutes}분`;
  if (minutes === 0) return `${hours}시간`;
  return `${hours}시간 ${minutes}분`;
}

function formatCompactTokens(tokens: number) {
  const units = [
    { threshold: 1_000_000_000, suffix: "B" },
    { threshold: 1_000_000, suffix: "M" },
    { threshold: 1_000, suffix: "k" }
  ];
  const unit = units.find(({ threshold }) => tokens >= threshold);
  if (!unit) return formatTokens(tokens);
  const value = tokens / unit.threshold;
  return `${value.toFixed(1).replace(/\.0$/, "")}${unit.suffix}`;
}

function StatsPeriod({ title, stats, children }: { title: string; stats: ActivityStats | null; children?: ReactNode }) {
  const maxSeconds = Math.max(1, ...(stats?.hourly.map((slot) => slot.activeSeconds) ?? [1]));
  const sourceCopy = {
    focus: "집중",
    keyboard: "키보드",
    ai: "AI",
    boost: "쿠폰",
    trait: "특성",
    other: "기타"
  } as const;
  const xpSources = stats?.xpSources.filter((source) => source.amount > 0) ?? [];
  const sourceTotal = Math.max(1, xpSources.reduce((sum, source) => sum + source.amount, 0));
  const busiestSlot = stats?.hourly.reduce((busiest, slot) => slot.activeSeconds > busiest.activeSeconds ? slot : busiest, stats.hourly[0]);
  const heatmapLabel = busiestSlot?.activeSeconds
    ? `${title} 시간대별 활동. 가장 활발한 시간은 ${busiestSlot.date} ${busiestSlot.hour}시, ${formatDuration(busiestSlot.activeSeconds)}`
    : `${title} 시간대별 활동 없음`;
  const sourceLabel = xpSources.length
    ? `${title} XP 구성. ${xpSources.map((source) => `${sourceCopy[source.id]} ${source.amount} XP`).join(", ")}`
    : `${title} 획득 XP 없음`;
  const heatmap = <div className={`hourly-heatmap ${stats?.period ?? "day"}`} role="img" aria-label={heatmapLabel}>
    {stats?.hourly.map((slot) => <i key={`${slot.date}-${slot.hour}`} style={{ opacity: slot.activeSeconds === 0 ? .14 : .3 + .7 * slot.activeSeconds / maxSeconds }} title={`${slot.date} ${slot.hour}:00 · ${formatDuration(slot.activeSeconds)}`} />)}
  </div>;
  return <section className="stats-period">
    <h3>{title}</h3>
    <div className="stats-totals">
      <div><span>집중시간</span><b>{formatDuration(stats?.activeSeconds ?? 0)}</b></div>
      <div><span>획득 XP</span><b>{stats?.xpEarned ?? 0} XP</b></div>
      <div><span>키보드</span><b>{formatCompactTokens(stats?.keyboardPresses ?? 0)}</b></div>
    </div>
    <div className="xp-composition">
      <div className="xp-composition-heading"><span>XP 구성</span><b>{stats?.xpEarned ?? 0} XP</b></div>
      <div className="xp-composition-track" role="img" aria-label={sourceLabel}>
        {xpSources.map((source) => <i key={source.id} className={source.id} style={{ width: `${source.amount / sourceTotal * 100}%` }} />)}
      </div>
      <div className="xp-composition-legend">
        {xpSources.map((source) => <span key={source.id} className={source.id}><i />{sourceCopy[source.id]} <b>{source.amount}</b></span>)}
      </div>
    </div>
    {stats?.period === "history" ? children : stats?.period === "week" ? <div className="weekly-heatmap-layout">
      <div className="heatmap-weekdays" aria-hidden="true"><span>월</span><span>화</span><span>수</span><span>목</span><span>금</span><span>토</span><span>일</span></div>
      <div className="weekly-heatmap-content">
        {heatmap}
        <div className="heatmap-hours week-hours"><span>0시</span><span>6시</span><span>12시</span><span>18시</span><span>23시</span></div>
      </div>
    </div> : <>
      {heatmap}
      <div className="heatmap-hours"><span>0시</span><span>12시</span><span>23시</span></div>
    </>}
  </section>;
}

function ActivityGrass({ activityHistory, activeDays, compact = false }: {
  activityHistory: ActivityHistoryDay[];
  activeDays: number;
  compact?: boolean;
}) {
  return <div className={`activity-history${compact ? " compact-grass" : ""}`}>
    <div className="activity-history-heading">
      <span>최근 20주 활동</span>
      <strong>{activeDays}일</strong>
    </div>
    <div className="activity-grass" role="img" aria-label={`최근 20주 중 ${activeDays}일 개발 활동`}>
      {activityHistory.map((day) => <i key={day.date} className={`activity-cell intensity-${day.intensity}`} title={`${day.date} · ${formatDuration(day.activeSeconds)}`} />)}
    </div>
    <div className="activity-legend" aria-hidden="true">
      <span>적음</span>
      {[0, 1, 2, 3, 4].map((intensity) => <i key={intensity} className={`activity-cell intensity-${intensity}`} />)}
      <span>많음</span>
    </div>
  </div>;
}

function FocusAppBreakdown({ apps, title }: { apps: ActivityStats["apps"] | undefined; title: string }) {
  return <section className="stats-app-breakdown">
    <h3>{title}</h3>
    {apps?.length ? apps.map((app) => <div key={app.appName}>
      <span>{app.appName}</span>
      <strong>{formatDuration(app.activeSeconds)}</strong>
    </div>) : <p>기록된 개발 도구가 없습니다.</p>}
  </section>;
}

export function StatsPanel({ dailyStats, weeklyStats, activityHistory, activeHistoryDays, closing, onClose }: {
  dailyStats: ActivityStats | null;
  weeklyStats: ActivityStats | null;
  activityHistory: ActivityHistoryDay[];
  activeHistoryDays: number;
  closing?: boolean;
  onClose: () => void;
}) {
  const [activeTab, setActiveTab] = useState<"today" | "week" | "history">("today");
  const [historyStats, setHistoryStats] = useState<ActivityStats | null>(null);
  const [historyError, setHistoryError] = useState(false);
  const [historyAttempt, setHistoryAttempt] = useState(0);
  useEffect(() => {
    if (activeTab !== "history" || historyStats) return;
    let current = true;
    setHistoryError(false);
    void getActivityStats("history").then((stats) => {
      if (current) setHistoryStats(stats);
    }).catch(() => { if (current) setHistoryError(true); });
    return () => { current = false; };
  }, [activeTab, historyStats, historyAttempt]);
  const tabs = [
    { id: "today" as const, label: "오늘" },
    { id: "week" as const, label: "이번 주" },
    { id: "history" as const, label: "20주" }
  ];
  function moveTab(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let nextIndex = index;
    if (event.key === "ArrowRight") nextIndex = (index + 1) % tabs.length;
    else if (event.key === "ArrowLeft") nextIndex = (index - 1 + tabs.length) % tabs.length;
    else if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = tabs.length - 1;
    else return;
    event.preventDefault();
    setActiveTab(tabs[nextIndex].id);
    event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[role=tab]")[nextIndex]?.focus();
  }
  return <aside className={`stats-panel${closing ? " closing" : ""}`} aria-label="활동 통계">
    <header><strong>활동 통계</strong><button type="button" onClick={onClose} aria-label="통계 접기"><X size={14}/></button></header>
    <div className="stats-tabs" role="tablist" aria-label="통계 기간">
      {tabs.map((tab, index) => <button key={tab.id} id={`stats-tab-${tab.id}`} type="button" role="tab" aria-selected={activeTab === tab.id} aria-controls={`stats-panel-${tab.id}`} tabIndex={activeTab === tab.id ? 0 : -1} onClick={() => setActiveTab(tab.id)} onKeyDown={(event) => moveTab(event, index)}>{tab.label}</button>)}
    </div>
    {activeTab === "today" && <div className="stats-tab-panel" id="stats-panel-today" role="tabpanel" aria-labelledby="stats-tab-today"><StatsPeriod title="오늘" stats={dailyStats} /><FocusAppBreakdown apps={dailyStats?.apps} title="오늘 사용한 개발 도구" /></div>}
    {activeTab === "week" && <div className="stats-tab-panel" id="stats-panel-week" role="tabpanel" aria-labelledby="stats-tab-week"><StatsPeriod title="이번 주" stats={weeklyStats} /><FocusAppBreakdown apps={weeklyStats?.apps} title="이번 주 사용한 개발 도구" /></div>}
    {activeTab === "history" && <div className="stats-tab-panel" id="stats-panel-history" role="tabpanel" aria-labelledby="stats-tab-history">
      {historyStats ? <>
        <StatsPeriod title="최근 20주" stats={historyStats}><ActivityGrass activityHistory={activityHistory} activeDays={activeHistoryDays} compact /></StatsPeriod>
        <FocusAppBreakdown apps={historyStats.apps} title="최근 20주 사용한 개발 도구" />
      </> : historyError ? <div className="stats-history-status" role="alert">20주 통계를 불러오지 못했습니다.<button type="button" onClick={() => setHistoryAttempt(value => value + 1)}>다시 시도</button></div>
        : <p className="stats-history-status" role="status">20주 통계를 불러오는 중…</p>}
    </div>}
  </aside>;
}

