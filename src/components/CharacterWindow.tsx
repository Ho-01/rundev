import { useEffect, useRef, useState, type CSSProperties, type MouseEvent, type PointerEvent } from "react";
import { Maximize2, X } from "lucide-react";
import type { DragDropEvent } from "@tauri-apps/api/webview";
import { desktopRunnerFrames } from "../assets/runners/desktop";
import { feedingRunnerFrames } from "../assets/runners/feeding";
import { grabbedRunnerFrames } from "../assets/runners/grabbed";
import { roamingRunnerFrames } from "../assets/runners/roaming";
import type { RunnerId, RunnerSkinId } from "../types/activity";
import {
  beginCharacterDrag,
  beginCharacterFileDrop,
  dragCharacterWindow,
  endCharacterDrag,
  endCharacterFileDrop,
  finishCharacterResize,
  getCharacterWindowState,
  getCharacterRunner,
  requestCharacterWhip,
  resizeCharacterWindow,
  setCharacterWindowVisible,
  showCharacterContextMenu,
  subscribeCharacterMotion,
  subscribeCharacterFileDrop,
  subscribeCharacterDragEnd,
  subscribeCharacterWindowState,
  subscribeRunnerSelection,
  subscribeTypingPulse,
  trashDroppedFiles
} from "../services/characterWindow";

const DRAG_THRESHOLD = 7;
const DRAG_DELAY_MS = 140;
const CHARACTER_SIZE_MIN = 36;
const CHARACTER_SIZE_MAX = 128;
const DEFAULT_CHARACTER_SIZE = 48;
const ROAMING_FRAME_INTERVAL_MS = 170;
const GRABBED_FRAME_INTERVAL_MS = 300;
const FEED_FRAME_INTERVAL_MS = 200;
const FEED_SWALLOW_HOLD_MS = 280;
const FEED_FRAME_SEQUENCE = [1, 2, 1, 2, 3] as const;
type FeedPhase = "idle" | "ready" | "processing" | "consuming" | "finishing";
// Master roaming sprites are authored facing right, except the vtuber run cycle.
const roamingSourceDirectionByRunner: Record<RunnerId, 1 | -1> = {
  "coding-cat": 1,
  "coding-fish": 1,
  "coding-orange-cat": 1,
  "coding-shrimp": 1,
  "coding-vtuber": -1
};
const roamingVisualScaleByRunner: Record<RunnerId, number> = {
  "coding-cat": 0.82,
  "coding-fish": 0.72,
  "coding-orange-cat": 0.79,
  "coding-shrimp": 1,
  "coding-vtuber": 1
};
const grabbedVisualScaleByRunner: Record<RunnerId, number> = {
  "coding-cat": 1.35,
  "coding-fish": 1.25,
  "coding-orange-cat": 1.25,
  "coding-shrimp": 1.3,
  // The held sprite's head is 98 px wide versus 145 px in the seated sprite.
  // Keep the perceived head size consistent while the character is held.
  "coding-vtuber": 1.5
};

function grabbedVisualScale(runnerId: RunnerId, skinId: RunnerSkinId) {
  // Pool Party held frames use the same normalized canvas, but the standing head is
  // 16% narrower than the seated pose. Compensate only enough to match head size.
  return runnerId === "coding-vtuber" && skinId === "pool-party"
    ? 1.16
    : grabbedVisualScaleByRunner[runnerId];
}

