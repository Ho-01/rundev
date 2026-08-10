import path from "node:path";
import { fileURLToPath } from "node:url";
import { copyFile } from "node:fs/promises";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const input = path.join(root, "src-tauri", "app-icon-macos.icns");
const output = path.join(root, "src-tauri", "icons", "icon.icns");

await copyFile(input, output);

console.log(`Restored ${path.relative(root, output)} from the macOS source icon`);
