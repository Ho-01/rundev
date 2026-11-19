import { useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { Terminal, X } from "lucide-react";
import type { TraitId, TraitProgress } from "../types/activity";
import { traitBoardEdges, traitBoardNodes } from "./traitBoard";
import "./traitBoard.css";

const positions = new Map<string, { x: number; y: number }>(traitBoardNodes.map(node => [node.id, node]));
positions.set("core", { x: 420, y: 270 });
type Camera = { x: number; y: number; scale: number };

export function TraitDialog({ progress, onUpgrade, onClose }: {
  progress: TraitProgress; onUpgrade: (id: TraitId) => void | Promise<void>; onClose: () => void;
}) {
  const [selectedId, setSelectedId] = useState("focus-ready");
  const [camera, setCamera] = useState<Camera>({ x: 0, y: 0, scale: .8 });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [dragging, setDragging] = useState(false);
  const dialog = useRef<HTMLElement>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; camera: Camera } | null>(null);
  const selected = traitBoardNodes.find(node => node.id === selectedId) ?? traitBoardNodes[0];
  const trait = progress.traits.find(item => item.id === selected.traitId);
  const atMax = !!trait && trait.level >= trait.maxLevel;
  const Icon = selected.icon;

  function center() {
    drag.current = null;
    setDragging(false);
    setCamera(current => ({ ...current, x: 0, y: 0 }));
  }
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    dialog.current?.querySelector<HTMLButtonElement>("[aria-label='특성 닫기']")?.focus({ preventScroll: true });
    return () => { if (previous?.isConnected) previous.focus({ preventScroll: true }); };
  }, []);
  function zoom(delta: number) {
    if (!viewport.current) return;
    setCamera(current => {
      const scale = Math.min(1.5, Math.max(.2, Math.round((current.scale + delta) * 10) / 10));
      return { scale, x: current.x * scale / current.scale, y: current.y * scale / current.scale };
    });
  }
  function pointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button")) return;
    drag.current = { x: event.clientX, y: event.clientY, camera };
    event.currentTarget.setPointerCapture(event.pointerId); setDragging(true);
  }
  function revealNode(x: number, y: number) {
    if (!viewport.current) return;
    const { clientWidth, clientHeight } = viewport.current;
    setCamera(current => {
      const px = clientWidth / 2 + current.x + (x - 420) * current.scale;
      const py = clientHeight / 2 + current.y + (y - 270) * current.scale;
      return px < 30 || px > clientWidth - 30 || py < 30 || py > clientHeight - 65
        ? { ...current, x: -(x - 420) * current.scale, y: -(y - 270) * current.scale } : current;
    });
  }
  async function upgrade() {
    if (!trait || busy || atMax || progress.availablePoints < trait.upgradeCost) return;
    setBusy(true); setError("");
    try { await onUpgrade(trait.id); }
    catch { setError("강화하지 못했습니다. 다시 시도해 주세요."); }
    finally { setBusy(false); }
  }
  const effect = trait ? `${trait.effectValue.toFixed(1)}${trait.effectUnit === "percent" ? "%" : " XP"}` : "";
  const nextEffect = trait ? `${trait.nextEffectValue.toFixed(1)}${trait.effectUnit === "percent" ? "%" : " XP"}` : "";
  const reward = trait ? trait.id === "reload"
    ? atMax ? `활동일 +${trait.exampleRewardValue.toFixed(0)} XP` : `활동일 +${trait.exampleRewardValue.toFixed(0)} → +${trait.nextExampleRewardValue.toFixed(0)} XP`
    : atMax ? `10 → ${trait.exampleRewardValue.toFixed(1)} XP` : `${trait.exampleRewardValue.toFixed(1)} → ${trait.nextExampleRewardValue.toFixed(1)} XP` : "";

  return <div className="dialog-backdrop trait-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget && !busy) onClose(); }}>
    <section ref={dialog} className="account-dialog trait-dialog" role="dialog" aria-modal="true" aria-label="개발자 특성"
      onKeyDown={event => {
        if (event.key === "Escape") { event.stopPropagation(); if (!busy) onClose(); }
        if (event.key === "Tab") {
          const controls = dialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)");
          if (!controls?.length) return;
          const first = controls[0], last = controls[controls.length - 1];
          if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus({ preventScroll: true }); }
          else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus({ preventScroll: true }); }
        }
      }}>
      <header className="trait-board-header"><div><strong>개발자 특성</strong><span>성장 보드</span></div><div className="trait-board-points"><span>사용 가능</span><b>◆ {progress.availablePoints}</b></div><button type="button" aria-label="특성 닫기" disabled={busy} onClick={onClose}><X size={15} /></button></header>
      <div ref={viewport} className={`trait-board-viewport${dragging ? " dragging" : ""}`} onPointerDown={pointerDown}
        onPointerMove={event => { if (drag.current) setCamera({ ...drag.current.camera, x: drag.current.camera.x + event.clientX - drag.current.x, y: drag.current.camera.y + event.clientY - drag.current.y }); }}
        onPointerUp={() => { drag.current = null; setDragging(false); }} onPointerCancel={() => { drag.current = null; setDragging(false); }}>
        <div className="trait-board-world" style={{ transform: `translate(${camera.x}px, ${camera.y}px) scale(${camera.scale}) translate(-420px, -270px)` }}>
          <div className="trait-board-orbit inner" /><div className="trait-board-orbit outer" />
          {traitBoardEdges.map(([from, to]) => {
            const a = positions.get(from)!, b = positions.get(to)!;
            return <i key={to} className={`trait-board-edge${to === selectedId ? " selected" : ""}`} style={{ left: a.x, top: a.y, width: Math.hypot(b.x - a.x, b.y - a.y), transform: `rotate(${Math.atan2(b.y - a.y, b.x - a.x)}rad)` }} />;
          })}
          <div className="trait-board-core"><Terminal size={20} /><b>RunDev</b></div>
          {traitBoardNodes.map((node, index) => {
            const value = progress.traits.find(item => item.id === node.traitId), NodeIcon = node.icon;
            return <button key={node.id} data-trait-node={node.id} type="button" className={`trait-board-node${node.traitId ? "" : " future"}`} aria-label={node.traitId ? `${node.name} 선택` : `확장 노드 ${index - 2} 선택`}
              aria-pressed={selectedId === node.id} title={node.traitId ? `${node.name} · Lv. ${value?.level ?? 0} / ${value?.maxLevel ?? 20}` : "확장 노드 · Coming soon"}
              style={{ left: node.x, top: node.y, "--trait-progress": `${value ? Math.max(0, Math.min(1, value.level / value.maxLevel)) * 360 : 0}deg` } as CSSProperties}
              onFocus={() => revealNode(node.x, node.y)} onClick={() => { setSelectedId(node.id); setError(""); }}>
              <span><NodeIcon size={17} /><small>{value?.level ?? "–"}</small></span>
            </button>;
          })}
        </div>
        <div className="trait-board-camera"><button type="button" aria-label="보드 축소" disabled={camera.scale <= .2} onClick={() => zoom(-.1)}>−</button><span>{Math.round(camera.scale * 100)}%</span><button type="button" aria-label="보드 확대" disabled={camera.scale >= 1.5} onClick={() => zoom(.1)}>+</button><button type="button" onClick={center}>중앙</button></div>
      </div>
      <div className="trait-board-legend"><span><i />특성 3개</span><span><i />확장 예정 13개</span></div>
      <section className="trait-board-detail" aria-label="선택한 특성 상세">
        <header><div><Icon size={18} /><strong>{selected.name}</strong></div><span>{trait ? `Lv. ${trait.level} / ${trait.maxLevel}` : "확장 예정"}</span></header>
        <p>{trait ? selected.description : "새로운 특성이 추가될 공간입니다."}</p>
        {trait ? <div className="trait-board-detail-body"><div><span>현재 → 다음 효과</span><b>{effect} → <em>{atMax ? "MAX" : nextEffect}</em></b></div><div><span>{trait.id === "reload" ? "매일 첫 집중 20분" : "평균 보상 환산"}</span><b>{reward}</b></div><button type="button" aria-label={`${selected.name} 강화`} disabled={busy || atMax || progress.availablePoints < trait.upgradeCost} onClick={() => void upgrade()}>{busy ? "강화 중…" : atMax ? "최대 레벨" : progress.availablePoints < trait.upgradeCost ? "포인트 부족" : "강화"} · ◆ {trait.upgradeCost}</button></div>
          : <div className="trait-board-coming-soon">Coming soon<span>새로운 개발자 특성이 준비되면 이곳에서 확인할 수 있어요.</span></div>}
        {error && <p role="alert" className="trait-board-error">{error}</p>}
      </section>
    </section>
  </div>;
}
