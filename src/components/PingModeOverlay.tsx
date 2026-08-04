import { Cable, Circle, Radio } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import {
  PING_DRAG_THRESHOLD,
  actionForPingSlot,
  pingSlotForDelta,
  type PingSlot
} from "./pingMode";

type Point = { x: number; y: number };
type Gesture = Point & { pointerId: number; dragging: boolean };
type PingBurst = Point & { id: number };

type Props = {
  rootRef: RefObject<HTMLElement | null>;
  onWhip: () => void;
};

function isTypingTarget(target: EventTarget | null) {
  const element = target instanceof HTMLElement ? target : null;
  return Boolean(
    element?.isContentEditable ||
    element?.closest("input, textarea, select, [contenteditable='true']")
  );
}

export function PingModeOverlay({ rootRef, onWhip }: Props) {
  const [armed, setArmed] = useState(false);
  const [gesture, setGesture] = useState<Gesture | null>(null);
  const [pointer, setPointer] = useState<Point | null>(null);
  const [selectedSlot, setSelectedSlot] = useState<PingSlot>("center");
  const [bursts, setBursts] = useState<PingBurst[]>([]);
  const armedRef = useRef(false);
  const gestureRef = useRef<Gesture | null>(null);
  const suppressClickRef = useRef(false);
  const cancelledPointerRef = useRef<number | null>(null);
  const nextBurstIdRef = useRef(0);

  function setMode(next: boolean) {
    armedRef.current = next;
    setArmed(next);
    rootRef.current?.classList.toggle("ping-mode-active", next);
    if (!next) {
      gestureRef.current = null;
      setGesture(null);
      setPointer(null);
      setSelectedSlot("center");
    }
  }

  function localPoint(clientX: number, clientY: number) {
    const rect = rootRef.current?.getBoundingClientRect();
    if (!rect) return null;
    return { x: clientX - rect.left, y: clientY - rect.top };
  }

  function emitPing(point: Point) {
    const id = ++nextBurstIdRef.current;
    setBursts((current) => [...current, { ...point, id }]);
    window.setTimeout(() => {
      setBursts((current) => current.filter((burst) => burst.id !== id));
    }, 850);
  }

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;

    const releaseClickSuppressionSoon = () => {
      window.setTimeout(() => {
        suppressClickRef.current = false;
      }, 0);
    };

    const cancelGesture = () => {
      const current = gestureRef.current;
      cancelledPointerRef.current = current?.pointerId ?? null;
      suppressClickRef.current = Boolean(current);
      setMode(false);
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.repeat || event.ctrlKey || event.altKey || event.metaKey || isTypingTarget(event.target)) {
        return;
      }
      if (event.key.toLowerCase() === "g") {
        event.preventDefault();
        if (armedRef.current) cancelGesture();
        else setMode(true);
      } else if (event.key === "Escape" && armedRef.current) {
        event.preventDefault();
        cancelGesture();
      }
    };

    const onPointerDown = (event: PointerEvent) => {
      if (!armedRef.current) return;
      if (event.button === 2) {
        event.preventDefault();
        event.stopPropagation();
        suppressClickRef.current = true;
        cancelledPointerRef.current = gestureRef.current?.pointerId ?? null;
        setMode(false);
        return;
      }
      if (event.button !== 0) return;
      const point = localPoint(event.clientX, event.clientY);
      if (!point) return;
      event.preventDefault();
      event.stopPropagation();
      suppressClickRef.current = true;
      const nextGesture = { ...point, pointerId: event.pointerId, dragging: false };
      gestureRef.current = nextGesture;
      setGesture(nextGesture);
      setPointer(point);
      setSelectedSlot("center");
      (event.target as Element | null)?.setPointerCapture?.(event.pointerId);
    };

    const onPointerMove = (event: PointerEvent) => {
      const current = gestureRef.current;
      if (!armedRef.current || !current || current.pointerId !== event.pointerId) return;
      const point = localPoint(event.clientX, event.clientY);
      if (!point) return;
      event.preventDefault();
      const deltaX = point.x - current.x;
      const deltaY = point.y - current.y;
      const dragging = current.dragging || Math.hypot(deltaX, deltaY) >= PING_DRAG_THRESHOLD;
      const nextGesture = { ...current, dragging };
      gestureRef.current = nextGesture;
      setGesture(nextGesture);
      setPointer(point);
      setSelectedSlot(dragging ? pingSlotForDelta(deltaX, deltaY) : "center");
    };

    const onPointerUp = (event: PointerEvent) => {
      if (cancelledPointerRef.current === event.pointerId) {
        event.preventDefault();
        event.stopPropagation();
        cancelledPointerRef.current = null;
        suppressClickRef.current = true;
        releaseClickSuppressionSoon();
        return;
      }
      const current = gestureRef.current;
      if (!armedRef.current || !current || current.pointerId !== event.pointerId) return;
      event.preventDefault();
      event.stopPropagation();
      const point = localPoint(event.clientX, event.clientY) ?? current;
      const slot = current.dragging
        ? pingSlotForDelta(point.x - current.x, point.y - current.y)
        : "center";
      const action = actionForPingSlot(slot);
      if (action === "basic-ping") emitPing(current);
      if (action === "whip") onWhip();
      setMode(false);
      releaseClickSuppressionSoon();
    };

    const onPointerCancel = (event: PointerEvent) => {
      const current = gestureRef.current;
      if (!current || current.pointerId !== event.pointerId) return;
      event.preventDefault();
      cancelGesture();
      releaseClickSuppressionSoon();
    };

    const onContextMenu = (event: MouseEvent) => {
      if (!armedRef.current && !suppressClickRef.current) return;
      event.preventDefault();
      event.stopPropagation();
      if (cancelledPointerRef.current === null) suppressClickRef.current = false;
      setMode(false);
    };

    const onClick = (event: MouseEvent) => {
      if (!armedRef.current && !suppressClickRef.current) return;
      event.preventDefault();
      event.stopPropagation();
      suppressClickRef.current = false;
    };

    const cancel = () => {
      cancelledPointerRef.current = null;
      suppressClickRef.current = false;
      setMode(false);
    };
    window.addEventListener("keydown", onKeyDown, true);
    root.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("pointermove", onPointerMove, true);
    window.addEventListener("pointerup", onPointerUp, true);
    window.addEventListener("pointercancel", onPointerCancel, true);
    window.addEventListener("contextmenu", onContextMenu, true);
    window.addEventListener("click", onClick, true);
    window.addEventListener("blur", cancel);
    return () => {
      root.classList.remove("ping-mode-active");
      window.removeEventListener("keydown", onKeyDown, true);
      root.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("pointermove", onPointerMove, true);
      window.removeEventListener("pointerup", onPointerUp, true);
      window.removeEventListener("pointercancel", onPointerCancel, true);
      window.removeEventListener("contextmenu", onContextMenu, true);
      window.removeEventListener("click", onClick, true);
      window.removeEventListener("blur", cancel);
    };
  }, [onWhip, rootRef]);

  return (
    <div className={`ping-layer${armed ? " armed" : ""}`} aria-hidden="true">
      {gesture?.dragging ? (
        <div className="ping-wheel" style={{ left: gesture.x, top: gesture.y }}>
          <PingWheelItem slot="up" selected={selectedSlot === "up"} enabled label="채찍">
            <Cable size={17} />
          </PingWheelItem>
          {(["right", "down", "left"] as const).map((slot) => (
            <PingWheelItem key={slot} slot={slot} selected={selectedSlot === slot} label="준비 중">
              <Circle size={9} />
            </PingWheelItem>
          ))}
          <PingWheelItem slot="center" selected={selectedSlot === "center"} enabled label="기본 핑">
            <Radio size={16} />
          </PingWheelItem>
        </div>
      ) : null}
      {gesture?.dragging && pointer ? (
        <div className="ping-pointer" style={{ left: pointer.x, top: pointer.y }} />
      ) : null}
      {bursts.map((burst) => (
        <span key={burst.id} className="ping-burst" style={{ left: burst.x, top: burst.y }}>
          <i /><i /><i />
        </span>
      ))}
    </div>
  );
}

function PingWheelItem({
  slot,
  selected,
  enabled = false,
  label,
  children
}: {
  slot: PingSlot;
  selected: boolean;
  enabled?: boolean;
  label: string;
  children: ReactNode;
}) {
  return (
    <span className={`ping-wheel-item ${slot}${selected ? " selected" : ""}${enabled ? " enabled" : " disabled"}`}>
      {children}
      <em>{label}</em>
    </span>
  );
}
