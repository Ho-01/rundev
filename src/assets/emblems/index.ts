import type { LevelTierId } from "../../domain/level";
import extraction from "./source/extraction.json";

// Eager URL imports do not fetch or decode the sprites until an image uses one.
const sprites = import.meta.glob<string>("./pixel/*.png", { eager: true, query: "?url&no-inline", import: "default" });

const dimensions = new Map(extraction.map(({ tier, stage, cropBounds }) => [`${tier}-${stage}`, cropBounds]));

export function tierEmblemAsset(tier: LevelTierId, stage: number) {
  const key = `${tier}-${Math.max(0, Math.min(2, Math.floor(stage)))}`;
  const src = sprites[`./pixel/${key}.png`];
  const size = dimensions.get(key);
  if (!src || !size) throw new Error(`Missing tier emblem: ${key}`);
  return { src, width: size.width, height: size.height };
}
