import type { FocusActivityUpdate } from "../types/activity";

export const DEVELOPMENT_MESSAGES = [
  "미래의 나를 구하는 중",
  "코드를 조련하는 중",
  "키보드에 불을 붙이는 중",
  "버그의 흔적을 추적하는 중",
  "로직에 생명을 불어넣는 중",
  "아이디어를 현실로 만드는 중",
  "다음 커밋을 빚는 중",
  "버그와 한판 붙는 중",
  "시스템을 성장시키는 중",
  "코드 세계를 개척하는 중"
] as const;

export const AWAY_MESSAGES = [
  "잠깐 샛길을 탐험하는 중",
  "다음 개발을 위한 충전 중",
  "사이드 퀘스트 진행 중",
  "아이디어를 수집하는 중",
  "코드 너머의 단서를 찾는 중",
  "새로운 영감을 줍는 중",
  "머릿속 퍼즐을 맞추는 중",
  "새로운 단서를 찾아보는 중",
  "개발력을 재충전하는 중",
  "다음 수를 고민하는 중",
  "아이디어 인벤토리를 채우는 중",
  "코드로 돌아갈 길을 찾는 중",
  "잠깐 시야를 넓히는 중",
  "숨겨진 힌트를 수집하는 중"
] as const;

export type ActivityStatus = {
  tone: "development" | "away" | "idle";
  appName: string | null;
  message: string;
};

function randomMessage(messages: readonly string[]) {
  return messages[Math.floor(Math.random() * messages.length)];
}

export function buildActivityStatus(
  activity: FocusActivityUpdate | null
): ActivityStatus {
  if (!activity?.active || !activity.appName) {
    return { tone: "idle", appName: null, message: "개발 활동 대기 중" };
  }

  const messages = activity.focused ? DEVELOPMENT_MESSAGES : AWAY_MESSAGES;
  return {
    tone: activity.focused ? "development" : "away",
    appName: activity.appName,
    message: randomMessage(messages)
  };
}
