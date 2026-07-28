import { useEffect, useState } from "react";
import {
  ChevronRight,
  CircleHelp,
  Clock3,
  Code2,
  Flame,
  Settings2,
  Sparkles
} from "lucide-react";
import { useDashboardStore } from "./store/dashboard";
import { previewClaudeConnection, previewCodexAccount } from "./services/rundev";
import type { ClaudeConnectionPreview, CodexAccountPreview } from "./types/activity";
import codingCat1 from "../src-tauri/icons/tray/coding/01.png";
import codingCat2 from "../src-tauri/icons/tray/coding/02.png";
import codingCat3 from "../src-tauri/icons/tray/coding/03.png";
import codingCat4 from "../src-tauri/icons/tray/coding/04.png";
import codingFish1 from "../src-tauri/icons/tray/coding-fish/01.png";
import codingFish2 from "../src-tauri/icons/tray/coding-fish/02.png";
import codingFish3 from "../src-tauri/icons/tray/coding-fish/03.png";
import codingFish4 from "../src-tauri/icons/tray/coding-fish/04.png";
import openAiIcon from "./assets/providers/openai.svg";
import claudeIcon from "./assets/providers/claude.svg";

const codingCatFrames = [codingCat1, codingCat2, codingCat3, codingCat4];
const codingFishFrames = [codingFish1, codingFish2, codingFish3, codingFish4];

function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  return `${hours}h ${minutes}m`;
}

function formatTokens(tokens: number) {
  return new Intl.NumberFormat("ko-KR").format(tokens);
}

function formatSyncTime(value: string | null | undefined) {
  if (!value) return "동기화 중";
  return new Intl.DateTimeFormat("ko-KR", {
    hour: "2-digit",
    minute: "2-digit"
  }).format(new Date(value));
}

