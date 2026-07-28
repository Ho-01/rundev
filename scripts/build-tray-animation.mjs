import fs from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";

const input = process.argv[2];
const outputDir = process.argv[3] ?? "src-tauri/icons/tray/coding";

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
  const magentaDominance = Math.min(red, blue) - green;
  const alpha = Math.round(
    Math.max(0, Math.min(255, ((140 - magentaDominance) / 60) * 255))
  );
  data[index + 3] = alpha;

  if (alpha < 245) {
    data[index] = Math.min(red, green + 18);
    data[index + 2] = Math.min(blue, green + 18);
  }
}

const frameWidth = Math.floor(info.width / 4);
for (let frame = 0; frame < 4; frame += 1) {
  const frameBuffer = await sharp(data, {
    raw: { width: info.width, height: info.height, channels: 4 }
  })
    .extract({
      left: frame * frameWidth,
      top: 150,
      width: frame === 3 ? info.width - frame * frameWidth : frameWidth,
      height: 480
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
