import { useEffect, useState, type CSSProperties } from "react";
import { subscribeSalaryAnchor, type SalaryAnchor } from "../services/salary";
import { useSalarySnapshot } from "../hooks/useSalarySnapshot";
import { SalaryCharacterLabel } from "./SalaryCharacterLabel";

export function CharacterSalaryWindow() {
  const snapshot = useSalarySnapshot();
  const [anchor, setAnchor] = useState<SalaryAnchor | null>(null);
  useEffect(() => {
    let active = true;
    let unlisten = () => {};
    void subscribeSalaryAnchor((value) => { if (active) setAnchor(value); }).then((off) => {
      if (active) unlisten = off; else off();
    }).catch(() => {});
    return () => { active = false; unlisten(); };
  }, []);
  const bubbleScale = anchor ? Math.max(0.75, Math.min(4, anchor.characterSize / 48)) : 1;
  return <main className="character-salary-window"
    data-bubble-below={anchor != null && anchor.y < anchor.characterSize / 2 + 85 * bubbleScale}
    style={anchor ? {
      "--salary-anchor-x": `${anchor.x}px`, "--salary-anchor-y": `${anchor.y}px`,
      "--salary-character-size": `${anchor.characterSize}px`, "--salary-bubble-scale": bubbleScale
    } as CSSProperties : undefined}>
    {anchor && <SalaryCharacterLabel snapshot={snapshot} bubble />}
  </main>;
}
