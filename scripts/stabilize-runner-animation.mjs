import path from "node:path";
import sharp from "sharp";

const runnerId = process.argv[2] ?? "coding-cat";
const frameDirectory =
  process.argv[3] ?? `src/assets/runners/master/${runnerId}`;
const frameSize = Number(process.argv[4] ?? 256);

const runnerRegions = {
  "coding-cat": {
    typing: { left: 94, top: 126, width: 72, height: 58 }
  },
  "coding-orange-cat": {
    typing: { left: 88, top: 136, width: 82, height: 62 }
  },
  "coding-white-cat": {
    typing: { left: 72, top: 132, width: 98, height: 68 }
  },
  "coding-fish": {
    typing: { left: 112, top: 144, width: 70, height: 58 }
  },
  "coding-vtuber": {
    typing: { left: 128, top: 136, width: 74, height: 66 },
    blink: { frame: 4, left: 108, top: 76, width: 70, height: 66 }
  }
};

if (!Number.isInteger(frameSize) || frameSize <= 0) {
  throw new Error("frame size must be a positive integer");
}

const regions = runnerRegions[runnerId];
if (!regions) {
  throw new Error(`Unknown runner: ${runnerId}`);
}

const scale = frameSize / 256;
const scaledRegion = (region) => ({
  left: Math.round(region.left * scale),
  top: Math.round(region.top * scale),
  width: Math.max(1, Math.round(region.width * scale)),
  height: Math.max(1, Math.round(region.height * scale))
});
const copyRegion = (source, destination, region) => {
  const target = scaledRegion(region);
  for (let y = target.top; y < target.top + target.height; y += 1) {
    for (let x = target.left; x < target.left + target.width; x += 1) {
      if (x < 0 || y < 0 || x >= frameSize || y >= frameSize) continue;
      const offset = (y * frameSize + x) * 4;
      source.copy(destination, offset, offset, offset + 4);
    }
  }
};

const basePath = path.join(frameDirectory, "01.png");
const base = await sharp(basePath).ensureAlpha().raw().toBuffer();

for (let frame = 2; frame <= 4; frame += 1) {
  const framePath = path.join(
    frameDirectory,
    `${String(frame).padStart(2, "0")}.png`
  );
  const current = await sharp(framePath).ensureAlpha().raw().toBuffer();
  const stabilized = Buffer.from(base);

  copyRegion(current, stabilized, regions.typing);
  if (regions.blink?.frame === frame) {
    copyRegion(current, stabilized, regions.blink);
  }

  await sharp(stabilized, {
    raw: { width: frameSize, height: frameSize, channels: 4 }
  })
    .png({ compressionLevel: 9 })
    .toFile(framePath);
}

console.log(
  `Locked laptop and body; preserved intentional motion for ${runnerId}`
);
