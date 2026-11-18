import sharp from "sharp";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const root = new URL("../src/assets/emblems/", import.meta.url);
const source = fileURLToPath(new URL("source/approved-tier-board.png", root));
const destination = fileURLToPath(new URL("pixel/", root));
const tiers = ["sprout", "rookie", "maker", "debugger", "hacker", "systems", "architect", "lead", "master", "legend"];
const { data, info } = await sharp(source).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
const { width, height } = info;
// Artwork-only cells exclude labels and row separators. Remove dark background
// connected to cell edges, retaining dark jewel interiors and detached crests.
const cells = [[344, 512], [600, 785], [878, 1095]];
const columns = [];
for (const [cellLeft, cellRight] of cells) {
  const cellTop = 110, cellBottom = 1220;
  const cw = cellRight - cellLeft, ch = cellBottom - cellTop;
  const background = new Uint8Array(cw * ch), queue = [];
  const enqueue = (x, y) => {
    if (x < 0 || y < 0 || x >= cw || y >= ch) return;
    const local = y * cw + x, offset = ((cellTop + y) * width + cellLeft + x) * 4;
    if (background[local] || Math.max(data[offset], data[offset + 1], data[offset + 2]) >= 85) return;
    background[local] = 1; queue.push(local);
  };
  for (let x = 0; x < cw; x++) { enqueue(x, 0); enqueue(x, ch - 1); }
  for (let y = 0; y < ch; y++) { enqueue(0, y); enqueue(cw - 1, y); }
  for (let i = 0; i < queue.length; i++) {
    const x = queue[i] % cw, y = Math.floor(queue[i] / cw);
    enqueue(x - 1, y); enqueue(x + 1, y); enqueue(x, y - 1); enqueue(x, y + 1);
  }
  const seen = new Uint8Array(cw * ch), groups = [];
  for (let start = 0; start < seen.length; start++) {
    if (seen[start] || background[start]) continue;
    const pending = [start], component = { pixels: [], left: width, right: 0, top: height, bottom: 0 };
    seen[start] = 1;
    for (let i = 0; i < pending.length; i++) {
      const x = pending[i] % cw, y = Math.floor(pending[i] / cw), px = cellLeft + x, py = cellTop + y;
      component.pixels.push(py * width + px);
      component.left = Math.min(component.left, px); component.right = Math.max(component.right, px);
      component.top = Math.min(component.top, py); component.bottom = Math.max(component.bottom, py);
      for (const [nx, ny] of [[x-1,y],[x+1,y],[x,y-1],[x,y+1]]) {
        const next = ny*cw+nx;
        if (nx<0 || nx>=cw || ny<0 || ny>=ch || seen[next] || background[next]) continue;
        seen[next]=1; pending.push(next);
      }
    }
    if (component.pixels.length >= 8) groups.push(component);
  }
  const bodies = Array.from({length:10},()=>({pixels:[],left:width,right:0,top:height,bottom:0}));
  for (const ornament of groups) {
    const row = Math.max(0,Math.min(9,Math.round(((ornament.top+ornament.bottom)/2-170)/110)));
    const body = bodies[row];
    body.pixels.push(...ornament.pixels);
    body.left=Math.min(body.left,ornament.left); body.right=Math.max(body.right,ornament.right);
    body.top=Math.min(body.top,ornament.top); body.bottom=Math.max(body.bottom,ornament.bottom);
  }
  if (bodies.some(body=>body.pixels.length<3000)) throw new Error('Missing tier body');
  columns.push(bodies);
}
const ordered = Array.from({length:10},(_,row)=>columns.map(column=>column[row])).flat();
// Shared scale preserves growing armor instead of filling every badge to one size.
const scale = 56 / Math.max(...ordered.flatMap(c => [c.right - c.left + 1, c.bottom - c.top + 1]));
await mkdir(destination, { recursive: true });
const manifest = [];
for (const [index, component] of ordered.entries()) {
  const tier = tiers[Math.floor(index / 3)], stage = index % 3;
  const w = component.right - component.left + 1, h = component.bottom - component.top + 1;
  if (component.left === 0 || component.top === 0 || component.right === width - 1 || component.bottom === height - 1) throw new Error(`Source touches atlas edge: ${tier}-${stage}`);
  const isolated = Buffer.alloc(w * h * 4);
  for (const pixel of component.pixels) {
    const offset = ((Math.floor(pixel / width) - component.top) * w + pixel % width - component.left) * 4;
    data.copy(isolated, offset, pixel * 4, pixel * 4 + 3); isolated[offset + 3] = 255;
  }
  // Even dimensions place every badge on exactly the same half-pixel center.
  const targetWidth = Math.max(2, Math.round(w * scale / 2) * 2);
  const targetHeight = Math.max(2, Math.round(h * scale / 2) * 2);
  const resized = await sharp(isolated, { raw: { width: w, height: h, channels: 4 } }).resize(targetWidth, targetHeight, { kernel: "nearest" }).raw().toBuffer();
  const canvas = Buffer.alloc(64 * 64 * 4), left = (64 - targetWidth) / 2, top = (64 - targetHeight) / 2;
  for (let y = 0; y < targetHeight; y++) resized.copy(canvas, ((top + y) * 64 + left) * 4, y * targetWidth * 4, (y + 1) * targetWidth * 4);
  // Keep the central 24px glyph area. Normalize the surrounding frame geometry
  // and pixels, so wings and ornaments are exactly symmetric at the native grid.
  for (let y = 0; y < 64; y++) for (let x = 0; x < 32; x++) {
    const a = (y * 64 + x) * 4, b = (y * 64 + 63 - x) * 4;
    if (canvas[a + 3] && !canvas[b + 3]) canvas.copy(canvas, b, a, a + 4);
    else if (!canvas[a + 3] && canvas[b + 3]) canvas.copy(canvas, a, b, b + 4);
    if (x < 20 || y < 20 || y >= 44) canvas.copy(canvas, b, a, a + 4);
  }
  let inkLeft = 64, inkRight = 0, inkTop = 64, inkBottom = 0;
  for (let y = 0; y < 64; y++) for (let x = 0; x < 64; x++) {
    if (!canvas[(y * 64 + x) * 4 + 3]) continue;
    inkLeft = Math.min(inkLeft, x); inkRight = Math.max(inkRight, x);
    inkTop = Math.min(inkTop, y); inkBottom = Math.max(inkBottom, y);
  }
  // The 64px normalization canvas is not the display asset. Trim its unused
  // space so the actual artwork, rather than transparent padding, fills the UI.
  const cropBounds = { left: inkLeft - 2, top: inkTop - 2, width: inkRight - inkLeft + 5, height: inkBottom - inkTop + 5 };
  await sharp(canvas, { raw: { width: 64, height: 64, channels: 4 } }).extract(cropBounds)
    .png({ palette: true, colours: 64, dither: 0 }).toFile(`${destination}/${tier}-${stage}.png`);
  manifest.push({ tier, stage, sourceBounds: { left: component.left, top: component.top, width: w, height: h }, cropBounds });
}
await writeFile(new URL("source/extraction.json", root), `${JSON.stringify(manifest, null, 2)}\n`);
console.log("Recovered and centered 30 complete badges with symmetric alpha outlines");







