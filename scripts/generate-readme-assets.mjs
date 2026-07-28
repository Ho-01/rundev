import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { chromium } from "@playwright/test";
import gifenc from "gifenc";
import sharp from "sharp";

const { GIFEncoder, applyPalette, quantize } = gifenc;

const root = process.cwd();
const distDir = path.join(root, "dist");
const outputDir = path.join(root, "docs", "assets", "readme");
const runners = [
  { id: "coding-cat" },
  { id: "coding-orange-cat" },
  { id: "coding-shrimp" },
  { id: "coding-fish" },
  { id: "coding-vtuber" }
];

await fs.mkdir(outputDir, { recursive: true });

function contentType(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".css")) return "text/css; charset=utf-8";
  if (filePath.endsWith(".svg")) return "image/svg+xml";
  if (filePath.endsWith(".png")) return "image/png";
  return "application/octet-stream";
}

const server = http.createServer(async (request, response) => {
  try {
    const pathname = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
    const relative = pathname === "/" ? "index.html" : pathname.slice(1);
    const requested = path.resolve(distDir, relative);
    const safePath = requested.startsWith(path.resolve(distDir))
      ? requested
      : path.join(distDir, "index.html");
    let filePath = safePath;
    try {
      const stat = await fs.stat(filePath);
      if (stat.isDirectory()) filePath = path.join(filePath, "index.html");
    } catch {
      filePath = path.join(distDir, "index.html");
    }
    response.writeHead(200, { "Content-Type": contentType(filePath) });
    response.end(await fs.readFile(filePath));
  } catch (error) {
    response.writeHead(500);
    response.end(String(error));
  }
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
if (!address || typeof address === "string") throw new Error("Preview server did not start");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({
    viewport: { width: 320, height: 480 },
    deviceScaleFactor: 2,
    timezoneId: "Asia/Seoul"
  });
  for (const scenario of [
    { name: "dashboard-disconnected", query: "?preview=disconnected&freezeRunner=1" },
    {
      name: "dashboard-connected",
      query: "?preview=connected&runner=coding-fish&freezeRunner=1"
    },
    { name: "dashboard-active", query: "?preview=active&runner=coding-fish&freezeRunner=1" },
    {
      name: "runner-picker",
      query: "?preview=disconnected&runner=coding-fish&freezeRunner=1",
      action: async () => page.getByRole("button", { name: "개발자 변경" }).click()
    }
  ]) {
    await page.goto(`http://127.0.0.1:${address.port}/${scenario.query}`, {
      waitUntil: "networkidle"
    });
    await scenario.action?.();
    await page.screenshot({
      path: path.join(outputDir, `${scenario.name}.png`),
      animations: "disabled"
    });
  }
} finally {
  await browser.close();
  server.close();
}

for (const runner of runners) {
  const frames = [];
  for (let index = 1; index <= 4; index += 1) {
    const fileName = `${String(index).padStart(2, "0")}.png`;
    const framePath = path.join(
      root,
      "src",
      "assets",
      "runners",
      "ui",
      runner.id,
      fileName
    );
    const { data } = await sharp(framePath)
      .resize(128, 128, { kernel: sharp.kernel.lanczos3 })
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });
    frames.push(new Uint8Array(data));
  }

  const combined = new Uint8Array(frames.reduce((size, frame) => size + frame.length, 0));
  let offset = 0;
  for (const frame of frames) {
    combined.set(frame, offset);
    offset += frame.length;
  }
  const palette = quantize(combined, 256, {
    format: "rgba4444",
    oneBitAlpha: 96
  });
  const transparentIndex = Math.max(
    0,
    palette.findIndex((color) => color[3] === 0)
  );
  const gif = GIFEncoder();
  frames.forEach((frame, index) => {
    gif.writeFrame(applyPalette(frame, palette, "rgba4444"), 128, 128, {
      palette: index === 0 ? palette : undefined,
      delay: 170,
      repeat: 0,
      dispose: 2,
      transparent: true,
      transparentIndex
    });
  });
  gif.finish();
  await fs.writeFile(path.join(outputDir, `${runner.id}.gif`), gif.bytes());
}

console.log(`Generated README assets in ${path.relative(root, outputDir)}`);
