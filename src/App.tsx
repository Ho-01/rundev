import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  Workflow,
  Zap
} from "lucide-react";
import { useDashboardStore } from "./store/dashboard";
import {
  openDiagnosticsFolder,
  previewClaudeConnection,
  previewCodexAccount,
  grantCursorUsageConsent,
  previewCursorAccount,
  recordWhip,
  getWhipStats,
  getXpBoostStatus,
  previewXpCoupon,
  redeemXpCoupon,
  resetKeyboardPermissionAndRelaunch,
  WHIP_COOLDOWN_MS,
  subscribeFocusActivity,
  subscribeKeyboardActivity,
  setSystemPanelExpanded as resizeSystemPanel,
  subscribeSystemStats,
  subscribeUsageRefreshed
} from "./services/rundev";
import {
  UPDATE_CHECK_INTERVAL_MS,
  checkForAppUpdate,
  downloadAndInstallAppUpdate,
  getCachedUpdateStatus,
  notifyIfUpdateAvailable,
  type UpdateStatus
} from "./services/updater";
import { SystemStatusStrip } from "./components/SystemStatusStrip";
import { buildActivityStatus } from "./components/activityStatus";
import { PingModeOverlay } from "./components/PingModeOverlay";
import {
  WhipCrackOverlay,
  type WhipCrackApi
} from "./components/WhipCrackOverlay";
import type {
  ClaudeConnectionPreview,
  CodexAccountPreview,
  CursorAccountPreview,
  AiWeeklyXp,
  WhipStats,
  XpBoostStatus,
  XpCouponPreview
} from "./types/activity";
import { runnerFramesById, runnerOptions } from "./assets/runners";
import openAiIcon from "./assets/providers/openai.svg";

const KEYBOARD_PERMISSION_REPAIR_PENDING_KEY =
  "keyboard.macos.permissionRepairPending";
import claudeIcon from "./assets/providers/claude.svg";
import cursorIcon from "./assets/providers/cursor.svg";
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

function formatBoostRemaining(milliseconds: number) {
  const totalMinutes = Math.max(0, Math.ceil(milliseconds / 60_000));
  if (totalMinutes >= 24 * 60) {
    const days = Math.floor(totalMinutes / (24 * 60));
    const hours = Math.ceil((totalMinutes % (24 * 60)) / 60);
    return hours > 0 ? `${days}일 ${hours}시간` : `${days}일`;
  }
  if (totalMinutes >= 60) {
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;
    return minutes > 0 ? `${hours}시간 ${minutes}분` : `${hours}시간`;
  }
  return `${totalMinutes}분`;
}

