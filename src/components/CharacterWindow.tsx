import { useEffect, useRef, useState, type PointerEvent } from "react";
import { X } from "lucide-react";
import { desktopRunnerFramesById } from "../assets/runners/desktop";
import type { RunnerId } from "../types/activity";
import {
  dragCharacterWindow,
  getCharacterWindowState,
  getCharacterRunner,
  setCharacterWindowVisible,
  showCharacterContextMenu,
  subscribeCharacterWindowState,
  subscribeRunnerSelection,
  subscribeTypingPulse
} from "../services/characterWindow";

export function CharacterWindow() {
  const [runnerId, setRunnerId] = useState<RunnerId>("coding-cat");
  const [frame, setFrame] = useState(0);
  const [typingMotion, setTypingMotion] = useState(0);
  const [followPointer, setFollowPointer] = useState(false);
  const lastTypingAt = useRef(Number.NEGATIVE_INFINITY);
  const typingEnergy = useRef(0);
  const rescheduleAnimation = useRef<() => void>(() => {});
  const [windowVisible, setWindowVisible] = useState(false);
  const frames = desktopRunnerFramesById[runnerId];

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
      if (!windowVisible) return;
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
  }, [frames.length, windowVisible]);

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
      if (startingSession) {
        setFrame((current) => (current + 1) % frames.length);
        setTypingMotion(1);
      }
      rescheduleAnimation.current();
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, [frames.length]);

  function startDrag(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || followPointer) return;
    event.preventDefault();
    void dragCharacterWindow();
  }

  return (
    <div
      className={`character-window${typingMotion ? ` typing-motion-${typingMotion}` : ""}${followPointer ? " following-pointer" : ""}`}
      onPointerDown={startDrag}
      onContextMenu={(event) => {
        event.preventDefault();
        void showCharacterContextMenu();
      }}
      title={followPointer ? "마우스 따라다니는 중" : "드래그해서 이동 · 우클릭으로 옵션 열기"}
    >
      <img src={frames[frame]} alt="RunDev 캐릭터" draggable={false} />
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
