import { Flame, Target, Boxes, RotateCcw, Plus } from "lucide-react";
import type { TraitId, TraitProgress } from "../types/activity";
const traitCopy: Record<TraitId, { name: string; description: string }> = {
  "focus-ready": { name: "몰입 준비", description: "집중시간 XP 획득량 증가" },
  "hot-keyboard": { name: "뜨거운 키보드", description: "키보드 XP 획득량 증가" },
  reload: { name: "재장전", description: "매일 첫 집중 20분에 활동 보너스" }
};

export function TraitDialog({ progress, onUpgrade, onClose }: { progress: TraitProgress; onUpgrade: (id: TraitId) => void; onClose: () => void }) {
  const iconFor = (id: TraitId) => {
    if (id === "focus-ready") return <Target size={18} />;
    if (id === "hot-keyboard") return <Flame size={18} />;
    if (id === "reload") return <RotateCcw size={18} />;
    return <Boxes size={18} />;
  };
  const actualEffect = (trait: TraitProgress["traits"][number], atMax: boolean) => {
    if (trait.id === "reload") {
      return atMax
        ? `활동일 +${trait.exampleRewardValue.toFixed(0)} XP`
        : `활동일 +${trait.exampleRewardValue.toFixed(0)} → +${trait.nextExampleRewardValue.toFixed(0)} XP`;
    }
    return atMax
      ? `10 → ${trait.exampleRewardValue.toFixed(1)} XP`
      : `${trait.exampleRewardValue.toFixed(1)} → ${trait.nextExampleRewardValue.toFixed(1)} XP`;
  };
  const actualLabel = (id: TraitId) => {
    if (id === "reload") return "매일 첫 집중 20분";
    return "평균 보상 환산";
  };
  return <div className="dialog-backdrop trait-backdrop" role="presentation" onMouseDown={onClose}>
    <section className="account-dialog trait-dialog" role="dialog" aria-modal="true" aria-label="개발자 특성" onMouseDown={(event) => event.stopPropagation()}>
      <header className="trait-dialog-heading"><div><strong>개발자 특성</strong><span>나만의 개발 스타일을 강화하세요</span></div><div className="trait-points"><span>사용 가능</span><b>◆ {progress.availablePoints}</b></div></header>
      <div className="trait-board">{progress.traits.map((trait) => {
        const atMax = trait.level >= trait.maxLevel;
        return <article key={trait.id} className={`trait-card ${trait.id}${progress.availablePoints >= trait.upgradeCost && !atMax ? " upgrade-ready" : ""}`}>
          <header><i>{iconFor(trait.id)}</i><span>Lv. {trait.level}<small>/ {trait.maxLevel}</small></span></header>
          <strong>{traitCopy[trait.id].name}</strong>
          <p>{traitCopy[trait.id].description}</p>
          <div className="trait-level-track"><i style={{ width: `${trait.level / trait.maxLevel * 100}%` }} /></div>
          <div className="trait-effect"><b>{trait.effectValue.toFixed(1)}{trait.effectUnit === "percent" ? "%" : " XP"}</b><span>{atMax ? "MAX" : `다음 ${trait.nextEffectValue.toFixed(1)}${trait.effectUnit === "percent" ? "%" : " XP"}`}</span></div>
          <div className="trait-actual"><span>{actualLabel(trait.id)}</span><b>{actualEffect(trait, atMax)}</b></div>
          <button type="button" disabled={progress.availablePoints < trait.upgradeCost || atMax} onClick={() => onUpgrade(trait.id)} aria-label={`${traitCopy[trait.id].name} 강화`}>{atMax ? "최대 레벨" : <><Plus size={12}/> ◆ {trait.upgradeCost} 강화</>}</button>
        </article>;
      })}</div>
      <div className="dialog-actions"><button type="button" onClick={onClose}>닫기</button></div>
    </section>
  </div>;
}

