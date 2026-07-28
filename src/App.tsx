import { useEffect, useState } from "react";
import {
  Clock3,
  Cpu,
  Crown,
  Bug,
  BadgeCheck,
  Code2,
  Hammer,
  Info,
  Keyboard,
  Network,
  Settings2,
  Sprout,
  SquareTerminal,
  Workflow
} from "lucide-react";
import { useDashboardStore } from "./store/dashboard";
import {
  openKeyboardPermissionSettings,
  previewClaudeConnection,
  previewCodexAccount,
  subscribeFocusActivity,
  subscribeKeyboardActivity
} from "./services/rundev";
import type {
  ClaudeConnectionPreview,
  CodexAccountPreview
} from "./types/activity";
import { runnerFramesById, runnerOptions } from "./assets/runners";
import openAiIcon from "./assets/providers/openai.svg";
import claudeIcon from "./assets/providers/claude.svg";
import packageJson from "../package.json";
import {
  getLevelTier,
  levelTiers,
  type LevelTierId
} from "./domain/level";

function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours === 0) return `${minutes}분`;
  if (minutes === 0) return `${hours}시간`;
  return `${hours}시간 ${minutes}분`;
}

function formatTokens(tokens: number) {
  return new Intl.NumberFormat("ko-KR").format(tokens);
}