function formatTokens(tokens: number) {
  return new Intl.NumberFormat("ko-KR").format(tokens);
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

function formatRequestUnits(requests: number | null | undefined) {
  if (requests == null) return "—";
  return new Intl.NumberFormat("ko-KR", {
    maximumFractionDigits: requests % 1 === 0 ? 0 : 1
  }).format(requests);
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

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h2 className="section-title">{children}</h2>;
}

function ProviderInlineDetails({
  rows,
  error,
  children
}: {
  rows: Array<{ label: string; value: React.ReactNode }>;
  error?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="provider-inline-detail">
      <dl>
        {rows.map((row) => (
          <div key={row.label}>
            <dt>{row.label}</dt>
            <dd>{row.value}</dd>
          </div>
        ))}
      </dl>
      {error && <p className="provider-inline-error">{error}</p>}
      <div className="provider-inline-actions">{children}</div>
    </div>
  );
}

function AiWeeklyXpProgress({ progress }: { progress: AiWeeklyXp | null }) {
  const maxXp = progress?.maxXp ?? 210;
  const providers = [
    { id: "codex", label: "Codex", xp: progress?.codexXp ?? 0 },
    { id: "claude", label: "Claude", xp: progress?.claudeXp ?? 0 },
    { id: "cursor", label: "Cursor", xp: progress?.cursorXp ?? 0 }
  ];

  return (
    <div className="ai-weekly-xp">
      <div className="ai-weekly-xp-heading">
        <div>
          <strong>이번 주 AI 토큰 사용 XP</strong>
          <span className="reward-rule">
            <BadgeCheck size={11} />
            사용량 마일스톤마다 <b>+10 XP</b>
          </span>
        </div>
        <b>{progress?.earnedXp ?? 0} / {maxXp} XP</b>
      </div>
      <div
        className="ai-weekly-xp-track"
        role="progressbar"
        aria-label="이번 주 AI 토큰 사용 XP"
        aria-valuemin={0}
        aria-valuemax={maxXp}
        aria-valuenow={progress?.earnedXp ?? 0}
      >
        {providers.map((provider) => (
          <span
            key={provider.id}
            className={`ai-weekly-xp-segment ${provider.id}`}
            style={{ width: `${(provider.xp / maxXp) * 100}%` }}
          />
        ))}
      </div>
      <div className="ai-weekly-xp-legend">
        {providers.map((provider) => (
          <span key={provider.id} className={provider.id}>
            <i />{provider.label} <b>{provider.xp} XP</b>
          </span>
        ))}
      </div>
    </div>
  );
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
  const [cursorConsentOpen, setCursorConsentOpen] = useState(false);
  const [cursorPreview, setCursorPreview] = useState<CursorAccountPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [headerFrame, setHeaderFrame] = useState(0);
  const [runnerDialogOpen, setRunnerDialogOpen] = useState(false);
  const [infoDialogOpen, setInfoDialogOpen] = useState(false);
  const [couponDialogOpen, setCouponDialogOpen] = useState(false);
  const [couponCode, setCouponCode] = useState("");
  const [couponPreview, setCouponPreview] = useState<XpCouponPreview | null>(null);
  const [couponError, setCouponError] = useState<string | null>(null);
  const [couponLoading, setCouponLoading] = useState(false);
  const [xpBoost, setXpBoost] = useState<XpBoostStatus | null>(null);
  const [boostClock, setBoostClock] = useState(Date.now());
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>(() =>
    getCachedUpdateStatus(packageJson.version)
  );
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [permissionRepairing, setPermissionRepairing] = useState(false);
  const [permissionRepairError, setPermissionRepairError] = useState<string | null>(null);
  const [permissionRepairPending, setPermissionRepairPending] = useState(
    () => localStorage.getItem(KEYBOARD_PERMISSION_REPAIR_PENDING_KEY) === "true"
  );
  const [whipStats, setWhipStats] = useState<WhipStats | null>(null);
  const [whipHitClass, setWhipHitClass] = useState<"hit-a" | "hit-b" | null>(null);
  const [whipSaveError, setWhipSaveError] = useState(false);
  const [lastWhipAt, setLastWhipAt] = useState(0);
  const [whipVariant, setWhipVariant] = useState<"a" | "b">("a");
  const [systemPanelExpanded, setSystemPanelExpanded] = useState(
    () => localStorage.getItem("rundev.systemPanelExpanded") === "true"
  );
  const updateInstallStartedRef = useRef(false);
  const whipCrackRef = useRef<WhipCrackApi>(null);
  const runnerRef = useRef<HTMLButtonElement>(null);
  const shellRef = useRef<HTMLElement>(null);
  const freezeRunner = new URLSearchParams(window.location.search).has("freezeRunner");
  const showLevelShowcase = new URLSearchParams(window.location.search).has("levelShowcase");
  const {
    summary,
    focus,
    currentActivity,
    activityHistory,
    character,
    aiUsage,
    aiWeeklyXp,
    claudeUsage,
    cursorUsage,
    keyboard,
    runner,
    systemStats,
    loading,
    error,
    refresh,
    connectCodex,
    disconnectCodex,
    connectClaude,
    disconnectClaude,
    connectCursor,
    disconnectCursor,
    selectRunner,
    setKeyboardActivity,
    setFocusActivity,
    setSystemStats
  } = useDashboardStore();
  const runnerFrames = runnerFramesById[runner?.runnerId ?? "coding-cat"];

  function mergeWhipStats(next: WhipStats) {
    setWhipStats((current) => {
      if (!current || current.localDate !== next.localDate) return next;
      return next.whipCount >= current.whipCount ? next : current;
    });
  }

  async function loadWhipStats() {
    try {
      mergeWhipStats(await getWhipStats());
      setWhipSaveError(false);
    } catch {
      // Keep last known count; focus refresh can retry.
    }
  }

  function spawnWhipFx(clientX: number, clientY: number, variant: "a" | "b") {
    const shell = shellRef.current;
    if (!shell) return;
    const rect = shell.getBoundingClientRect();
    whipCrackRef.current?.crackAt(
      clientX - rect.left,
      clientY - rect.top,
      variant
    );
  }

  const performWhip = useCallback(async () => {
    const now = Date.now();
    if (now - lastWhipAt < WHIP_COOLDOWN_MS) return;
    setLastWhipAt(now);
    const nextVariant = whipVariant === "a" ? "b" : "a";
    setWhipVariant(nextVariant);
    setWhipHitClass(nextVariant === "a" ? "hit-a" : "hit-b");
    setWhipSaveError(false);

    const runnerElement = runnerRef.current;
    if (runnerElement) {
      const rect = runnerElement.getBoundingClientRect();
      spawnWhipFx(
        rect.left + rect.width / 2,
        rect.top + rect.height / 2,
        nextVariant
      );
    }

    try {
      mergeWhipStats(await recordWhip());
    } catch {
      setWhipSaveError(true);
    }
  }, [lastWhipAt, whipVariant]);

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

  async function confirmCursorConsent() {
    setPreviewLoading(true);
    setPreviewError(null);
    try {
      await grantCursorUsageConsent();
      setCursorConsentOpen(false);
      setCursorPreview(await previewCursorAccount());
    } catch (connectionError) {
      setPreviewError(
        connectionError instanceof Error ? connectionError.message : String(connectionError)
      );
    } finally {
      setPreviewLoading(false);
    }
  }

  async function confirmCursorConnection() {
    setPreviewError(null);
    try {
      await connectCursor();
      setCursorPreview(null);
    } catch (connectionError) {
      setPreviewError(
        connectionError instanceof Error ? connectionError.message : String(connectionError)
      );
    }
  }

  async function runUpdateCheck(force: boolean) {
    setUpdateChecking(true);
    if (force) setUpdateError(null);
    try {
      const status = await checkForAppUpdate({
        force,
        currentVersion: packageJson.version
      });
      setUpdateStatus(status);
      if (status.available && status.version) {
        await notifyIfUpdateAvailable(status.version);
      }
    } catch (checkError) {
      if (force) {
        setUpdateError(
          checkError instanceof Error ? checkError.message : String(checkError)
        );
      }
    } finally {
      setUpdateChecking(false);
    }
  }

  async function repairKeyboardPermission() {
    setPermissionRepairing(true);
    setPermissionRepairError(null);
    localStorage.setItem(KEYBOARD_PERMISSION_REPAIR_PENDING_KEY, "true");
    setPermissionRepairPending(true);
    try {
      await resetKeyboardPermissionAndRelaunch();
    } catch (repairError) {
      localStorage.removeItem(KEYBOARD_PERMISSION_REPAIR_PENDING_KEY);
      setPermissionRepairPending(false);
      setPermissionRepairError(
        repairError instanceof Error ? repairError.message : String(repairError)
      );
      setPermissionRepairing(false);
    }
  }

  async function showDiagnosticsFolder() {
    setPermissionRepairError(null);
    try {
      await openDiagnosticsFolder();
    } catch (diagnosticError) {
      setPermissionRepairError(
        diagnosticError instanceof Error
          ? diagnosticError.message
          : String(diagnosticError)
      );
    }
  }

  async function installAvailableUpdate() {
    if (updateInstallStartedRef.current) return;
    updateInstallStartedRef.current = true;
    setUpdateInstalling(true);
    setUpdateError(null);
    try {
      await downloadAndInstallAppUpdate();
    } catch (installError) {
      setUpdateError(
        installError instanceof Error ? installError.message : String(installError)
      );
      setUpdateInstalling(false);
      updateInstallStartedRef.current = false;
    }
  }

  useEffect(() => {
    void refresh();
    void loadWhipStats();
    const timer = window.setInterval(() => void refresh(), 5_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    if (keyboard?.status !== "active" || !permissionRepairPending) return;
    localStorage.removeItem(KEYBOARD_PERMISSION_REPAIR_PENDING_KEY);
    setPermissionRepairPending(false);
    setPermissionRepairError(null);
  }, [keyboard?.status, permissionRepairPending]);

  useEffect(() => {
    const onFocus = () => {
      void loadWhipStats();
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  useEffect(() => {
    void runUpdateCheck(false);
    const timer = window.setInterval(
      () => void runUpdateCheck(false),
      UPDATE_CHECK_INTERVAL_MS
    );
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!infoDialogOpen) return;
    void runUpdateCheck(false);
  }, [infoDialogOpen]);

  useEffect(() => {
    void getXpBoostStatus().then(setXpBoost).catch(() => {});
    const timer = window.setInterval(() => setBoostClock(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!xpBoost?.endsAt || new Date(xpBoost.endsAt).getTime() > boostClock) return;
    void getXpBoostStatus()
      .then(setXpBoost)
      .catch(() => setXpBoost({ active: false, multiplier: null, startsAt: null, endsAt: null }));
  }, [boostClock, xpBoost]);

  async function inspectCoupon() {
    const normalized = couponCode.trim();
    if (!normalized) return;
    setCouponLoading(true);
    setCouponError(null);
    try {
      setCouponPreview(await previewXpCoupon(normalized));
    } catch (couponFailure) {
      setCouponError(couponFailure instanceof Error ? couponFailure.message : String(couponFailure));
    } finally {
      setCouponLoading(false);
    }
  }

  async function applyCoupon() {
    setCouponLoading(true);
    setCouponError(null);
    try {
      const next = await redeemXpCoupon(couponCode.trim());
      setXpBoost(next);
      setBoostClock(Date.now());
      setCouponDialogOpen(false);
      setCouponPreview(null);
      setCouponCode("");
      await refresh();
    } catch (couponFailure) {
      setCouponError(couponFailure instanceof Error ? couponFailure.message : String(couponFailure));
    } finally {
      setCouponLoading(false);
    }
  }

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
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void subscribeSystemStats((stats) => {
      if (!cancelled) setSystemStats(stats);
    }).then((dispose) => {
      if (cancelled) {
        dispose();
        return;
      }
      unlisten = dispose;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setSystemStats]);

  useEffect(() => {
    void resizeSystemPanel(systemPanelExpanded);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void subscribeUsageRefreshed(() => {
      void refresh();
    }).then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  }, [refresh]);

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
  const focusRewardCount = Math.floor((summary?.activeSeconds ?? 0) / 1_800);
  const keyboardProgress = ((keyboard?.pressCount ?? 0) % 2_000) / 20;
  const keyboardRemaining = Math.max(
    0,
    (keyboard?.nextRewardAt ?? 2_000) - (keyboard?.pressCount ?? 0)
  );
  const keyboardRewardCount = Math.floor((keyboard?.pressCount ?? 0) / 2_000);
  const hasUsageDetails =
    aiUsage?.status !== "disconnected" ||
    claudeUsage?.status !== "disconnected" ||
    cursorUsage?.status !== "disconnected";
  const activeHistoryDays = activityHistory.filter((day) => day.activeSeconds > 0).length;
  const activityStatus = useMemo(
    () => buildActivityStatus(currentActivity),
    [currentActivity?.active, currentActivity?.focused, currentActivity?.appName]
  );

  if (showLevelShowcase) {
    return <LevelShowcase />;
  }

  function toggleSystemPanel() {
    const expanded = !systemPanelExpanded;
    setSystemPanelExpanded(expanded);
    localStorage.setItem("rundev.systemPanelExpanded", String(expanded));
    void resizeSystemPanel(expanded);
  }

  return (
    <main
      ref={shellRef}
      className={`popover-shell${hasUsageDetails ? " dense" : ""}`}
    >
      <WhipCrackOverlay
        ref={whipCrackRef}
        width={systemPanelExpanded ? 512 : 340}
        height={480}
      />
      <PingModeOverlay rootRef={shellRef} onWhip={performWhip} />
      <div className="popover-main">
      <header className="runner-header">
        <button
          ref={runnerRef}
          type="button"
          className={`runner${whipHitClass ? ` ${whipHitClass}` : ""}`}
          aria-label="개발자 캐릭터 채찍질하기"
          title={
            whipSaveError
              ? "저장 실패"
              : `오늘 ${whipStats?.whipCount ?? 0}`
          }
          onClick={() => void performWhip()}
        >
          <img src={runnerFrames[headerFrame]} alt="" aria-hidden="true" />
          <span className="whip-count">오늘 {whipStats?.whipCount ?? 0}</span>
        </button>
        <div className="runner-copy">
          <div className="runner-title-row">
            <strong>RunDev</strong>
            {xpBoost?.active && xpBoost.multiplier && xpBoost.endsAt ? (
              <div className="xp-boost-badge" title={`경험치 ${xpBoost.multiplier}배 적용 중`}>
                <Zap size={10} fill="currentColor" />
                <b>XP ×{xpBoost.multiplier}</b>
                <span>{formatBoostRemaining(new Date(xpBoost.endsAt).getTime() - boostClock)} 남음</span>
              </div>
            ) : null}
          </div>
          <div
            className="activity-status"
            aria-label={
              activityStatus.appName
                ? `${activityStatus.appName}에서 ${activityStatus.message}`
                : activityStatus.message
            }
          >
            <i className={`status-dot ${activityStatus.tone}`} />
            <div className="activity-status-copy">
              <span className="activity-app-line">
                {activityStatus.appName
                  ? `${activityStatus.appName}에서`
                  : activityStatus.message}
              </span>
              {activityStatus.appName ? (
                <span className="activity-message">{activityStatus.message}</span>
              ) : null}
            </div>
          </div>
          {whipSaveError ? <em className="whip-save-error">저장 실패</em> : null}
        </div>
        <div className="header-actions">
          <button
            className="plain-button"
            type="button"
            aria-label="RunDev 정보"
            onClick={() => setInfoDialogOpen(true)}
          >
            <Info size={17} />
            {updateStatus.available && <i className="update-dot" aria-hidden="true" />}
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
        <div className="reward-cycle-summary">
          <span className="reward-rule">
            <BadgeCheck size={11} />
            30분마다 <b>+10 XP</b>
          </span>
          <strong>오늘 {focusRewardCount}회 달성</strong>
        </div>
        <div className="keyboard-progress focus-reward">
          <span>
            {focusRewardCount + 1}회차 · {formatRemainingMinutes(focusRewardRemaining)} 남음
          </span>
          <Meter value={focusRewardProgress} />
        </div>
        <div className="keyboard-stat">
          <div>
            <Keyboard size={14} />
            <span>오늘 두드린 키보드</span>
          </div>
          <strong>{formatTokens(keyboard?.pressCount ?? 0)}회</strong>
        </div>
        {keyboard?.permissionRequired ? (
          <div className="keyboard-permission">
            <span>
              {permissionRepairPending
                ? "기존 RunDev를 −로 삭제하고 /Applications의 RunDev를 다시 추가하세요."
                : "새 설치 후에는 입력 권한을 다시 연결해야 할 수 있습니다."}
            </span>
            <button
              type="button"
              disabled={permissionRepairing}
              onClick={() => void repairKeyboardPermission()}
            >
              {permissionRepairing ? "복구 중…" : "권한 다시 연결"}
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
          <>
            <div className="reward-cycle-summary keyboard-reward-summary">
              <span className="reward-rule">
                <BadgeCheck size={11} />
                2,000회마다 <b>+10 XP</b>
              </span>
              <strong>오늘 {keyboardRewardCount}회 달성</strong>
            </div>
            <div className="keyboard-progress">
              <span>
                {keyboardRewardCount + 1}회차 · {formatTokens(keyboardRemaining)}회 남음
              </span>
              <Meter value={keyboardProgress} />
            </div>
          </>
        )}
      </section>

      <div className="divider" />

      <section className="info-section compact">
        <SectionTitle>AI 사용량</SectionTitle>
        <details className="provider-details">
          <summary className="provider-summary-row">
            <span className="provider-icon">
              <img className="openai-icon" src={openAiIcon} alt="" aria-hidden="true" />
            </span>
            <strong>Codex</strong>
            <div className="provider-summary-value">
              <span>이번 주 토큰 사용량</span>
              <b className={aiUsage?.status === "error" ? "needs-attention" : ""}>
                {aiUsage?.status === "disconnected"
                  ? "연동하기"
                  : aiUsage?.status === "error"
                  ? "확인 필요"
                  : aiUsage?.weekTokens == null
                  ? "—"
                  : formatCompactTokens(aiUsage.weekTokens)}
              </b>
            </div>
          </summary>
          <ProviderInlineDetails
            rows={[
              {
                label: "상태",
                value:
                  aiUsage?.status === "disconnected"
                    ? "연동되지 않음"
                    : aiUsage?.status === "error"
                    ? "연결 확인 필요"
                    : aiUsage?.status === "delayed"
                    ? "오늘 집계 지연"
                    : "정상"
              },
              {
                label: "오늘의 토큰 사용량",
                value: aiUsage?.totalTokens == null ? "—" : formatTokens(aiUsage.totalTokens)
              },
              {
                label: "이번 주 토큰 사용량",
                value: aiUsage?.weekTokens == null ? "—" : formatTokens(aiUsage.weekTokens)
              },
              { label: "계정", value: aiUsage?.accountLabel ?? "확인되지 않음" },
              { label: "마지막 갱신", value: formatSyncTime(aiUsage?.lastSyncedAt) },
              { label: "출처", value: aiUsage?.source ? "Codex 계정" : "—" }
            ]}
            error={
              aiUsage?.error
                ? "Codex 로그인 또는 설치 상태를 확인해 주세요."
                : previewError
            }
          >
            {aiUsage?.status === "disconnected" ? (
              <button
                type="button"
                disabled={loading || previewLoading}
                onClick={() => void openCodexConnection()}
              >
                {previewLoading ? "확인 중" : "연동하기"}
              </button>
            ) : (
              <button type="button" disabled={loading} onClick={() => void disconnectCodex()}>
                연동 해제
              </button>
            )}
          </ProviderInlineDetails>
        </details>

        <details className="provider-details">
          <summary className="provider-summary-row">
            <span className="provider-icon">
              <img src={claudeIcon} alt="" aria-hidden="true" />
            </span>
            <strong>Claude Code</strong>
            <div className="provider-summary-value">
              <span>오늘의 토큰 사용량</span>
              <b className={claudeUsage?.status === "error" ? "needs-attention" : ""}>
                {claudeUsage?.status === "disconnected"
                  ? "연동하기"
                  : claudeUsage?.status === "waiting"
                  ? "대기 중"
                  : claudeUsage?.status === "error"
                  ? "확인 필요"
                  : formatCompactTokens(claudeUsage?.totalTokens ?? 0)}
              </b>
            </div>
          </summary>
          <ProviderInlineDetails
            rows={[
              {
                label: "상태",
                value:
                  claudeUsage?.status === "disconnected"
                    ? "연동되지 않음"
                    : claudeUsage?.status === "waiting"
                    ? "첫 사용량 대기 중"
                    : claudeUsage?.status === "error"
                    ? "로컬 수집기 확인 필요"
                    : "정상"
              },
              {
                label: "오늘의 토큰 사용량",
                value:
                  claudeUsage?.status === "disconnected"
                    ? "—"
                    : formatTokens(claudeUsage?.totalTokens ?? 0)
              },
              {
                label: "이번 주 토큰 사용량",
                value:
                  claudeUsage?.status === "disconnected"
                    ? "—"
                    : formatTokens(claudeUsage?.weekTokens ?? 0)
              },
              { label: "최근 활동 세션", value: `${claudeUsage?.sessionCount ?? 0}개` },
              { label: "마지막 수신", value: formatSyncTime(claudeUsage?.lastReceivedAt) },
              { label: "출처", value: "로컬 OpenTelemetry" }
            ]}
            error={
              claudeUsage?.error
                ? "RunDev를 재시작하거나 로컬 포트 상태를 확인해 주세요."
                : previewError
            }
          >
            {claudeUsage?.status === "disconnected" ? (
              <button
                type="button"
                disabled={loading || previewLoading}
                onClick={() => void openClaudeConnection()}
              >
                {previewLoading ? "확인 중" : "연동하기"}
              </button>
            ) : (
              <button type="button" disabled={loading} onClick={() => void disconnectClaude()}>
                연동 해제
              </button>
            )}
          </ProviderInlineDetails>
        </details>

        <details className="provider-details">
          <summary className="provider-summary-row">
            <span className="provider-icon">
              <img src={cursorIcon} alt="" aria-hidden="true" />
            </span>
            <strong>Cursor</strong>
            <div className="provider-summary-value">
              <span>오늘의 토큰 사용량</span>
              <b
                className={
                  cursorUsage?.status === "reauthRequired" ||
                  cursorUsage?.status === "unsupportedSchema" ||
                  cursorUsage?.status === "error"
                    ? "needs-attention"
                    : ""
                }
              >
                {cursorUsage?.status === "disconnected"
                  ? "연동하기"
                  : cursorUsage?.status === "syncing"
                  ? "동기화 중"
                  : cursorUsage?.status === "rateLimited"
                  ? "잠시 후"
                  : cursorUsage?.status === "reauthRequired" ||
                    cursorUsage?.status === "unsupportedSchema" ||
                    cursorUsage?.status === "error"
                  ? "확인 필요"
                  : formatCompactTokens(cursorUsage?.totalTokens ?? 0)}
              </b>
            </div>
          </summary>
          <ProviderInlineDetails
            rows={[
              {
                label: "상태",
                value:
                  cursorUsage?.status === "disconnected"
                    ? "연동되지 않음"
                    : cursorUsage?.status === "reauthRequired"
                    ? "Cursor에서 다시 로그인 필요"
                    : cursorUsage?.status === "rateLimited"
                    ? "요청 제한 · 잠시 후 재시도"
                    : cursorUsage?.status === "unsupportedSchema"
                    ? "사용량 형식 확인 필요"
                    : cursorUsage?.status === "stale"
                    ? "마지막 정상 데이터"
                    : cursorUsage?.status === "syncing"
                    ? "동기화 중"
                    : cursorUsage?.status === "error"
                    ? "동기화 오류"
                    : "정상"
              },
              {
                label: "오늘의 토큰 사용량",
                value:
                  cursorUsage?.status === "disconnected"
                    ? "—"
                    : formatTokens(cursorUsage?.totalTokens ?? 0)
              },
              {
                label: "이번 주 토큰 사용량",
                value:
                  cursorUsage?.status === "disconnected"
                    ? "—"
                    : formatTokens(cursorUsage?.weekTokens ?? 0)
              },
              {
                label: "요청 한도",
                value: `${formatRequestUnits(cursorUsage?.usedRequests)} / ${formatRequestUnits(
                  cursorUsage?.limitRequests
                )}`
              },
              {
                label: "오늘 요청량",
                value: formatRequestUnits(cursorUsage?.todayRequests)
              },
              { label: "마지막 갱신", value: formatSyncTime(cursorUsage?.lastSyncedAt) },
              { label: "출처", value: "Cursor Dashboard" }
            ]}
            error={
              cursorUsage?.errorCode
                ? "Cursor 사용량을 갱신하지 못했습니다. 화면을 다시 열어 재시도할 수 있습니다."
                : previewError
            }
          >
            {cursorUsage?.status === "disconnected" ? (
              <button
                type="button"
                disabled={loading || previewLoading}
                onClick={() => setCursorConsentOpen(true)}
              >
                연동하기
              </button>
            ) : (
              <button type="button" disabled={loading} onClick={() => void disconnectCursor()}>
                연동 해제
              </button>
            )}
          </ProviderInlineDetails>
        </details>
        <AiWeeklyXpProgress progress={aiWeeklyXp} />
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
      </div>
      <SystemStatusStrip
        stats={systemStats}
        expanded={systemPanelExpanded}
        onToggle={toggleSystemPanel}
      />
      {infoDialogOpen && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog app-info-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="app-info-title"
          >
            <h2 id="app-info-title">RunDev 정보</h2>
            <p>개발 활동을 기록하고 성장으로 보여주는 로컬 우선 트레이 앱입니다.</p>
            <dl>
              <div><dt>버전</dt><dd>v{packageJson.version}</dd></div>
              <div><dt>기술</dt><dd>Tauri 2 · React · Rust · SQLite</dd></div>
              <div><dt>데이터</dt><dd>이 기기에만 저장</dd></div>
              <div>
                <dt>업데이트</dt>
                <dd>
                  {updateInstalling
                    ? "설치 중…"
                    : updateChecking
                      ? "확인 중…"
                      : updateStatus.available && updateStatus.version
                        ? `v${updateStatus.version} 사용 가능`
                        : "최신 버전"}
                </dd>
              </div>
            </dl>
            <p>프롬프트, 소스 코드, 키 입력 내용은 저장하지 않습니다.</p>
            {/Macintosh|Mac OS X/.test(navigator.userAgent) && (
              <div className="keyboard-repair">
                <div>
                  <strong>키보드 입력이 집계되지 않나요?</strong>
                  <span>입력 모니터링 권한을 초기화하고 현재 RunDev를 다시 등록합니다.</span>
                </div>
                <button
                  type="button"
                  disabled={permissionRepairing}
                  onClick={() => void repairKeyboardPermission()}
                >
                  {permissionRepairing ? "복구 중…" : "권한 다시 연결"}
                </button>
              </div>
            )}
            {permissionRepairError && <p className="error-message">{permissionRepairError}</p>}
            {updateError && <p className="error-message">{updateError}</p>}
            <div className="dialog-actions">
              <button
                type="button"
                onClick={() => {
                  setInfoDialogOpen(false);
                  setCouponError(null);
                  setCouponDialogOpen(true);
                }}
              >
                쿠폰 입력
              </button>
              <button type="button" onClick={() => void showDiagnosticsFolder()}>
                진단 로그 폴더
              </button>
              <button type="button" onClick={() => setInfoDialogOpen(false)}>닫기</button>
              <button
                type="button"
                disabled={updateChecking || updateInstalling}
                onClick={() => void runUpdateCheck(true)}
              >
                {updateChecking ? "확인 중" : "업데이트 확인"}
              </button>
              {updateStatus.available && (
                <button
                  type="button"
                  className="confirm-button"
                  disabled={updateInstalling || updateChecking}
                  onClick={() => void installAvailableUpdate()}
                >
                  {updateInstalling ? "설치 중" : "다운로드 및 재시작"}
                </button>
              )}
            </div>
          </section>
        </div>
      )}
      {couponDialogOpen && (
        <div className="dialog-backdrop" role="presentation">
          <section className="account-dialog coupon-dialog" role="dialog" aria-modal="true" aria-labelledby="coupon-title">
            <h2 id="coupon-title">경험치 쿠폰</h2>
            {!couponPreview ? (
              <>
                <p>쿠폰 번호를 입력하면 적용 배수와 시간을 먼저 확인할 수 있어요.</p>
                <input
                  className="coupon-input"
                  value={couponCode}
                  autoFocus
                  spellCheck={false}
                  placeholder="RDC1.…"
                  onChange={(event) => {
                    setCouponCode(event.target.value);
                    setCouponError(null);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void inspectCoupon();
                  }}
                />
              </>
            ) : (
              <>
                <p>이 쿠폰을 지금 사용할까요?</p>
                <dl>
                  <div><dt>경험치</dt><dd>{couponPreview.multiplier}배</dd></div>
                  <div><dt>적용 시간</dt><dd>{formatBoostRemaining(couponPreview.durationMinutes * 60_000)}</dd></div>
                  <div><dt>등록 기한</dt><dd>{new Date(couponPreview.redeemBefore).toLocaleDateString("ko-KR")}</dd></div>
                </dl>
                <p>이미 부스트가 예약되어 있다면 기존 종료 시점 뒤에 이어서 적용됩니다.</p>
              </>
            )}
            {couponError && <p className="error-message">{couponError}</p>}
            <div className="dialog-actions">
              <button
                type="button"
                onClick={() => {
                  setCouponDialogOpen(false);
                  setCouponPreview(null);
                  setCouponError(null);
                }}
              >
                취소
              </button>
              {couponPreview ? (
                <button type="button" className="confirm-button" disabled={couponLoading} onClick={() => void applyCoupon()}>
                  {couponLoading ? "적용 중" : "부스트 적용"}
                </button>
              ) : (
                <button type="button" className="confirm-button" disabled={couponLoading || !couponCode.trim()} onClick={() => void inspectCoupon()}>
                  {couponLoading ? "확인 중" : "쿠폰 확인"}
                </button>
              )}
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
      {cursorConsentOpen && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="cursor-consent-title"
          >
            <h2 id="cursor-consent-title">Cursor 사용량 조회에 동의할까요?</h2>
            <p>
              RunDev는 이 PC의 Cursor 로그인 정보에서 인증 토큰 한 항목을
              일시적으로 읽어 Cursor 서버에서 내 사용량만 확인합니다. 토큰은
              RunDev DB에 저장하지 않으며 Cursor 외부로 보내지 않습니다.
            </p>
            <p>
              Cursor의 비공식 인터페이스를 사용하므로 향후 동작하지 않을 수 있습니다.
            </p>
            <div className="dialog-actions">
              <button type="button" onClick={() => setCursorConsentOpen(false)}>취소</button>
              <button
                type="button"
                className="confirm-button"
                disabled={previewLoading}
                onClick={() => void confirmCursorConsent()}
              >
                {previewLoading ? "확인 중" : "동의하고 계정 확인"}
              </button>
            </div>
          </section>
        </div>
      )}
      {cursorPreview && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="account-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="cursor-account-title"
          >
            <h2 id="cursor-account-title">이 Cursor 계정을 연동할까요?</h2>
            <dl>
              <div><dt>계정</dt><dd>{cursorPreview.accountLabel}</dd></div>
              <div><dt>요금제</dt><dd>{cursorPreview.planType ?? "확인 불가"}</dd></div>
              <div><dt>수집 범위</dt><dd>사용량 집계와 주기 한도</dd></div>
            </dl>
            <p>프롬프트, 응답, 채팅 및 소스 코드는 읽지 않습니다.</p>
            {previewError && <p className="adapter-error">{previewError}</p>}
            <div className="dialog-actions">
              <button type="button" onClick={() => setCursorPreview(null)}>취소</button>
              <button
                type="button"
                className="confirm-button"
                disabled={loading}
                onClick={() => void confirmCursorConnection()}
              >
                {loading ? "연동 중" : "Cursor 연동"}
              </button>
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
