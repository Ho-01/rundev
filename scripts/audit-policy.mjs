export const exception = {
  advisory: "https://github.com/advisories/GHSA-fc5q-39xc-vjpf",
  expires: "2026-12-18T00:00:00Z"
};

export function checkAudit(report, lock, { allowDevException = false, now = new Date() } = {}) {
  if (report?.error || report?.auditReportVersion !== 2 || !report.vulnerabilities || !lock?.packages) {
    throw new Error("Invalid npm audit report or lockfile");
  }
  const vulnerabilities = report.vulnerabilities;
  function allowed(name, path = new Set()) {
    const finding = vulnerabilities[name];
    if (!allowDevException || now >= new Date(exception.expires) || path.has(name) || !finding) return false;
    if (!finding.nodes?.length || !finding.nodes.every(node => lock.packages[node]?.dev === true)) return false;
    if (!finding.via?.length) return false;
    const next = new Set([...path, name]);
    return finding.via.every(via => typeof via === "string" ? allowed(via, next)
      : name === "picomatch" && via.name === "picomatch" && via.url === exception.advisory);
  }
  const blocked = Object.keys(vulnerabilities).filter(name => !allowed(name));
  if (blocked.length) throw new Error(`Security audit blocked: ${blocked.join(", ")}`);
  return Object.keys(vulnerabilities);
}
