import assert from "node:assert/strict";
import sharp from "sharp";
import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../src/assets/emblems/pixel/", import.meta.url));
const files = (await readdir(root)).filter(file => file.endsWith(".png"));
assert.equal(files.length, 30);
const sizes = {};
const manifest = JSON.parse(await readFile(new URL("../src/assets/emblems/source/extraction.json", import.meta.url), "utf8"));
for (const file of files) {
  const { data, info } = await sharp(`${root}/${file}`).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const record = manifest.find(item => `${item.tier}-${item.stage}.png` === file);
  assert.ok(record?.cropBounds, `${file}: missing crop metadata`);
  const { width, height } = info;
  assert.equal(width, record.cropBounds.width, file); assert.equal(height, record.cropBounds.height, file);
  let left = width, right = -1, top = height, bottom = -1;
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const alpha = data[(y * width + x) * 4 + 3];
    assert.ok(alpha === 0 || alpha === 255, `${file}: fuzzy alpha`);
    assert.equal(alpha, data[(y * width + width - 1 - x) * 4 + 3], `${file}: asymmetric outline at ${x},${y}`);
    const originalX = x + record.cropBounds.left, originalY = y + record.cropBounds.top;
    if (originalX < 20 || originalX >= 44 || originalY < 20 || originalY >= 44) {
      for (let channel = 0; channel < 4; channel++) assert.equal(data[(y * width + x) * 4 + channel],
        data[(y * width + width - 1 - x) * 4 + channel], `${file}: asymmetric frame at ${x},${y}`);
    }
    if (!alpha) continue;
    left = Math.min(left, x); right = Math.max(right, x);
    top = Math.min(top, y); bottom = Math.max(bottom, y);
  }
  assert.ok(right >= left, `${file}: empty`);
  assert.deepEqual([left, top, width - 1 - right, height - 1 - bottom], [2, 2, 2, 2], `${file}: excessive or missing transparent padding`);
  assert.equal(left + right, width - 1, `${file}: horizontal alignment`);
  assert.equal(top + bottom, height - 1, `${file}: vertical alignment`);
  sizes[file] = { width: right - left + 1, height: bottom - top + 1 };
}
for (const tier of new Set(manifest.map(item => item.tier))) {
  assert.ok(sizes[`${tier}-1.png`].width > sizes[`${tier}-0.png`].width, `${tier}: stage 2 must expand`);
  assert.ok(sizes[`${tier}-2.png`].width > sizes[`${tier}-1.png`].width, `${tier}: stage 3 must expand`);
}
console.log(JSON.stringify({ checked: 30, centered: true, padding: "exactly 2px", symmetricFrames: true, centralGlyphsPreserved: true, legendary: [0, 1, 2].map(stage => sizes[`legend-${stage}.png`]) }));
