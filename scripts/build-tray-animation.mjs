import fs from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";

const input = process.argv[2];
const outputDir = process.argv[3] ?? "src-tauri/icons/tray/coding";
const greenKey = process.argv.includes("--green-key");
const magentaKey = process.argv.includes("--magenta-key");
const flipHorizontal = process.argv.includes("--flip-horizontal");
const normalizeReference = process.argv.includes("--normalize-reference");

if (!input) {
  throw new Error("Usage: node scripts/build-tray-animation.mjs <sprite.png> [output-dir]");
}

await fs.mkdir(outputDir, { recursive: true });

const source = sharp(input);
const metadata = await source.metadata();
const { data, info } = await source.ensureAlpha().raw().toBuffer({ resolveWithObject: true });

for (let index = 0; index < data.length; index += 4) {
  const red = data[index];
  const green = data[index + 1];
  const blue = data[index + 2];
  const keyDistance = magentaKey
    ? Math.hypot(red - 255, green, blue - 255)
    : Math.hypot(red, green - 255, blue);
  const alpha = greenKey || magentaKey
    ? Math.round(
        Math.max(
          0,
          Math.min(255, ((keyDistance - 42) / 54) * 255)
        )
      )
    : Math.round(
        Math.max(
          0,
          Math.min(255, ((140 - (Math.min(red, blue) - green)) / 60) * 255)
        )
      );
  data[index + 3] = alpha;

  if (greenKey && alpha > 0) {
    data[index + 1] = Math.min(green, Math.max(red, blue) + 12);
  } else if (magentaKey && alpha > 0) {
    const magentaExcess = Math.max(0, Math.min(red - green, blue - green));
    data[index] = Math.max(0, red - Math.round(magentaExcess * 0.85));
    data[index + 2] = Math.max(0, blue - Math.round(magentaExcess * 0.85));
  } else if (alpha < 245) {
    data[index] = Math.min(red, green + 18);
    data[index + 2] = Math.min(blue, green + 18);
  }
}

const frameWidth = Math.floor(info.width / 4);
let crop = { left: 0, top: 150, width: frameWidth, height: 480 };

if (greenKey || magentaKey) {
  let minX = frameWidth;
  let minY = info.height;
  let maxX = 0;
  let maxY = 0;
  for (let frame = 0; frame < 4; frame += 1) {
    for (let y = 0; y < info.height; y += 1) {
      for (let x = 0; x < frameWidth; x += 1) {
        const alpha = data[(y * info.width + frame * frameWidth + x) * 4 + 3];
        if (alpha > 20) {
          minX = Math.min(minX, x);
          minY = Math.min(minY, y);
          maxX = Math.max(maxX, x);
          maxY = Math.max(maxY, y);
        }
      }
    }
  }
  const padding = Math.round(Math.max(info.width, info.height) * 0.008);
  const left = Math.max(0, minX - padding);
  const top = Math.max(0, minY - padding);
  crop = {
    left,
    top,
    width: Math.min(frameWidth - left, maxX - minX + 1 + padding * 2),
    height: Math.min(info.height - top, maxY - minY + 1 + padding * 2)
  };
}

for (let frame = 0; frame < 4; frame += 1) {
  let framePipeline = sharp(data, {
    raw: { width: info.width, height: info.height, channels: 4 }
  })
    .extract({
      left: frame * frameWidth + crop.left,
      top: crop.top,
      width: crop.width,
      height: crop.height
    })
    .resize({
      width: 30,
      height: 28,
      fit: "contain",
      background: { r: 0, g: 0, b: 0, alpha: 0 },
      kernel: sharp.kernel.nearest
    })
    .extend({
      top: 2,
      bottom: 2,
      left: 1,
      right: 1,
      background: { r: 0, g: 0, b: 0, alpha: 0 }
    });
  if (flipHorizontal) framePipeline = framePipeline.flop();
  const frameBuffer = await framePipeline.png().toBuffer();

  await fs.writeFile(
    path.join(outputDir, `${String(frame + 1).padStart(2, "0")}.png`),
    frameBuffer
  );
}

if (normalizeReference) {
  const frameData = [];
  let minX = 32;
  let minY = 32;
  let maxX = 0;
  let maxY = 0;
  for (let frame = 1; frame <= 4; frame += 1) {
    const framePath = path.join(outputDir, `${String(frame).padStart(2, "0")}.png`);
    const { data: current, info: currentInfo } = await sharp(framePath)
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });
    frameData.push({ framePath, current, currentInfo });
    for (let y = 0; y < currentInfo.height; y += 1) {
      for (let x = 0; x < currentInfo.width; x += 1) {
        if (current[(y * currentInfo.width + x) * 4 + 3] <= 20) continue;
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }
  const width = maxX - minX + 1;
  const height = maxY - minY + 1;
  for (const { framePath, current, currentInfo } of frameData) {
    await sharp(current, {
      raw: {
        width: currentInfo.width,
        height: currentInfo.height,
        channels: 4
      }
    })
      .extract({ left: minX, top: minY, width, height })
      .resize({
        width: 28,
        height: 22,
        fit: "contain",
        background: { r: 0, g: 0, b: 0, alpha: 0 },
        kernel: sharp.kernel.nearest
      })
      .extend({
        top: 5,
        bottom: 5,
        left: 3,
        right: 1,
        background: { r: 0, g: 0, b: 0, alpha: 0 }
      })
      .png()
      .toFile(framePath);
  }
}
