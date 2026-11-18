import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Check, Pin, Settings } from "lucide-react";
import { accentThemes, resolveTheme, type AccentTheme } from "../domain/themes";
import { getAccentTheme, previewTheme, saveAccentTheme } from "../services/appearance";

export function ThemeDialog({ onClose }: { onClose: () => void }) {
  const [selected, setSelected] = useState(() => resolveTheme(document.documentElement.dataset.accentTheme));
  const original = useRef(selected);
  const applied = useRef(false);
  const dialog = useRef<HTMLElement>(null);
  const [ready, setReady] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let active = true;
    void getAccentTheme().then(theme => {
      if (!active) return;
      original.current = theme;
      setSelected(theme); previewTheme(theme); setReady(true); setError("");
    }).catch(() => { if (active) setError("색상 설정을 읽지 못했습니다. 다시 시도해 주세요."); });
    return () => { active = false; };
  }, [attempt]);

  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.focus();
    return () => {
      if (!applied.current) previewTheme(original.current);
      if (previous?.isConnected) previous.focus();
    };
  }, []);

  useEffect(() => { if (ready) dialog.current?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus(); }, [ready]);

  function choose(theme: AccentTheme) { setSelected(theme); previewTheme(theme); }
  function close() { if (!busy) onClose(); }
  async function apply() {
    setBusy(true); setError("");
    try {
      await saveAccentTheme(selected.id);
      applied.current = true;
      onClose();
    } catch { setError("색상 테마를 저장하지 못했습니다. 다시 시도해 주세요."); }
    finally { setBusy(false); }
  }
  function handleKeys(event: KeyboardEvent<HTMLElement>) {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); close(); }
    if (event.key !== "Tab") return;
    const controls = dialog.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled):not([tabindex="-1"])');
    if (!controls?.length) return;
    const first = controls[0], last = controls[controls.length - 1];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  }
  function moveSelection(event: KeyboardEvent<HTMLDivElement>) {
    if (!ready || busy) return;
    const delta = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: 3, ArrowUp: -3 }[event.key];
    if (delta == null && event.key !== "Home" && event.key !== "End") return;
    event.preventDefault();
    const index = accentThemes.findIndex(theme => theme.id === selected.id);
    const next = event.key === "Home" ? 0 : event.key === "End" ? accentThemes.length - 1
      : (index + delta! + accentThemes.length) % accentThemes.length;
    choose(accentThemes[next]);
    event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus();
  }

  return <div className="dialog-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) close(); }}>
    <section ref={dialog} tabIndex={-1} className="account-dialog theme-dialog" role="dialog" aria-modal="true" aria-labelledby="theme-dialog-title" onKeyDown={handleKeys}>
      <h2 id="theme-dialog-title">색상 테마</h2>
      <p>색을 고르면 바로 미리 볼 수 있어요.</p>
      <div className="theme-preview" aria-label="강조색 미리보기">
        <span className="theme-preview-amount">₩123,456</span>
        <span className="theme-preview-pin"><Pin size={12} aria-hidden="true" /> 고정됨</span>
        <Settings size={13} aria-hidden="true" />
      </div>
      <div className="theme-options" role="radiogroup" aria-label="강조색" onKeyDown={moveSelection}>
        {accentThemes.map(theme => <button key={theme.id} type="button" role="radio" aria-checked={selected.id === theme.id}
          tabIndex={selected.id === theme.id ? 0 : -1} disabled={!ready || busy} onClick={() => choose(theme)}>
          <span className="theme-swatch" style={{ backgroundColor: theme.hex }}>{selected.id === theme.id && <Check size={11} aria-hidden="true" />}</span>
          <span>{theme.name}</span>
        </button>)}
      </div>
      <div className="theme-selection"><span>{selected.name}</span><span>{selected.hex}</span></div>
      {error && <p role="alert" className="theme-error">{error}</p>}
      {!ready && error && <button type="button" className="theme-retry" onClick={() => { setError(""); setAttempt(value => value + 1); }}>다시 시도</button>}
      <div className="dialog-actions">
        <button type="button" disabled={busy} onClick={close}>취소</button>
        <button type="button" className="theme-apply" disabled={!ready || busy} onClick={() => void apply()}>{busy ? "저장 중…" : "적용"}</button>
      </div>
    </section>
  </div>;
}
