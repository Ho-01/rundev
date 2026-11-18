export const accentThemes = [
  { id: "default", name: "기본 그린", hex: "#B9E69B" },
  { id: "periwinkle", name: "퍼리윙클", hex: "#A5B4FC" },
  { id: "lavender", name: "라벤더", hex: "#C4B5FD" },
  { id: "sky-blue", name: "스카이 블루", hex: "#7DB9FF" },
  { id: "ice-cyan", name: "아이스 시안", hex: "#76D8EA" },
  { id: "steel-blue", name: "스틸 블루", hex: "#9EBBDB" },
  { id: "silver", name: "실버", hex: "#CBD2DF" },
  { id: "champagne", name: "샴페인", hex: "#DCC9A3" },
  { id: "honey-amber", name: "허니 앰버", hex: "#E9BF74" },
  { id: "peach", name: "피치", hex: "#F4B59A" },
  { id: "coral", name: "코랄", hex: "#F29CA5" },
  { id: "rose", name: "로즈", hex: "#E9A9CF" },
  { id: "orchid", name: "오키드", hex: "#D0A3F0" }
] as const;

export type AccentTheme = typeof accentThemes[number];
export type AccentThemeId = AccentTheme["id"];
export const resolveTheme = (id: string | null | undefined): AccentTheme =>
  accentThemes.find(theme => theme.id === id) ?? accentThemes[0];