function formatUsageDate(value: string | null | undefined) {
  if (!value) return "";
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
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
  const [accountPreview, setAccountPreview] = useState<CodexAccountPreview | null>(null);
  const [claudePreview, setClaudePreview] = useState<ClaudeConnectionPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [headerFrame, setHeaderFrame] = useState(0);
  const [runnerDialogOpen, setRunnerDialogOpen] = useState(false);
  const freezeRunner = new URLSearchParams(window.location.search).has("freezeRunner");
  const {
    summary,
    character,
    aiUsage,
    claudeUsage,
    aiActivity,
    runner,
    loading,
    error,
    refresh,
    connectCodex,
    disconnectCodex,
    connectClaude,
    disconnectClaude,
    selectRunner
  } = useDashboardStore();
  const runnerFrames = runner?.runnerId === "coding-fish" ? codingFishFrames : codingCatFrames;

  async function openCodexConnection() {
    setPreviewLoading(true);
    setPreviewError(null);
    try {
      setAccountPreview(await previewCodexAccount());
    } catch (connectionError) {
      setPreviewError(
        connectionError instanceof Error ? connectionError.message : String(connectionError)
      );
    } finally {
      setPreviewLoading(false);
    }
  }

  async function confirmCodexConnection() {
    await connectCodex();
    setAccountPreview(null);
  }

  async function openClaudeConnection() {
    setPreviewLoading(true);
    setPreviewError(null);
    try {
      setClaudePreview(await previewClaudeConnection());
    } catch (connectionError) {
      setPreviewError(
        connectionError instanceof Error ? connectionError.message : String(connectionError)
      );
    } finally {
      setPreviewLoading(false);
    }
  }

  async function confirmClaudeConnection() {
    await connectClaude();
    setClaudePreview(null);
  }

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 30_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    if (freezeRunner) return;
    const timer = window.setInterval(
      () => setHeaderFrame((frame) => (frame + 1) % runnerFrames.length),
      170
    );
    return () => window.clearInterval(timer);
  }, [freezeRunner, runnerFrames.length]);

  const levelProgress = character
    ? (character.xpIntoLevel / character.xpForNextLevel) * 100
    : 0;
  const activeMinutes = Math.floor((summary?.activeSeconds ?? 0) / 60);
  const focusProgress = Math.min(100, (activeMinutes / 120) * 100);
  const hasUsageDetails =
    aiUsage?.status !== "disconnected" || claudeUsage?.status !== "disconnected";

  return (
    <main className={`popover${hasUsageDetails ? " dense" : ""}`}>
      <header className="runner-header">
        <div className="runner">
          <img src={runnerFrames[headerFrame]} alt="" aria-hidden="true" />
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
            <span>활성 AI</span>
            <strong
              className="active-provider-icons"
              title={`Codex ${aiActivity?.codexActive ? "활성" : "비활성"}, Claude ${
                aiActivity?.claudeActive ? `${aiActivity.claudeActiveSessions}개 세션 활성` : "비활성"
              }`}
            >
              <img
                src={openAiIcon}
                className={aiActivity?.codexActive ? "active" : ""}
                alt="Codex"
              />
              <img
                src={claudeIcon}
                className={aiActivity?.claudeActive ? "active" : ""}
                alt="Claude Code"
              />
              <span>{aiActivity?.activeProviderCount ?? 0}</span>
            </strong>
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
        <SectionTitle>AI 사용량</SectionTitle>
        <div className="provider-row">
          <span className="provider-icon">
            <img className="openai-icon" src={openAiIcon} alt="" aria-hidden="true" />
          </span>
          <div>
            <strong>Codex</strong>
            <span>
              {aiUsage?.status === "disconnected"
                ? "연동되지 않음"
                : aiUsage?.status === "error"
                ? "연결 확인 필요"
                : aiUsage?.status === "delayed"
                ? `${aiUsage.accountLabel ?? "Codex 계정"} · 집계 지연`
                : `${aiUsage?.accountLabel ?? "Codex 계정"} · ${formatSyncTime(aiUsage?.lastSyncedAt)}`}
            </span>
          </div>
          {aiUsage?.status === "disconnected" ? (
            <button
              className="connect-button"
              type="button"
              onClick={() => void openCodexConnection()}
              disabled={loading || previewLoading}
            >
              {previewLoading ? "확인 중" : "연동"}
            </button>
          ) : (
            <b>{aiUsage?.totalTokens == null ? "—" : formatTokens(aiUsage.totalTokens)}</b>
          )}
        </div>
        {aiUsage?.status !== "disconnected" && (
          <div className="mini-stats">
            <span>
              {aiUsage?.status === "delayed"
                ? `오늘 집계 대기 중 · 최신 ${formatUsageDate(aiUsage.latestAvailableDate)}`
                : aiUsage?.totalTokens == null
                ? "오늘 데이터 없음"
                : "오늘 총 토큰"}
            </span>
            {aiUsage?.source && <span>출처 <b>Codex 계정</b></span>}
            <button
              className="disconnect-button"
              type="button"
              onClick={() => void disconnectCodex()}
              disabled={loading}
            >
              연동 해제
            </button>
          </div>
        )}
        {aiUsage?.error && (
          <p className="adapter-error" title={aiUsage.error}>
            Codex 로그인 또는 설치 상태를 확인해 주세요.
          </p>
        )}
        <div className="provider-row">
          <span className="provider-icon">
            <img src={claudeIcon} alt="" aria-hidden="true" />
          </span>
          <div>
            <strong>Claude Code</strong>
            <span>
              {claudeUsage?.status === "disconnected"
                ? "연동되지 않음"
                : claudeUsage?.status === "waiting"
                ? "첫 사용량 대기 중"
                : claudeUsage?.status === "error"
                ? "로컬 수집기 확인 필요"
                : `로컬 텔레메트리 · ${formatSyncTime(claudeUsage?.lastReceivedAt)}`}
            </span>
          </div>
          {claudeUsage?.status === "disconnected" ? (
            <button
              className="connect-button"
              type="button"
              onClick={() => void openClaudeConnection()}
              disabled={loading || previewLoading}
            >
              {previewLoading ? "확인 중" : "연동"}
            </button>
          ) : (
            <b>{formatTokens(claudeUsage?.totalTokens ?? 0)}</b>
          )}
        </div>
        {claudeUsage?.status !== "disconnected" && (
          <div className="mini-stats">
            <span>
              {claudeUsage?.status === "waiting"
                ? "Claude Code 재시작 후 첫 응답 대기"
                : "오늘 총 토큰"}
            </span>
            <span>오늘 세션 <b>{claudeUsage?.sessionCount ?? 0}개</b></span>
            <button
              className="disconnect-button"
              type="button"
              onClick={() => void disconnectClaude()}
              disabled={loading}
            >
              연동 해제
            </button>
          </div>
        )}
        {claudeUsage?.error && (
          <p className="adapter-error" title={claudeUsage.error}>
            RunDev를 재시작하거나 로컬 포트 상태를 확인해 주세요.
          </p>
        )}
        {previewError && <p className="adapter-error">{previewError}</p>}
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
        <button type="button" onClick={() => setRunnerDialogOpen(true)}>
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
      {runnerDialogOpen && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog runner-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="runner-title"
          >
            <h2 id="runner-title">러너 변경</h2>
            <p>트레이와 RunDev 화면에서 함께 달릴 러너를 선택하세요.</p>
            <div className="runner-options">
              {[
                { id: "coding-cat" as const, name: "코딩 고양이", frame: codingCat1 },
                { id: "coding-fish" as const, name: "노란 물고기", frame: codingFish1 }
              ].map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className={runner?.runnerId === option.id ? "selected" : ""}
                  onClick={() => void selectRunner(option.id).then(() => setRunnerDialogOpen(false))}
                >
                  <img src={option.frame} alt="" aria-hidden="true" />
                  <span>{option.name}</span>
                </button>
              ))}
            </div>
            <div className="dialog-actions">
              <button type="button" onClick={() => setRunnerDialogOpen(false)}>닫기</button>
            </div>
          </section>
        </div>
      )}
      {accountPreview && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="account-title"
          >
            <h2 id="account-title">이 Codex 계정과 연동할까요?</h2>
            <dl>
              <div><dt>계정</dt><dd>{accountPreview.accountLabel}</dd></div>
              <div><dt>로그인</dt><dd>{accountPreview.authType}</dd></div>
              <div><dt>요금제</dt><dd>{accountPreview.planType ?? "확인 불가"}</dd></div>
              <div><dt>인증 환경</dt><dd>{accountPreview.environment}</dd></div>
            </dl>
            <p>RunDev는 토큰 합계만 저장하며 프롬프트와 응답은 읽지 않습니다.</p>
            <div className="dialog-actions">
              <button type="button" onClick={() => setAccountPreview(null)}>취소</button>
              <button
                type="button"
                className="confirm-button"
                disabled={loading}
                onClick={() => void confirmCodexConnection()}
              >
                {loading ? "연동 중" : "이 계정 연동"}
              </button>
            </div>
          </section>
        </div>
      )}
      {claudePreview && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="claude-title"
          >
            <h2 id="claude-title">Claude Code 사용량을 연동할까요?</h2>
            <dl>
              <div><dt>설정 파일</dt><dd>{claudePreview.settingsPath}</dd></div>
              <div><dt>수집 방식</dt><dd>로컬 OpenTelemetry</dd></div>
            </dl>
            <p>
              RunDev는 토큰 합계만 로컬에 저장합니다. 프롬프트, 응답, 도구 내용 수집은
              명시적으로 끕니다. 연동 후 실행 중인 Claude Code를 재시작해야 합니다.
            </p>
            {claudePreview.hasConflicts && (
              <p className="adapter-error">
                기존 OpenTelemetry 설정을 연동 중에 대체하며, 해제할 때 안전하게 복원합니다.
              </p>
            )}
            <div className="dialog-actions">
              <button type="button" onClick={() => setClaudePreview(null)}>취소</button>
              <button
                type="button"
                className="confirm-button"
                disabled={loading}
                onClick={() => void confirmClaudeConnection()}
              >
                {loading ? "연동 중" : "Claude 연동"}
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
