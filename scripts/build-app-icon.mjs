import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const input = path.join(root, "src-tauri", "app-icon-reference.jpg");
const output = path.join(root, "src-tauri", "app-icon-source.png");
const size = 1024;

await sharp(input)
  .resize({ width: size, height: size, fit: "cover" })
  .png()
  .toFile(output);

console.log(`Generated ${path.relative(root, output)}`);
