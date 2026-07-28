import { readFile, writeFile } from "node:fs/promises";
import process from "node:process";

const paths = {
  package: new URL("../package.json", import.meta.url),
  lock: new URL("../package-lock.json", import.meta.url),
  cargo: new URL("../src-tauri/Cargo.toml", import.meta.url),
  tauri: new URL("../src-tauri/tauri.conf.json", import.meta.url)
};

const semverPattern =
  /^\d+\.\d+\.\d+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

async function readVersions() {
  const [packageText, lockText, cargoText, tauriText] = await Promise.all([
    readFile(paths.package, "utf8"),
    readFile(paths.lock, "utf8"),
    readFile(paths.cargo, "utf8"),
    readFile(paths.tauri, "utf8")
  ]);
  const packageJson = JSON.parse(packageText);
  const packageLock = JSON.parse(lockText);
  const tauriConfig = JSON.parse(tauriText);
  const cargoVersion = cargoText.match(
    /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m
  )?.[1];

  if (!cargoVersion) {
    throw new Error("src-tauri/Cargo.toml의 [package] version을 찾지 못했습니다.");
  }

  return {
    packageJson: packageJson.version,
    packageLock: packageLock.version,
    packageLockRoot: packageLock.packages?.[""]?.version,
    cargo: cargoVersion,
    tauri: tauriConfig.version
  };
}

function assertVersion(version) {
  if (!semverPattern.test(version)) {
    throw new Error(`올바른 Semantic Version이 아닙니다: ${version}`);
  }
}

async function check() {
  const versions = await readVersions();
  const entries = Object.entries(versions);
  entries.forEach(([, version]) => assertVersion(version));
  const expected = entries[0][1];
  const mismatches = entries.filter(([, version]) => version !== expected);

  if (mismatches.length > 0) {
    const details = entries.map(([name, version]) => `${name}=${version}`).join(", ");
    throw new Error(`앱 버전이 일치하지 않습니다: ${details}`);
  }
  console.log(`RunDev version ${expected}`);
  return expected;
}

async function setVersion(version) {
  assertVersion(version);
  const [packageText, lockText, cargoText, tauriText] = await Promise.all([
    readFile(paths.package, "utf8"),
    readFile(paths.lock, "utf8"),
    readFile(paths.cargo, "utf8"),
    readFile(paths.tauri, "utf8")
  ]);
  const packageJson = JSON.parse(packageText);
  const packageLock = JSON.parse(lockText);
  const tauriConfig = JSON.parse(tauriText);

  packageJson.version = version;
  packageLock.version = version;
  packageLock.packages[""].version = version;
  tauriConfig.version = version;
  const nextCargo = cargoText.replace(
    /^(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
    `$1"${version}"`
  );

  await Promise.all([
    writeFile(paths.package, `${JSON.stringify(packageJson, null, 2)}\n`),
    writeFile(paths.lock, `${JSON.stringify(packageLock, null, 2)}\n`),
    writeFile(paths.cargo, nextCargo),
    writeFile(paths.tauri, `${JSON.stringify(tauriConfig, null, 2)}\n`)
  ]);
  await check();
}

async function checkTag() {
  const version = await check();
  const tag = process.argv[3] ?? process.env.GITHUB_REF_NAME;
  if (!tag) {
    throw new Error("검증할 태그를 인자 또는 GITHUB_REF_NAME으로 전달해 주세요.");
  }
  if (tag !== `v${version}`) {
    throw new Error(`태그 ${tag}와 앱 버전 v${version}이 일치하지 않습니다.`);
  }
  console.log(`Release tag ${tag}`);
}

const command = process.argv[2];
try {
  if (command === "set") {
    const version = process.argv[3];
    if (!version) throw new Error("설정할 버전을 입력해 주세요.");
    await setVersion(version);
  } else if (command === "check") {
    await check();
  } else if (command === "tag") {
    await checkTag();
  } else {
    throw new Error("사용법: node scripts/version.mjs <set VERSION|check|tag [TAG]>");
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
