const targets = [
  ["windows-x86_64", "x64-setup.exe"],
  ["darwin-aarch64", "aarch64.app.tar.gz"],
  ["darwin-x86_64", "x64.app.tar.gz"]
];

// Build from the complete release asset list, never another job's latest.json.
export async function buildReleaseManifest(release, readSignature) {
  const version = release.tag_name?.match(/^v(\d+\.\d+\.\d+)$/)?.[1];
  if (!version) throw new Error("Expected a stable vX.Y.Z release tag");
  const platforms = {};
  for (const [platform, suffix] of targets) {
    const name = `RunDev_${version}_${suffix}`;
    const asset = release.assets.find(item => item.name === name);
    const sig = release.assets.find(item => item.name === `${name}.sig`);
    if (!asset?.url || !asset.size || !sig?.url) throw new Error(`Missing signed asset: ${name}`);
    const signature = (await readSignature(sig)).trim();
    if (signature.length < 50 || !/^[A-Za-z0-9+/=]+$/.test(signature)) {
      throw new Error(`Invalid signature: ${sig.name}`);
    }
    platforms[platform] = { signature, url: asset.url };
  }
  return { version, notes: release.body ?? "", pub_date: release.published_at, platforms };
}
