import assert from "node:assert/strict";
import { test } from "node:test";
import { checkAudit, exception } from "./audit-policy.mjs";

function fixture() {
  return {
    report: { auditReportVersion: 2, vulnerabilities: {
      picomatch: { nodes: ["node_modules/picomatch"], via: [{ name: "picomatch", url: exception.advisory }] },
      vite: { nodes: ["node_modules/vite"], via: ["picomatch"] }
    } },
    lock: { packages: { "node_modules/picomatch": { dev: true }, "node_modules/vite": { dev: true } } }
  };
}
const options = { allowDevException: true, now: new Date("2026-11-18T00:00:00Z") };
test("allows only the approved development advisory and its dependency chain", () => {
  const { report, lock } = fixture();
  assert.deepEqual(checkAudit(report, lock, options), ["picomatch", "vite"]);
  assert.throws(() => checkAudit(report, lock), /blocked/);
});
test("blocks the exception at expiry and any production dependency", () => {
  const { report, lock } = fixture();
  assert.throws(() => checkAudit(report, lock, { ...options, now: new Date(exception.expires) }), /blocked/);
  lock.packages["node_modules/picomatch"].dev = false;
  assert.throws(() => checkAudit(report, lock, options), /blocked/);
});
test("blocks a new advisory even when it affects the same package", () => {
  const { report, lock } = fixture();
  report.vulnerabilities.picomatch.via.push({ name: "picomatch", url: "https://github.com/advisories/new" });
  assert.throws(() => checkAudit(report, lock, options), /blocked/);
});
test("fails closed for incomplete reports, unknown nodes, and cyclic dependency chains", () => {
  const { report, lock } = fixture();
  assert.throws(() => checkAudit({ error: {} }, lock, options), /Invalid/);
  delete lock.packages["node_modules/picomatch"];
  assert.throws(() => checkAudit(report, lock, options), /blocked/);
  const cyclic = fixture();
  cyclic.report.vulnerabilities.picomatch.via = ["vite"];
  assert.throws(() => checkAudit(cyclic.report, cyclic.lock, options), /blocked/);
});
