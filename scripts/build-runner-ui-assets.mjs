import fs from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";

const root = process.cwd();
const masterRoot = path.join(root, "src", "assets", "runners", "master");
const uiRoot = path.join(root, "src", "assets", "runners", "ui");
const runnerIds = [
  "coding-cat",
  "coding-orange-cat",
  "coding-shrimp",
  "coding-fish",
  "coding-vtuber",
  "coding-chubby-cat"
];
const uiScale = {
  "coding-cat": 1.15
};

for (const runnerId of runnerIds) {
  const outputDirectory = path.join(uiRoot, runnerId);
  await fs.mkdir(outputDirectory, { recursive: true });
  for (let frame = 1; frame <= 4; frame += 1) {
    const fileName = `${String(frame).padStart(2, "0")}.png`;
    const scale = uiScale[runnerId] ?? 1;
    const renderedSize = Math.round(128 * scale);
    let pipeline = sharp(path.join(masterRoot, runnerId, fileName)).resize(
      renderedSize,
      renderedSize,
      {
        fit: "fill",
        kernel: sharp.kernel.lanczos3
      }
    );
    if (renderedSize > 128) {
      const inset = Math.floor((renderedSize - 128) / 2);
      pipeline = pipeline.extract({
        left: inset,
        top: inset,
        width: 128,
        height: 128
      });
    }
    await pipeline
      .png({ compressionLevel: 9 })
      .toFile(path.join(outputDirectory, fileName));
  }
}

console.log(`Generated 128×128 developer frames in ${path.relative(root, uiRoot)}`);
