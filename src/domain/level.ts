export type LevelTierId =
  | "sprout"
  | "rookie"
  | "maker"
  | "debugger"
  | "hacker"
  | "systems"
  | "architect"
  | "lead"
  | "master"
  | "legend";

export type LevelTier = {
  id: LevelTierId;
  name: string;
  minLevel: number;
  maxLevel: number | null;
};

export const levelTiers: LevelTier[] = [
  { id: "sprout", name: "새싹 개발자", minLevel: 1, maxLevel: 4 },
  { id: "rookie", name: "루키 빌더", minLevel: 5, maxLevel: 9 },
  { id: "maker", name: "코드 메이커", minLevel: 10, maxLevel: 19 },
  { id: "debugger", name: "디버거", minLevel: 20, maxLevel: 34 },
  { id: "hacker", name: "코드 해커", minLevel: 35, maxLevel: 49 },
  { id: "systems", name: "시스템 설계자", minLevel: 50, maxLevel: 69 },
  { id: "architect", name: "아키텍트", minLevel: 70, maxLevel: 99 },
  { id: "lead", name: "테크 리드", minLevel: 100, maxLevel: 149 },
  { id: "master", name: "마스터 개발자", minLevel: 150, maxLevel: 249 },
  { id: "legend", name: "레전드 개발자", minLevel: 250, maxLevel: null }
];

export function getLevelTier(level: number) {
  return (
    [...levelTiers]
      .reverse()
      .find((tier) => level >= tier.minLevel) ?? levelTiers[0]
  );
}

export function getNextLevelTier(level: number) {
  return levelTiers.find((tier) => tier.minLevel > level) ?? null;
}