export function CharacterWindow() {
  const [runnerId, setRunnerId] = useState<RunnerId>("coding-cat");
  const [skinId, setSkinId] = useState<RunnerSkinId>("default");
  const [frame, setFrame] = useState(0);
  const [typingMotion, setTypingMotion] = useState(0);
  const [followPointer, setFollowPointer] = useState(false);
  const [roaming, setRoaming] = useState(false);
  const [moving, setMoving] = useState(false);
  const [hoveringCharacter, setHoveringCharacter] = useState(false);
  const [direction, setDirection] = useState(1);
  const [roamingFrame, setRoamingFrame] = useState(0);
  const [whipPulse, setWhipPulse] = useState<0 | 1 | 2>(0);
  const [dragVisualActive, setDragVisualActive] = useState(false);
  const [resizingCharacter, setResizingCharacter] = useState(false);
  const [grabbedFrame, setGrabbedFrame] = useState(0);
  const [characterSize, setCharacterSize] = useState(DEFAULT_CHARACTER_SIZE);
  const [feedPhase, setFeedPhase] = useState<FeedPhase>("idle");
  const [feedFrame, setFeedFrame] = useState(0);
  const lastTypingAt = useRef(Number.NEGATIVE_INFINITY);
  const typingEnergy = useRef(0);
  const rescheduleAnimation = useRef<() => void>(() => {});
  const dragTimer = useRef<number | null>(null);
  const dragOrigin = useRef({ x: 0, y: 0 });
  const dragStarted = useRef(false);
  const dragPausePromise = useRef<Promise<void> | null>(null);
  const dragPauseActive = useRef(false);
  const dragPauseReleaseRequested = useRef(false);
  const dragPauseSafetyTimer = useRef<number | null>(null);
  const characterSizeRef = useRef(DEFAULT_CHARACTER_SIZE);
  const resizeGesture = useRef<{ pointerId: number; startX: number; startY: number; startSize: number } | null>(null);
  const resizePending = useRef<number | null>(null);
  const resizeFlushPromise = useRef<Promise<void> | null>(null);
  const whipTimer = useRef<number | null>(null);
  const feedPhaseRef = useRef<FeedPhase>("idle");
  const feedBeginPromise = useRef<Promise<void> | null>(null);
  const feedFinishTimer = useRef<number | null>(null);
  const [windowVisible, setWindowVisible] = useState(false);
  const frames = desktopRunnerFrames(runnerId, skinId);
  const feedingFrames = feedingRunnerFrames(runnerId, skinId);
  const grabbedFrames = grabbedRunnerFrames(runnerId, skinId);
  const roamingFrames = roamingRunnerFrames(runnerId, skinId);
  const roamingFrameInterval = runnerId === "coding-vtuber" && skinId === "pool-party"
    ? 260
    : ROAMING_FRAME_INTERVAL_MS;

  useEffect(() => {
    setRoamingFrame(0);
    if (!windowVisible || feedPhase !== "idle" || !roaming || !moving || roamingFrames.length < 2) return;
    const timer = window.setInterval(() => {
      setRoamingFrame((current) => (current + 1) % roamingFrames.length);
    }, roamingFrameInterval);
    return () => window.clearInterval(timer);
  }, [feedPhase, moving, roaming, roamingFrameInterval, roamingFrames.length, runnerId, windowVisible]);

  useEffect(() => {
    setGrabbedFrame(0);
    if (!windowVisible || feedPhase !== "idle" || !dragVisualActive || grabbedFrames.length < 2) return;
    const timer = window.setInterval(() => {
      setGrabbedFrame((current) => (current + 1) % grabbedFrames.length);
    }, GRABBED_FRAME_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [dragVisualActive, feedPhase, grabbedFrames.length, runnerId, windowVisible]);

  useEffect(() => {
    void getCharacterRunner().then((selection) => {
      setRunnerId(selection.runnerId);
      setSkinId(selection.skinId);
    });
    let unlisten = () => {};
    void subscribeRunnerSelection((selection) => {
      setRunnerId(selection.runnerId);
      setSkinId(selection.skinId);
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, []);

  useEffect(() => {
    let unlistenState = () => {};
    let unlistenMotion = () => {};
    let active = true;

    function applyState({ followPointer: follows, roaming: isRoaming, moving: isMoving, direction: nextDirection, visible, size }: {
      followPointer: boolean;
      roaming: boolean;
      moving: boolean;
      direction: number;
      visible: boolean;
      size: number;
    }) {
      setFollowPointer(follows);
      setRoaming(isRoaming);
      setMoving(isMoving);
      setDirection(nextDirection);
      setWindowVisible(visible);
      if (follows || isMoving || !visible) setHoveringCharacter(false);
      if (Number.isFinite(size)) {
        const nextSize = Math.min(CHARACTER_SIZE_MAX, Math.max(CHARACTER_SIZE_MIN, size));
        characterSizeRef.current = nextSize;
        setCharacterSize(nextSize);
      }
    }

    void Promise.all([
      subscribeCharacterWindowState(applyState),
      subscribeCharacterMotion(({ moving: isMoving, direction: nextDirection }) => {
        setMoving(isMoving);
        setDirection(nextDirection);
        if (isMoving) setHoveringCharacter(false);
      })
    ]).then(([nextState, nextMotion]) => {
      if (!active) {
        nextState();
        nextMotion();
        return;
      }
      unlistenState = nextState;
      unlistenMotion = nextMotion;
      return getCharacterWindowState().then(applyState);
    });

    return () => {
      active = false;
      unlistenState();
      unlistenMotion();
    };
  }, []);

  useEffect(() => {
    let timer = 0;

    function schedule() {
      if (!windowVisible || moving || feedPhase !== "idle") return;
      const now = performance.now();
      const sinceTyping = now - lastTypingAt.current;
      const typing = sinceTyping < 420;
      const delay = typing ? 120 - typingEnergy.current * 75 : 170;
      timer = window.setTimeout(() => {
        const nextNow = performance.now();
        const nextSinceTyping = nextNow - lastTypingAt.current;
        const nextTyping = nextSinceTyping < 420;
        if (nextTyping) {
          if (nextSinceTyping > 90) {
            typingEnergy.current = Math.max(0.35, typingEnergy.current - delay / 900);
          }
        } else {
          typingEnergy.current = Math.max(0, typingEnergy.current - delay / 260);
        }
        setFrame((current) => (current + 1) % frames.length);
        setTypingMotion((current) => (nextTyping ? current + 1 : 0) % 4);
        schedule();
      }, delay);
    }

    rescheduleAnimation.current = () => {
      window.clearTimeout(timer);
      schedule();
    };
    schedule();
    return () => {
      window.clearTimeout(timer);
      rescheduleAnimation.current = () => {};
    };
  }, [feedPhase, frames.length, moving, windowVisible]);

  useEffect(() => {
    let unlisten = () => {};
    void subscribeTypingPulse(() => {
      const now = performance.now();
      const startingSession = now - lastTypingAt.current >= 420;
      lastTypingAt.current = now;
      typingEnergy.current = Math.min(
        1,
        Math.max(startingSession ? 0.55 : typingEnergy.current, typingEnergy.current + 0.16)
      );
      if (startingSession && !moving && feedPhase === "idle") {
        setFrame((current) => (current + 1) % frames.length);
        setTypingMotion(1);
      }
      rescheduleAnimation.current();
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, [feedPhase, frames.length, moving]);

  function setFeedState(next: FeedPhase) {
    feedPhaseRef.current = next;
    setFeedPhase(next);
    if (next === "idle" || next === "ready" || next === "processing") setFeedFrame(0);
  }

  function clearFeedFinishTimer() {
    if (feedFinishTimer.current !== null) {
      window.clearTimeout(feedFinishTimer.current);
      feedFinishTimer.current = null;
    }
  }

  function startFileDropHover() {
    if (feedPhaseRef.current !== "idle" || feedBeginPromise.current !== null) return;
    setHoveringCharacter(false);
    setFeedState("ready");
    const pending = beginCharacterFileDrop();
    feedBeginPromise.current = pending;
    void pending.catch(() => {
      if (feedPhaseRef.current === "ready") setFeedState("idle");
      feedBeginPromise.current = null;
    });
  }

  function finishFileDrop() {
    clearFeedFinishTimer();
    const pending = feedBeginPromise.current ?? Promise.resolve();
    feedBeginPromise.current = null;
    setFeedState("finishing");
    void pending
      .then(() => endCharacterFileDrop())
      .catch(() => {})
      .finally(() => setFeedState("idle"));
  }

  function playConsumeAnimation() {
    let sequenceIndex = 0;
    setFeedState("consuming");
    setFeedFrame(FEED_FRAME_SEQUENCE[sequenceIndex]);

    function advance() {
      sequenceIndex += 1;
      if (sequenceIndex >= FEED_FRAME_SEQUENCE.length) {
        feedFinishTimer.current = window.setTimeout(finishFileDrop, FEED_SWALLOW_HOLD_MS);
        return;
      }
      setFeedFrame(FEED_FRAME_SEQUENCE[sequenceIndex]);
      feedFinishTimer.current = window.setTimeout(advance, FEED_FRAME_INTERVAL_MS);
    }

    feedFinishTimer.current = window.setTimeout(advance, FEED_FRAME_INTERVAL_MS);
  }

  function consumeDroppedFiles(paths: string[]) {
    if (feedPhaseRef.current !== "ready" || paths.length === 0) return;
    setFeedState("processing");
    const pending = feedBeginPromise.current ?? Promise.resolve();
    void pending
      .then(() => trashDroppedFiles(paths))
      .then((moved) => {
        if (moved < 1) {
          finishFileDrop();
          return;
        }
        playConsumeAnimation();
      })
      .catch(() => finishFileDrop());
  }

  function handleFileDropEvent(event: DragDropEvent) {
    if (event.type === "enter" || event.type === "over") {
      startFileDropHover();
      return;
    }
    if (event.type === "drop") {
      consumeDroppedFiles(event.paths);
      return;
    }
    if (feedPhaseRef.current === "ready") finishFileDrop();
  }

  useEffect(() => {
    let unlisten = () => {};
    let active = true;
    void subscribeCharacterFileDrop(handleFileDropEvent).then((next) => {
      if (!active) {
        next();
        return;
      }
      unlisten = next;
    });
    return () => {
      active = false;
      unlisten();
      clearFeedFinishTimer();
      const pending = feedBeginPromise.current;
      feedBeginPromise.current = null;
      if (pending) void pending.then(() => endCharacterFileDrop()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    let unlisten = () => {};
    void subscribeCharacterDragEnd(() => {
      clearDragTimer();
      finishWindowDrag();
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, []);

  function clearDragTimer() {
    if (dragTimer.current !== null) {
      window.clearTimeout(dragTimer.current);
      dragTimer.current = null;
    }
  }

  function clearDragPauseSafetyTimer() {
    if (dragPauseSafetyTimer.current !== null) {
      window.clearTimeout(dragPauseSafetyTimer.current);
      dragPauseSafetyTimer.current = null;
    }
  }

  function requestEndRoamingDrag() {
    dragPauseReleaseRequested.current = true;
    const pending = dragPausePromise.current;
    if (!pending) return;
    void pending.then(() => {
      if (!dragPauseReleaseRequested.current) return;
      dragPauseReleaseRequested.current = false;
      dragPauseActive.current = false;
      dragPausePromise.current = null;
      clearDragPauseSafetyTimer();
      return endCharacterDrag();
    }).catch(() => {
      dragPauseReleaseRequested.current = false;
      dragPauseActive.current = false;
      dragPausePromise.current = null;
      clearDragPauseSafetyTimer();
    });
  }

  function finishWindowDrag() {
    dragStarted.current = false;
    setHoveringCharacter(false);
    setDragVisualActive(false);
    requestEndRoamingDrag();
  }

  function startWindowDrag() {
    setHoveringCharacter(false);
    setGrabbedFrame(0);
    setDragVisualActive(true);
    dragPauseReleaseRequested.current = false;
    const pending = beginCharacterDrag(grabbedVisualScale(runnerId, skinId));
    dragPausePromise.current = pending;
    void pending
      .then(() => {
        if (!dragStarted.current || dragPauseReleaseRequested.current) {
          dragPauseReleaseRequested.current = false;
          dragPausePromise.current = null;
          return endCharacterDrag();
        }
        if (roaming) {
          dragPauseActive.current = true;
          clearDragPauseSafetyTimer();
          dragPauseSafetyTimer.current = window.setTimeout(() => {
            requestEndRoamingDrag();
          }, 15_000);
        }
        return dragCharacterWindow();
      })
      .catch(() => {
        setDragVisualActive(false);
        requestEndRoamingDrag();
      });
  }

  function startDrag(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || followPointer || feedPhaseRef.current !== "idle") return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragOrigin.current = { x: event.clientX, y: event.clientY };
    dragStarted.current = false;
    clearDragTimer();
    dragTimer.current = window.setTimeout(() => {
      dragTimer.current = null;
      dragStarted.current = true;
      startWindowDrag();
    }, DRAG_DELAY_MS);
  }

  function continueDrag(event: PointerEvent<HTMLDivElement>) {
    if (dragStarted.current || dragTimer.current === null) return;
    const dx = event.clientX - dragOrigin.current.x;
    const dy = event.clientY - dragOrigin.current.y;
    if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
    clearDragTimer();
    dragStarted.current = true;
    startWindowDrag();
  }

  function finishPointer(event: PointerEvent<HTMLDivElement>) {
    clearDragTimer();
    finishWindowDrag();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  function handlePointerEnter() {
    if (!followPointer && !moving && feedPhaseRef.current === "idle" && !dragStarted.current) {
      setHoveringCharacter(true);
    }
  }

  function handlePointerLeave() {
    setHoveringCharacter(false);
  }

  function clampCharacterSize(size: number) {
    return Math.min(CHARACTER_SIZE_MAX, Math.max(CHARACTER_SIZE_MIN, size));
  }

  function flushCharacterResize() {
    if (resizeFlushPromise.current) return resizeFlushPromise.current;
    const pending = (async () => {
      while (resizePending.current !== null) {
        const requestedSize = resizePending.current;
        resizePending.current = null;
        try {
          const appliedSize = await resizeCharacterWindow(requestedSize);
          characterSizeRef.current = appliedSize;
          setCharacterSize(appliedSize);
        } catch {
          resizePending.current = null;
          return;
        }
      }
    })();
    resizeFlushPromise.current = pending;
    void pending.finally(() => {
      if (resizeFlushPromise.current !== pending) return;
      resizeFlushPromise.current = null;
      if (resizePending.current !== null) void flushCharacterResize();
    });
    return pending;
  }

  function startCharacterResize(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0 || followPointer || feedPhaseRef.current !== "idle" || dragVisualActive) return;
    event.preventDefault();
    event.stopPropagation();
    clearDragTimer();
    setHoveringCharacter(false);
    resizeGesture.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startSize: characterSizeRef.current
    };
    setResizingCharacter(true);
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function continueCharacterResize(event: PointerEvent<HTMLButtonElement>) {
    const gesture = resizeGesture.current;
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    const horizontalDistance = gesture.startX - event.clientX;
    const verticalDistance = event.clientY - gesture.startY;
    const nextSize = clampCharacterSize(gesture.startSize + (horizontalDistance + verticalDistance) / 2);
    characterSizeRef.current = nextSize;
    setCharacterSize(nextSize);
    resizePending.current = nextSize;
    void flushCharacterResize();
  }

  function finishCharacterResizeGesture(event: PointerEvent<HTMLButtonElement>) {
    const gesture = resizeGesture.current;
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    resizeGesture.current = null;
    setResizingCharacter(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    resizePending.current = characterSizeRef.current;
    void flushCharacterResize().finally(() => {
      void finishCharacterResize(characterSizeRef.current)
        .then((appliedSize) => {
          characterSizeRef.current = appliedSize;
          setCharacterSize(appliedSize);
        })
        .catch(() => {});
    });
  }

  function handleDoubleClick(event: MouseEvent<HTMLDivElement>) {
    if (feedPhase !== "idle" || followPointer || roaming || moving) return;
    event.preventDefault();
    clearDragTimer();
    void requestCharacterWhip().then((accepted) => {
      if (!accepted) return;
      setWhipPulse((current) => (current === 1 ? 2 : 1));
      if (whipTimer.current !== null) window.clearTimeout(whipTimer.current);
      whipTimer.current = window.setTimeout(() => setWhipPulse(0), 850);
    }).catch(() => {});
  }

  const draggingCharacter = dragVisualActive && feedPhase === "idle";
  const canResizeCharacter = !followPointer && feedPhase === "idle" && !draggingCharacter;
  const controlsVisible = hoveringCharacter
    && !followPointer
    && !moving
    && feedPhase === "idle"
    && !draggingCharacter
    && !resizingCharacter;
  const roamingMoving = feedPhase === "idle" && !draggingCharacter && roaming && moving;
  const movingDirection = direction < 0 ? -1 : 1;
  const shouldFlipForDirection = movingDirection !== roamingSourceDirectionByRunner[runnerId];
  const movingClass = roamingMoving && shouldFlipForDirection ? " roaming-direction-flipped" : "";
  const feedClass = feedPhase === "ready"
    ? " file-drop-ready"
    : feedPhase === "processing"
      ? " file-drop-ready"
      : feedPhase === "consuming" || feedPhase === "finishing"
      ? " file-drop-consuming"
      : "";
  const className = `character-window${feedClass}${controlsVisible ? " character-controls-visible" : ""}${draggingCharacter ? " character-dragging" : ""}${resizingCharacter ? " character-resizing" : ""}${typingMotion && !draggingCharacter && !roamingMoving && feedPhase === "idle" ? ` typing-motion-${typingMotion}` : ""}${followPointer ? " following-pointer" : ""}${roaming ? " roaming-mode" : ""}${roamingMoving ? ` roaming-moving${movingClass}` : ""}${whipPulse && !draggingCharacter ? ` whip-pulse-${whipPulse}` : ""}`;
  const characterStyle = roamingMoving
    ? ({ "--roam-size": roamingVisualScaleByRunner[runnerId] } as CSSProperties & { "--roam-size": number })
    : undefined;
  const image = feedPhase !== "idle"
    ? feedingFrames[feedFrame]
    : draggingCharacter
      ? grabbedFrames[grabbedFrame % grabbedFrames.length]
    : roamingMoving
      ? roamingFrames[roamingFrame % roamingFrames.length]
      : frames[frame];

  return (
    <div
      className={className}
      data-runner={runnerId}
      style={characterStyle}
      onPointerDown={startDrag}
      onPointerMove={continueDrag}
      onPointerUp={finishPointer}
      onPointerCancel={finishPointer}
      onPointerEnter={handlePointerEnter}
      onPointerLeave={handlePointerLeave}
      onDoubleClick={handleDoubleClick}
      onContextMenu={(event) => {
        event.preventDefault();
        void showCharacterContextMenu();
      }}
      title={
        roaming
          ? "모니터를 자유롭게 돌아다니는 중 · 우클릭으로 옵션 열기"
          : followPointer
            ? "마우스를 따라다니는 중"
            : "드래그해서 이동 · 더블클릭으로 채찍질 · 우클릭으로 옵션 열기"
      }
    >
      <img src={image} alt="RunDev 캐릭터" draggable={false} />
      {canResizeCharacter && (
        <button
          className="character-control character-resize-handle"
          type="button"
          aria-label="캐릭터 크기 조절"
          title={`크기 조절 (${Math.round(characterSize)}px)`}
          onPointerDown={startCharacterResize}
          onPointerMove={continueCharacterResize}
          onPointerUp={finishCharacterResizeGesture}
          onPointerCancel={finishCharacterResizeGesture}
        >
          <Maximize2 size={10} strokeWidth={2.4} />
        </button>
      )}
      <button
        className="character-control character-hide-button"
        type="button"
        aria-label="캐릭터 숨기기"
        title="숨기기"
        onPointerDown={(event) => event.stopPropagation()}
        onDoubleClick={(event) => event.stopPropagation()}
        onClick={() => void setCharacterWindowVisible(false)}
      >
        <X size={9} />
      </button>
    </div>
  );
}
