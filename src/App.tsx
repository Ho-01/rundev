import { useEffect } from "react";
import {
  Bot,
  ChevronRight,
  CircleHelp,
  Clock3,
  Code2,
  Flame,
  Settings2,
  Sparkles
} from "lucide-react";
import { useDashboardStore } from "./store/dashboard";

function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  return `${hours}h ${minutes}m`;
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h2 className="section-title">{children}</h2>;
}

function Meter({ value }: { value: number }) {
  return (
    <div className="meter" aria-label={`${Math.round(value)}%`}>
      <span style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
    </div>
  );
}

export function App() {
  const { summary, character, loading, error, refresh } = useDashboardStore();

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const levelProgress = character
    ? (character.xpIntoLevel / character.xpForNextLevel) * 100
    : 0;
  const activeMinutes = Math.floor((summary?.activeSeconds ?? 0) / 60);
  const focusProgress = Math.min(100, (activeMinutes / 120) * 100);

  return (
    <main className="popover">
      <header className="runner-header">
        <div className="runner">
          <span className="runner-ear left" />
          <span className="runner-ear right" />
          <span className="runner-face">
            <i /><i />
          </span>
          <span className="runner-body" />
          <span className="runner-leg one" />
          <span className="runner-leg two" />
          <span className="runner-tail" />
        </div>
        <div className="runner-copy">
          <strong>RunDev</strong>
          <span><i className="status-dot" /> 개발 활동 대기 중</span>
        </div>
        <button className="plain-button" aria-label="설정">
          <Settings2 size={17} />
        </button>
      </header>

      <div className="divider" />

      <section className="info-section">
        <SectionTitle>개발 활동</SectionTitle>
        <div className="primary-stat">
          <span>오늘 집중</span>
          <strong>{formatDuration(summary?.activeSeconds ?? 0)}</strong>
        </div>
        <Meter value={focusProgress} />
        <div className="detail-grid">
          <div>
            <Clock3 size={13} />
            <span>목표</span>
            <strong>2h</strong>
          </div>
          <div>
            <Code2 size={13} />
            <span>활성 세션</span>
            <strong>0</strong>
          </div>
          <div>
            <Flame size={13} />
            <span>오늘 XP</span>
            <strong>{summary?.xpEarned ?? 0}</strong>
          </div>
        </div>
      </section>

      <div className="divider" />

      <section className="info-section compact">
        <SectionTitle>AI 사용</SectionTitle>
        <div className="provider-row">
          <span className="provider-icon"><Bot size={15} /></span>
          <div>
            <strong>전체 도구</strong>
            <span>오늘 감지된 AI 활동</span>
          </div>
          <b>{summary?.aiEvents ?? 0}</b>
        </div>
        <div className="mini-stats">
          <span>Codex <b>0</b></span>
          <span>Claude <b>0</b></span>
          <span>기타 <b>0</b></span>
        </div>
      </section>

      <div className="divider" />

      <section className="info-section compact">
        <SectionTitle>러너 상태</SectionTitle>
        <div className="level-row">
          <div className="level-badge">{character?.level ?? 1}</div>
          <div className="level-copy">
            <div>
              <strong>새싹 개발자</strong>
              <span>{character?.xpIntoLevel ?? 0} / {character?.xpForNextLevel ?? 100} XP</span>
            </div>
            <Meter value={levelProgress} />
          </div>
        </div>
      </section>

      <nav className="panel-menu">
        <button>
          <span><Sparkles size={15} /> 러너 변경</span>
          <ChevronRight size={14} />
        </button>
        <button>
          <span><CircleHelp size={15} /> RunDev 정보</span>
          <ChevronRight size={14} />
        </button>
      </nav>

      {loading && <div className="loading-line" />}
      {error && <p className="error-message">{error}</p>}
    </main>
  );
}
