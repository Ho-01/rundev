import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const input = path.join(root, "src", "assets", "runners", "master", "coding-cat", "01.png");
const output = path.join(root, "src-tauri", "app-icon-source.png");
const size = 1024;

const background = Buffer.from(`
  <svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <clipPath id="shape"><rect x="48" y="48" width="928" height="928" rx="220"/></clipPath>
      <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#242a29"/>
        <stop offset="1" stop-color="#101313"/>
      </linearGradient>
      <radialGradient id="glow" cx="50%" cy="42%" r="62%">
        <stop offset="0" stop-color="#55d88b" stop-opacity=".34"/>
        <stop offset="1" stop-color="#55d88b" stop-opacity="0"/>
      </radialGradient>
    </defs>
    <rect x="48" y="48" width="928" height="928" rx="220" fill="url(#bg)"/>
    <rect x="68" y="68" width="888" height="888" rx="202" fill="none" stroke="#70e59b" stroke-opacity=".25" stroke-width="10"/>
    <circle cx="512" cy="455" r="410" fill="url(#glow)" clip-path="url(#shape)"/>
  </svg>
`);

const character = await sharp(input)
  .trim({ background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .resize({ width: 800, height: 800, fit: "inside", kernel: sharp.kernel.nearest })
  .png()
  .toBuffer();

await sharp(background)
  .composite([{ input: character, gravity: "centre" }])
  .png()
  .toFile(output);

console.log(`Generated ${path.relative(root, output)}`);
