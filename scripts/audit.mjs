import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { checkAudit, exception } from "./audit-policy.mjs";

const lock = JSON.parse(await readFile(new URL("../package-lock.json", import.meta.url), "utf8"));
function audit(production) {
  const args = ["audit", "--json", ...(production ? ["--omit=dev"] : [])];
  const result = process.platform === "win32"
    ? spawnSync("cmd.exe", ["/d", "/s", "/c", `npm.cmd ${args.join(" ")}`], { encoding: "utf8", windowsHide: true })
    : spawnSync("npm", args, { encoding: "utf8" });
  if (result.error || (result.status !== 0 && result.status !== 1)) throw new Error("npm audit could not complete");
  const report = JSON.parse(result.stdout);
  const allowed = checkAudit(report, lock, { allowDevException: !production });
  if (production) console.log("Production dependency audit passed: 0 vulnerabilities");
  else if (allowed.length) console.warn(`Temporary dev-only exception: ${exception.advisory}; expires ${exception.expires}; affected: ${allowed.join(", ")}`);
  else console.log("Full dependency audit passed: 0 vulnerabilities");
}
audit(true);
audit(false);
