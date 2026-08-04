import fs from "node:fs";

const tag = process.argv[2];
if (!tag) {
  throw new Error("Usage: node scripts/release-notes.mjs vX.Y.Z");
}

const version = tag.replace(/^v/, "");
const changelog = fs.readFileSync("CHANGELOG.md", "utf8");
const escapedVersion = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const heading = new RegExp(`^## ${escapedVersion}(?: - .+)?$`, "m").exec(changelog);

if (!heading) {
  throw new Error(`CHANGELOG.md에서 ${version} 릴리스 섹션을 찾을 수 없습니다.`);
}

const sectionStart = heading.index + heading[0].length;
const remainder = changelog.slice(sectionStart).replace(/^\r?\n/, "");
const nextHeading = /^## /m.exec(remainder);
const section = (nextHeading ? remainder.slice(0, nextHeading.index) : remainder)
  .trim()
  .replace(/^### Added$/gm, "### 추가")
  .replace(/^### Changed$/gm, "### 변경")
  .replace(/^### Fixed$/gm, "### 수정")
  .replace(/^### Removed$/gm, "### 제거")
  .replace(/^### Security$/gm, "### 보안");

if (!section) {
  throw new Error(`CHANGELOG.md의 ${version} 릴리스 섹션이 비어 있습니다.`);
}

process.stdout.write(`${section}\n`);
