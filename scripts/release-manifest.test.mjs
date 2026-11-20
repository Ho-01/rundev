import test from "node:test";
import assert from "node:assert/strict";
import { buildReleaseManifest } from "./release-manifest.mjs";

const suffixes = ["x64-setup.exe", "aarch64.app.tar.gz", "x64.app.tar.gz"];
const assets = suffixes.flatMap((suffix, i) => [
  { name: `RunDev_0.9.1_${suffix}`, url: `https://api.github.com/repos/example/app/releases/assets/${i}`, size: 100 },
  { name: `RunDev_0.9.1_${suffix}.sig`, url: `https://api.github.com/repos/example/app/releases/assets/${i}sig` }
]);
const release = { tag_name: "v0.9.1", body: "Changes", published_at: "2026-11-20T00:00:00Z", assets };
const signature = "A".repeat(100);

test("reconstructs all three platforms regardless of prior partial manifests or asset order", async () => {
  const result = await buildReleaseManifest({ ...release, assets: [
    { name: "latest.json", url: "https://example.com/partial.json" },
    ...assets.toReversed(),
    { name: "RunDev_0.9.0_x64-setup.exe", url: "https://example.com/old.exe", size: 100 }
  ] }, async () => signature + "\n");
  assert.equal(result.version, "0.9.1");
  assert.deepEqual(Object.keys(result.platforms), ["windows-x86_64", "darwin-aarch64", "darwin-x86_64"]);
  assert.equal(result.platforms["darwin-aarch64"].url, assets[2].url);
  assert.ok(Object.values(result.platforms).every(item => item.signature === signature));
});

test("refuses to publish an incomplete or unsigned release", async () => {
  for (const missing of assets) {
    await assert.rejects(buildReleaseManifest({ ...release, assets: assets.filter(a => a !== missing) }, async () => signature), /Missing signed asset/);
  }
  await assert.rejects(buildReleaseManifest(release, async () => ""), /Invalid signature/);
  await assert.rejects(buildReleaseManifest({ ...release, tag_name: "main" }, async () => signature), /release tag/);
});
