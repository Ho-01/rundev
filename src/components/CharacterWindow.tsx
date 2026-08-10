import { useEffect, useRef, useState, type PointerEvent } from "react";
import { X } from "lucide-react";
import type { DragDropEvent } from "@tauri-apps/api/webview";
import { desktopRunnerFramesById } from "../assets/runners/desktop";
import { feedingRunnerFramesById } from "../assets/runners/feeding";
import type { RunnerId } from "../types/activity";
import {
  beginCharacterFileDrop,
  dragCharacterWindow,
  endCharacterFileDrop,
  getCharacterWindowState,
  getCharacterRunner,
  setCharacterWindowVisible,
  showCharacterContextMenu,
  subscribeCharacterFileDrop,
  subscribeCharacterWindowState,
  subscribeRunnerSelection,
  subscribeTypingPulse,
  trashDroppedFiles
} from "../services/characterWindow";

const FEED_FRAME_INTERVAL_MS = 200;
const FEED_SWALLOW_HOLD_MS = 280;
const FEED_FRAME_SEQUENCE = [1, 2, 1, 2, 3] as const;
type FeedPhase = "idle" | "ready" | "processing" | "consuming" | "finishing";

export function CharacterWindow() {
  const [runnerId, setRunnerId] = useState<RunnerId>("coding-cat");
  const [frame, setFrame] = useState(0);
  const [typingMotion, setTypingMotion] = useState(0);
  const [followPointer, setFollowPointer] = useState(false);
  const [feedPhase, setFeedPhase] = useState<FeedPhase>("idle");
  const [feedFrame, setFeedFrame] = useState(0);
  const lastTypingAt = useRef(Number.NEGATIVE_INFINITY);
  const typingEnergy = useRef(0);
  const rescheduleAnimation = useRef<() => void>(() => {});
  const feedPhaseRef = useRef<FeedPhase>("idle");
  const feedBeginPromise = useRef<Promise<void> | null>(null);
  const feedFinishTimer = useRef<number | null>(null);
  const [windowVisible, setWindowVisible] = useState(false);
  const frames = desktopRunnerFramesById[runnerId];
  const feedingFrames = feedingRunnerFramesById[runnerId];

  useEffect(() => {
    void getCharacterRunner().then(({ runnerId }) => setRunnerId(runnerId));
    let unlisten = () => {};
    void subscribeRunnerSelection(({ runnerId }) => setRunnerId(runnerId)).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, []);

  useEffect(() => {
    void getCharacterWindowState().then(({ followPointer, visible }) => {
      setFollowPointer(followPointer);
      setWindowVisible(visible);
    });
    let unlisten = () => {};
    void subscribeCharacterWindowState(({ followPointer, visible }) => {
      setFollowPointer(followPointer);
      setWindowVisible(visible);
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, []);

  useEffect(() => {
    let timer = 0;

    function schedule() {
      if (!windowVisible || feedPhase !== "idle") return;
      const now = performance.now();
      const sinceTyping = now - lastTypingAt.current;
      const typing = sinceTyping < 420;
      const delay = typing
        ? 120 - typingEnergy.current * 75
        : 170;
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
        setTypingMotion((current) => nextTyping ? (current + 1) % 4 : 0);
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
  }, [feedPhase, frames.length, windowVisible]);

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
      if (startingSession && feedPhase === "idle") {
        setFrame((current) => (current + 1) % frames.length);
        setTypingMotion(1);
      }
      rescheduleAnimation.current();
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, [feedPhase, frames.length]);

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

  function startDrag(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || followPointer || feedPhase !== "idle") return;
    event.preventDefault();
    void dragCharacterWindow();
  }

  const feedClass = feedPhase === "idle"
    ? ""
    : feedPhase === "ready" || feedPhase === "processing"
      ? " file-drop-ready"
      : " file-drop-consuming";
  const image = feedPhase === "idle" ? frames[frame] : feedingFrames[feedFrame];

  return (
    <div
      className={`character-window${feedClass}${typingMotion && feedPhase === "idle" ? ` typing-motion-${typingMotion}` : ""}${followPointer ? " following-pointer" : ""}`}
      onPointerDown={startDrag}
      onContextMenu={(event) => {
        event.preventDefault();
        void showCharacterContextMenu();
      }}
      title={followPointer ? "마우스 따라다니는 중" : "드래그해서 이동 · 우클릭으로 옵션 열기"}
    >
      <img src={image} alt="RunDev 캐릭터" draggable={false} />
      <button
        type="button"
        aria-label="캐릭터 숨기기"
        title="숨기기"
        onPointerDown={(event) => event.stopPropagation()}
        onClick={() => void setCharacterWindowVisible(false)}
      >
        <X size={9} />
      </button>
    </div>
  );
}
