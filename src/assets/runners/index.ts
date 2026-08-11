import type { RunnerId, RunnerSkinId } from "../../types/activity";
import codingCat1 from "./ui/coding-cat/01.png";
import codingCat2 from "./ui/coding-cat/02.png";
import codingCat3 from "./ui/coding-cat/03.png";
import codingCat4 from "./ui/coding-cat/04.png";
import codingFish1 from "./ui/coding-fish/01.png";
import codingFish2 from "./ui/coding-fish/02.png";
import codingFish3 from "./ui/coding-fish/03.png";
import codingFish4 from "./ui/coding-fish/04.png";
import codingOrangeCat1 from "./ui/coding-orange-cat/01.png";
import codingOrangeCat2 from "./ui/coding-orange-cat/02.png";
import codingOrangeCat3 from "./ui/coding-orange-cat/03.png";
import codingOrangeCat4 from "./ui/coding-orange-cat/04.png";
import codingShrimp1 from "./ui/coding-shrimp/01.png";
import codingShrimp2 from "./ui/coding-shrimp/02.png";
import codingShrimp3 from "./ui/coding-shrimp/03.png";
import codingShrimp4 from "./ui/coding-shrimp/04.png";
import codingVtuber1 from "./ui/coding-vtuber/01.png";
import codingVtuber2 from "./ui/coding-vtuber/02.png";
import codingVtuber3 from "./ui/coding-vtuber/03.png";
import codingVtuber4 from "./ui/coding-vtuber/04.png";
import poolPartyVtuber1 from "./ui/coding-vtuber/pool-party/01.png";
import poolPartyVtuber2 from "./ui/coding-vtuber/pool-party/02.png";
import poolPartyVtuber3 from "./ui/coding-vtuber/pool-party/03.png";
import poolPartyVtuber4 from "./ui/coding-vtuber/pool-party/04.png";

export const runnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": [codingCat1, codingCat2, codingCat3, codingCat4],
  "coding-fish": [codingFish1, codingFish2, codingFish3, codingFish4],
  "coding-orange-cat": [
    codingOrangeCat1,
    codingOrangeCat2,
    codingOrangeCat3,
    codingOrangeCat4
  ],
  "coding-shrimp": [
    codingShrimp1,
    codingShrimp2,
    codingShrimp3,
    codingShrimp4
  ],
  "coding-vtuber": [codingVtuber1, codingVtuber2, codingVtuber3, codingVtuber4]
};

export const runnerOptions: { id: RunnerId; name: string; frame: string }[] = [
  { id: "coding-cat", name: "코딩 고양이", frame: codingCat1 },
  { id: "coding-orange-cat", name: "주황 고양이", frame: codingOrangeCat1 },
  { id: "coding-shrimp", name: "주황 새우", frame: codingShrimp1 },
  { id: "coding-fish", name: "노란 물고기", frame: codingFish1 },
  { id: "coding-vtuber", name: "핑크 버튜버", frame: codingVtuber1 }
];

const poolPartyVtuberFrames = [
  poolPartyVtuber1,
  poolPartyVtuber2,
  poolPartyVtuber3,
  poolPartyVtuber4
];

export function runnerFrames(runnerId: RunnerId, skinId: RunnerSkinId = "default") {
  if (runnerId === "coding-vtuber" && skinId === "pool-party") {
    return poolPartyVtuberFrames;
  }
  return runnerFramesById[runnerId];
}

export function runnerPreviewFrame(runnerId: RunnerId, skinId: RunnerSkinId = "default") {
  return runnerFrames(runnerId, skinId)[0];
}