function formatRemainingMinutes(seconds: number) {
  return `${Math.max(1, Math.ceil(seconds / 60))}분`;
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

function TierIcon({ tierId, size = 13 }: { tierId: LevelTierId; size?: number }) {
  const props = { size, "aria-hidden": true as const };
  switch (tierId) {
    case "rookie":
      return <Hammer {...props} />;
    case "maker":
      return <Code2 {...props} />;
    case "debugger":
      return <Bug {...props} />;
    case "hacker":
      return <SquareTerminal {...props} />;
    case "systems":
      return <Network {...props} />;
    case "architect":
      return <Cpu {...props} />;
    case "lead":
      return <Workflow {...props} />;
    case "master":
      return <BadgeCheck {...props} />;
    case "legend":
      return <Crown {...props} />;
    default:
      return <Sprout {...props} />;
  }
}

function LevelStatus({
  level,
  xpIntoLevel,
  xpForNextLevel
}: {
  level: number;
  xpIntoLevel: number;
  xpForNextLevel: number;
}) {
  const tier = getLevelTier(level);
  const progress = (xpIntoLevel / xpForNextLevel) * 100;

  return (
    <div className={`level-row tier-${tier.id}`}>
      <div className="level-badge" aria-label={`${tier.name} 엠블럼`}>
        <TierIcon tierId={tier.id} size={21} />
      </div>
      <div className="level-copy">
        <div>
          <strong className="tier-name">
            <TierIcon tierId={tier.id} />
            {tier.name}
            <span className="level-label">Lv. {level}</span>
          </strong>
          <span>{xpIntoLevel} / {xpForNextLevel} XP</span>
        </div>
        <Meter value={progress} />
      </div>
    </div>
  );
}

function LevelShowcase() {
  return (
    <main className="level-showcase">
      <header>
        <span>RunDev Level System</span>
        <h1>개발자 등급</h1>
        <p>활동 XP가 쌓일수록 배지와 등급이 성장합니다.</p>
      </header>
      <div className="level-showcase-list">
        {levelTiers.map((tier) => (
          <section key={tier.id}>
            <LevelStatus
              level={tier.minLevel}
              xpIntoLevel={tier.id === "legend" ? 100 : 40}
              xpForNextLevel={100}
            />
            <p>
              Lv.{tier.minLevel}
              {tier.maxLevel ? `–${tier.maxLevel}` : "+"}
            </p>
          </section>
        ))}
      </div>
    </main>
  );
}

export function App() {
  const [accountPreview, setAccountPreview] = useState<CodexAccountPreview | null>(null);
  const [claudePreview, setClaudePreview] = useState<ClaudeConnectionPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [headerFrame, setHeaderFrame] = useState(0);
  const [runnerDialogOpen, setRunnerDialogOpen] = useState(false);
  const [infoDialogOpen, setInfoDialogOpen] = useState(false);
  const freezeRunner = new URLSearchParams(window.location.search).has("freezeRunner");
  const showLevelShowcase = new URLSearchParams(window.location.search).has("levelShowcase");
  const {
    summary,
    focus,
    activityHistory,
    character,
    aiUsage,
    claudeUsage,
    keyboard,
    runner,
    loading,
    error,
    refresh,
    connectCodex,
    disconnectCodex,
    connectClaude,
    disconnectClaude,
    selectRunner,
    setKeyboardActivity,
    setFocusActivity
  } = useDashboardStore();
  const runnerFrames = runnerFramesById[runner?.runnerId ?? "coding-cat"];

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
    const timer = window.setInterval(() => void refresh(), 5_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void subscribeKeyboardActivity(setKeyboardActivity).then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  }, [setKeyboardActivity]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void subscribeFocusActivity(setFocusActivity).then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  }, [setFocusActivity]);

  useEffect(() => {
    if (freezeRunner) return;
    const timer = window.setInterval(
      () => setHeaderFrame((frame) => (frame + 1) % runnerFrames.length),
      170
    );
    return () => window.clearInterval(timer);
  }, [freezeRunner, runnerFrames.length]);

  const focusRewardProgress = ((summary?.activeSeconds ?? 0) % 1_800) / 18;
  const focusRewardRemaining =
    1_800 - ((summary?.activeSeconds ?? 0) % 1_800);
  const keyboardProgress = ((keyboard?.pressCount ?? 0) % 2_000) / 20;
  const keyboardRemaining = Math.max(
    0,
    (keyboard?.nextRewardAt ?? 2_000) - (keyboard?.pressCount ?? 0)
  );
  const hasUsageDetails =
    aiUsage?.status !== "disconnected" || claudeUsage?.status !== "disconnected";
  const activeHistoryDays = activityHistory.filter((day) => day.activeSeconds > 0).length;

  if (showLevelShowcase) {
    return <LevelShowcase />;
  }

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
        <div className="header-actions">
          <button
            className="plain-button"
            type="button"
            aria-label="RunDev 정보"
            onClick={() => setInfoDialogOpen(true)}
          >
            <Info size={17} />
          </button>
          <button
            className="plain-button"
            type="button"
            aria-label="개발자 변경"
            onClick={() => setRunnerDialogOpen(true)}
          >
            <Settings2 size={17} />
          </button>
        </div>
      </header>

      <div className="divider" />

      <section className="info-section">
        <SectionTitle>개발 활동</SectionTitle>
        <div className="primary-stat">
          <div className="primary-label">
            <Clock3 size={14} />
            <span>개발 도구 노려본 시간</span>
          </div>
          <strong>{formatDuration(summary?.activeSeconds ?? 0)}</strong>
        </div>
        <div className="keyboard-progress focus-reward">
          <Meter value={focusRewardProgress} />
          <span>다음 +10 XP까지 {formatRemainingMinutes(focusRewardRemaining)}</span>
        </div>
        <details className="focus-apps">
          <summary>
            <span>마지막으로 본 도구</span>
            <strong>{focus?.lastAppName ?? "아직 없음"}</strong>
          </summary>
          <div className="focus-app-list">
            <p className="focus-app-list-title">오늘 앱별 시간</p>
            {focus?.apps.length ? (
              focus.apps.map((app) => (
                <div key={app.appName}>
                  <span>{app.appName}</span>
                  <strong>{formatDuration(app.activeSeconds)}</strong>
                </div>
              ))
            ) : (
              <p className="focus-app-empty">오늘 기록된 개발 도구가 없습니다.</p>
            )}
          </div>
        </details>
        <div className="keyboard-stat">
          <div>
            <Keyboard size={14} />
            <span>오늘 두드린 키보드</span>
          </div>
          <strong>{formatTokens(keyboard?.pressCount ?? 0)}회</strong>
        </div>
        {keyboard?.permissionRequired ? (
          <div className="keyboard-permission">
            <span>입력 내용은 저장하지 않고 횟수만 집계합니다.</span>
            <button
              type="button"
              onClick={() => void openKeyboardPermissionSettings()}
            >
              설정 열기
            </button>
          </div>
        ) : keyboard?.status === "error" || keyboard?.status === "unavailable" ? (
          <div className="keyboard-permission">
            <span>이 환경에서는 키보드 횟수를 측정할 수 없습니다.</span>
          </div>
        ) : keyboard?.status === "starting" ? (
          <div className="keyboard-permission">
            <span>키보드 측정을 준비하고 있습니다.</span>
          </div>
        ) : (
          <div className="keyboard-progress">
            <Meter value={keyboardProgress} />
            <span>
              다음 +10 XP까지 {formatTokens(keyboardRemaining)}회
            </span>
          </div>
        )}
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
        <SectionTitle>개발자 상태</SectionTitle>
        <LevelStatus
          level={character?.level ?? 1}
          xpIntoLevel={character?.xpIntoLevel ?? 0}
          xpForNextLevel={character?.xpForNextLevel ?? 100}
        />
        <div className="activity-history">
          <div className="activity-history-heading">
            <span>최근 20주 활동</span>
            <strong>{activeHistoryDays}일</strong>
          </div>
          <div
            className="activity-grass"
            role="img"
            aria-label={`최근 20주 중 ${activeHistoryDays}일 개발 활동`}
          >
            {activityHistory.map((day) => (
              <i
                key={day.date}
                className={`activity-cell intensity-${day.intensity}`}
                title={`${day.date} · ${formatDuration(day.activeSeconds)}`}
              />
            ))}
          </div>
          <div className="activity-legend" aria-hidden="true">
            <span>적음</span>
            {[0, 1, 2, 3, 4].map((intensity) => (
              <i key={intensity} className={`activity-cell intensity-${intensity}`} />
            ))}
            <span>많음</span>
          </div>
        </div>
      </section>

      {loading && <div className="loading-line" />}
      {error && <p className="error-message">{error}</p>}
      {infoDialogOpen && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog app-info-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="app-info-title"
          >
            <div className="app-info-mark"><Info size={20} /></div>
            <h2 id="app-info-title">RunDev 정보</h2>
            <p>개발 활동을 기록하고 성장으로 보여주는 로컬 우선 트레이 앱입니다.</p>
            <dl>
              <div><dt>버전</dt><dd>v{packageJson.version}</dd></div>
              <div><dt>기술</dt><dd>Tauri 2 · React · Rust · SQLite</dd></div>
              <div><dt>데이터</dt><dd>이 기기에만 저장</dd></div>
            </dl>
            <p>프롬프트, 소스 코드, 키 입력 내용은 저장하지 않습니다.</p>
            <div className="dialog-actions">
              <button type="button" onClick={() => setInfoDialogOpen(false)}>닫기</button>
            </div>
          </section>
        </div>
      )}
      {runnerDialogOpen && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog runner-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="runner-title"
          >
            <h2 id="runner-title">개발자 변경</h2>
            <p>트레이와 RunDev 화면에 표시할 개발자 캐릭터를 선택하세요.</p>
            <div className="runner-options">
              {runnerOptions.map((option) => (
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
