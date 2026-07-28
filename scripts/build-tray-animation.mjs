import fs from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";

const input = process.argv[2];
const outputDir = process.argv[3] ?? "src-tauri/icons/tray/coding";
const greenKey = process.argv.includes("--green-key");
const magentaKey = process.argv.includes("--magenta-key");
const lockLaptop = process.argv.includes("--lock-laptop");
const liftDarkLaptop = process.argv.includes("--lift-dark-laptop");

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
  const frameBuffer = await sharp(data, {
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
    })
    .png()
    .toBuffer();

  await fs.writeFile(
    path.join(outputDir, `${String(frame + 1).padStart(2, "0")}.png`),
    frameBuffer
  );
}

if ((greenKey || magentaKey) && lockLaptop) {
  const firstFramePath = path.join(outputDir, "01.png");
  const { data: fixed, info: fixedInfo } = await sharp(firstFramePath)
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });

  for (let frame = 2; frame <= 4; frame += 1) {
    const framePath = path.join(outputDir, `${String(frame).padStart(2, "0")}.png`);
    const { data: current } = await sharp(framePath)
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });

    for (let y = 0; y < fixedInfo.height; y += 1) {
      for (let x = 0; x < fixedInfo.width; x += 1) {
        const isLaptop = (x <= 15 && y >= 8) || (x <= 20 && y >= 20);
        if (!isLaptop) continue;
        const offset = (y * fixedInfo.width + x) * 4;
        fixed.copy(current, offset, offset, offset + 4);
      }
    }

    await sharp(current, {
      raw: {
        width: fixedInfo.width,
        height: fixedInfo.height,
        channels: 4
      }
    })
      .png()
      .toFile(framePath);
  }
}

if (liftDarkLaptop) {
  for (let frame = 1; frame <= 4; frame += 1) {
    const framePath = path.join(outputDir, `${String(frame).padStart(2, "0")}.png`);
    const { data: current, info: currentInfo } = await sharp(framePath)
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });
    for (let y = 8; y < currentInfo.height; y += 1) {
      for (let x = 0; x <= 20; x += 1) {
        const offset = (y * currentInfo.width + x) * 4;
        if (current[offset + 3] === 0) continue;
        const luminance =
          current[offset] * 0.2126 +
          current[offset + 1] * 0.7152 +
          current[offset + 2] * 0.0722;
        if (luminance >= 72) continue;
        const lift = Math.round(72 - luminance);
        current[offset] = Math.min(255, current[offset] + lift);
        current[offset + 1] = Math.min(255, current[offset + 1] + lift);
        current[offset + 2] = Math.min(255, current[offset + 2] + lift);
      }
    }
    await sharp(current, {
      raw: {
        width: currentInfo.width,
        height: currentInfo.height,
        channels: 4
      }
    })
      .png()
      .toFile(framePath);
  }
}
