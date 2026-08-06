import { useEffect, useRef, useState, type PointerEvent } from "react";
import { X } from "lucide-react";
import { desktopRunnerFramesById } from "../assets/runners/desktop";
import type { RunnerId } from "../types/activity";
import {
  dragCharacterWindow,
  getCharacterRunner,
  setCharacterWindowVisible,
  subscribeRunnerSelection,
  subscribeTypingPulse
} from "../services/characterWindow";

export function CharacterWindow() {
  const [runnerId, setRunnerId] = useState<RunnerId>("coding-cat");
  const [frame, setFrame] = useState(0);
  const [typingMotion, setTypingMotion] = useState<"a" | "b" | null>(null);
  const lastTypingAt = useRef(0);
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
    const timer = window.setInterval(() => {
      if (Date.now() - lastTypingAt.current < 400) return;
      setTypingMotion(null);
      setFrame((current) => (current + 1) % frames.length);
    }, 170);
    return () => window.clearInterval(timer);
  }, [frames.length]);

  useEffect(() => {
    let unlisten = () => {};
    void subscribeTypingPulse(() => {
      lastTypingAt.current = Date.now();
      setFrame((current) => (current + 1) % frames.length);
      setTypingMotion((current) => current === "a" ? "b" : "a");
    }).then((next) => {
      unlisten = next;
    });
    return () => unlisten();
  }, [frames.length]);

  function startDrag(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    void dragCharacterWindow();
  }

  return (
    <div className={`character-window${typingMotion ? ` typing-${typingMotion}` : ""}`} onPointerDown={startDrag} title="드래그해서 이동">
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
