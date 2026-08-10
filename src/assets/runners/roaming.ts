import type { RunnerId } from "../../types/activity";
import codingCatRoam from "./master/coding-cat/roam.png";
import codingCatRoam02 from "./master/coding-cat/roam-02.png";
import codingCatRoam03 from "./master/coding-cat/roam-03.png";
import codingCatRoam04 from "./master/coding-cat/roam-04.png";
import codingFishRoam from "./master/coding-fish/roam.png";
import codingFishRoam02 from "./master/coding-fish/roam-02.png";
import codingFishRoam03 from "./master/coding-fish/roam-03.png";
import codingFishRoam04 from "./master/coding-fish/roam-04.png";
import codingOrangeCatRoam from "./master/coding-orange-cat/roam.png";
import codingOrangeCatRoam02 from "./master/coding-orange-cat/roam-02.png";
import codingOrangeCatRoam03 from "./master/coding-orange-cat/roam-03.png";
import codingOrangeCatRoam04 from "./master/coding-orange-cat/roam-04.png";
import codingShrimpRoam from "./master/coding-shrimp/roam.png";
import codingShrimpRoam02 from "./master/coding-shrimp/roam-02.png";
import codingShrimpRoam03 from "./master/coding-shrimp/roam-03.png";
import codingShrimpRoam04 from "./master/coding-shrimp/roam-04.png";
import codingVtuberRoam from "./master/coding-vtuber/roam.png";
import codingVtuberRoam02 from "./master/coding-vtuber/roam-02.png";
import codingVtuberRoam03 from "./master/coding-vtuber/roam-03.png";
import codingVtuberRoam04 from "./master/coding-vtuber/roam-04.png";

export const roamingRunnerFramesById: Record<RunnerId, string[]> = {
  "coding-cat": [codingCatRoam, codingCatRoam02, codingCatRoam03, codingCatRoam04],
  "coding-fish": [codingFishRoam, codingFishRoam02, codingFishRoam03, codingFishRoam04],
  "coding-orange-cat": [
    codingOrangeCatRoam,
    codingOrangeCatRoam02,
    codingOrangeCatRoam03,
    codingOrangeCatRoam04
  ],
  "coding-shrimp": [codingShrimpRoam, codingShrimpRoam02, codingShrimpRoam03, codingShrimpRoam04],
  "coding-vtuber": [codingVtuberRoam, codingVtuberRoam02, codingVtuberRoam03, codingVtuberRoam04]
};
