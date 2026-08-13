import fs from "node:fs/promises";
import path from "node:path";
import sharp from "sharp";

const root = process.cwd();
const sourceRoot = path.join(root, "artifacts", "chubby-cat-source");
const masterRoot = path.join(root, "src", "assets", "runners", "master", "coding-chubby-cat");
const trayRoot = path.join(root, "src-tauri", "icons", "tray", "coding-chubby-cat");

const sheets = [
  { source: "idle-sheet.png", frames: 4, names: ["01.png", "02.png", "03.png", "04.png"] },
  { source: "roam-sheet.png", frames: 4, names: ["roam.png", "roam-02.png", "roam-03.png", "roam-04.png"] },
  { source: "grabbed-sheet.png", frames: 3, names: ["grabbed.png", "grabbed-2.png", "grabbed-3.png"] },
  { source: "feeding-sheet.png", frames: 4, names: ["feed-ready.png", "feed-bite.png", "feed-chew.png", "feed-swallow.png"] }
];

await fs.mkdir(masterRoot, { recursive: true });
await fs.mkdir(trayRoot, { recursive: true });

for (const sheet of sheets) {
  const sourcePath = path.join(sourceRoot, sheet.source);
  const metadata = await sharp(sourcePath).metadata();
  const cells = [];

  for (let index = 0; index < sheet.frames; index += 1) {
    const left = Math.floor((metadata.width * index) / sheet.frames);
    const right = index === sheet.frames - 1
      ? metadata.width
      : Math.floor((metadata.width * (index + 1)) / sheet.frames);
    const cropped = await sharp(sourcePath)
      .extract({ left, top: 0, width: right - left, height: metadata.height })
      .png()
      .toBuffer();
    const buffer = await sharp(cropped)
      .trim()
      .png()
      .toBuffer();
    const cellMetadata = await sharp(buffer).metadata();
    cells.push({ buffer, width: cellMetadata.width, height: cellMetadata.height });
  }

  const commonScale = Math.min(
    224 / Math.max(...cells.map((cell) => cell.width)),
    224 / Math.max(...cells.map((cell) => cell.height))
  );
  const normalizedHeight = Math.round(
    Math.max(...cells.map((cell) => cell.height)) * commonScale
  );
  const normalizedWidth = Math.round(
    Math.max(...cells.map((cell) => cell.width)) * commonScale
  );

  for (let index = 0; index < cells.length; index += 1) {
    const cell = cells[index];
    // The generated typing cycle pulls the torso inward in the middle frames.
    // Counter only that motion so the cat still wiggles without looking squeezed.
    const idleWidthScale = sheet.source === "idle-sheet.png"
      ? [1, 1.045, 1.09, 1][index]
      : 1;
    const width = Math.max(1, Math.min(248, Math.round(normalizedWidth * idleWidthScale)));
    // Generated sheets can accidentally squash an entire pose by a few pixels.
    // Keep the character's perceived size stable across the animation cycle.
    const height = Math.max(1, normalizedHeight);
    const subject = await sharp(cell.buffer).resize(width, height, {
      fit: "fill",
      kernel: sharp.kernel.lanczos3
    }).png().toBuffer();

    await sharp({
      create: { width: 256, height: 256, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } }
    })
      .composite([{ input: subject, left: Math.round((256 - width) / 2), top: 244 - height }])
      .png({ compressionLevel: 9 })
      .toFile(path.join(masterRoot, sheet.names[index]));
  }
}

for (let frame = 1; frame <= 4; frame += 1) {
  const fileName = `${String(frame).padStart(2, "0")}.png`;
  await sharp(path.join(masterRoot, fileName))
    .resize(30, 30, { fit: "contain", kernel: sharp.kernel.lanczos3 })
    .extend({
      top: 1,
      bottom: 1,
      left: 1,
      right: 1,
      background: { r: 0, g: 0, b: 0, alpha: 0 }
    })
    .png({ compressionLevel: 9 })
    .toFile(path.join(trayRoot, fileName));
}

console.log(`Built 하찮은 뚱냥이 master frames in ${path.relative(root, masterRoot)}`);
