import { Check, LockKeyhole, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { runnerPreviewFrame } from "../assets/runners";
import { equipRunnerSkin, getRunnerSkinCollection } from "../services/rundev";
import type { RunnerId, RunnerSelection, RunnerSkinCollection, RunnerSkinId } from "../types/activity";

type RunnerCollectionDialogProps = {
  selection: RunnerSelection | null;
  characterWindowVisible: boolean;
  onClose: () => void;
  onSelectRunner: (runnerId: RunnerId) => Promise<void>;
  onSelectionChanged: () => Promise<void>;
  onToggleCharacterWindow: () => Promise<void>;
};

function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (hours === 0) return `${minutes}분`;
  return minutes > 0 ? `${hours}시간 ${minutes}분` : `${hours}시간`;
}

export function RunnerCollectionDialog({
  selection,
  characterWindowVisible,
  onClose,
  onSelectRunner,
  onSelectionChanged,
  onToggleCharacterWindow
}: RunnerCollectionDialogProps) {
  const [collection, setCollection] = useState<RunnerSkinCollection | null>(null);
  const [activeRunnerId, setActiveRunnerId] = useState<RunnerId>(selection?.runnerId ?? "coding-cat");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    const next = await getRunnerSkinCollection();
    setCollection(next);
    setActiveRunnerId((current) =>
      next.characters.some((character) => character.runnerId === current)
        ? current
        : next.selected.runnerId
    );
  }

  useEffect(() => {
    void reload().catch((loadError) => {
      setError(loadError instanceof Error ? loadError.message : String(loadError));
    });
  }, []);

  useEffect(() => {
    if (selection) setActiveRunnerId(selection.runnerId);
  }, [selection?.runnerId]);

  const activeCharacter = useMemo(
    () => collection?.characters.find((character) => character.runnerId === activeRunnerId) ?? null,
    [activeRunnerId, collection]
  );
  const totalActiveSeconds = collection?.totalDevelopmentSeconds ?? 0;

  async function selectCharacter(runnerId: RunnerId) {
    if (busy || runnerId === selection?.runnerId) {
      setActiveRunnerId(runnerId);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await onSelectRunner(runnerId);
      setActiveRunnerId(runnerId);
      await reload();
    } catch (selectionError) {
      setError(selectionError instanceof Error ? selectionError.message : String(selectionError));
    } finally {
      setBusy(false);
    }
  }

  async function equipSkin(skinId: RunnerSkinId) {
    if (!activeCharacter || busy) return;
    setBusy(true);
    setError(null);
    try {
      await equipRunnerSkin(activeCharacter.runnerId, skinId);
      await onSelectionChanged();
      await reload();
    } catch (equipError) {
      setError(equipError instanceof Error ? equipError.message : String(equipError));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="dialog-backdrop" role="presentation">
      <section
        className="account-dialog runner-collection-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="runner-collection-title"
      >
        <header className="runner-collection-heading">
          <div>
            <h2 id="runner-collection-title">개발자 컬렉션</h2>
          </div>
          <button type="button" onClick={onClose} aria-label="캐릭터 컬렉션 닫기" title="닫기">
            <X size={15} />
          </button>
        </header>
        <p>캐릭터를 고르고, 해금한 외형을 장착하세요.</p>

        <section className="runner-collection-section" aria-label="캐릭터 선택">
          <div className="runner-collection-label"><strong>캐릭터</strong><small>{collection?.characters.length ?? 0}명</small></div>
          <div className="runner-character-grid">
            {collection?.characters.map((character) => {
              const selected = character.runnerId === activeRunnerId;
              return (
                <button
                  key={character.runnerId}
                  type="button"
                  className={selected ? "selected" : ""}
                  aria-pressed={selected}
                  disabled={busy}
                  onClick={() => void selectCharacter(character.runnerId)}
                >
                  <img src={runnerPreviewFrame(character.runnerId)} alt="" aria-hidden="true" />
                  <span>{character.name}</span>
                </button>
              );
            })}
          </div>
        </section>

        <section className="runner-collection-section" aria-label="스킨 선택">
          <div className="runner-collection-label">
            <strong>{activeCharacter?.name ?? "캐릭터"}의 외형</strong>
            <small>{activeCharacter?.skins.filter((skin) => skin.owned).length ?? 0}/{activeCharacter?.skins.length ?? 0} 해금</small>
          </div>
          <div className="runner-skin-grid">
            {activeCharacter?.skins.map((skin) => {
              const progress = skin.requiredActiveSeconds === 0
                ? 100
                : Math.min(100, (totalActiveSeconds / skin.requiredActiveSeconds) * 100);
              const active = collection?.selected.runnerId === activeCharacter.runnerId && skin.equipped;
              return (
                <article key={skin.skinId} className={`runner-skin-card${active ? " equipped" : ""}${skin.owned ? "" : " locked"}`}>
                  <img src={runnerPreviewFrame(activeCharacter.runnerId, skin.skinId)} alt="" aria-hidden="true" />
                  <div>
                    <strong>{skin.name}</strong>
                    <p>{skin.description}</p>
                  </div>
                  {skin.owned ? (
                    <button
                      type="button"
                      disabled={busy || active}
                      onClick={() => void equipSkin(skin.skinId)}
                    >
                      {active ? <><Check size={12} /> 장착됨</> : "장착"}
                    </button>
                  ) : (
                    <div className="runner-skin-lock">
                      <span><LockKeyhole size={11} /> 누적 집중 {formatDuration(skin.requiredActiveSeconds)}</span>
                      <progress max={skin.requiredActiveSeconds} value={Math.min(totalActiveSeconds, skin.requiredActiveSeconds)} aria-label={`${skin.name} 해금 진행도`} />
                      <small>{formatDuration(totalActiveSeconds)} / {formatDuration(skin.requiredActiveSeconds)}</small>
                    </div>
                  )}
                  {!skin.owned && <i className="runner-skin-lock-icon" aria-hidden="true"><LockKeyhole size={13} /></i>}
                </article>
              );
            })}
          </div>
        </section>

        {error && <p className="runner-collection-error" role="alert">{error}</p>}
        <button
          type="button"
          className="character-window-setting"
          role="switch"
          aria-checked={characterWindowVisible}
          onClick={() => void onToggleCharacterWindow()}
        >
          <span>
            <strong>화면에 캐릭터 띄우기</strong>
            <small>다른 앱 위에서도 타이핑 리듬을 보여줍니다.</small>
          </span>
          <i aria-hidden="true"><b /></i>
        </button>
      </section>
    </div>
  );
}
