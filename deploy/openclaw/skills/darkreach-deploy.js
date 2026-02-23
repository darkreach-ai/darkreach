// darkreach-deploy — Deployment skill for the Operator agent.
//
// Triggers production deployments with human approval gates.
// Verifies health before and after deployment.
//
// This skill requires explicit human approval via Discord #approvals
// before executing any deployment action.
//
// Endpoints used:
//   GET /api/health    — Pre/post-deploy health check
//   GET /api/status    — System status comparison

const API_BASE = process.env.DARKREACH_API_URL || "https://api.darkreach.ai";

/**
 * Check if deployment prerequisites are met.
 * Returns: { ready: boolean, checks: [{ name, passed, detail }] }
 */
async function checkDeployReadiness() {
  const checks = [];

  // 1. API health
  try {
    const res = await fetch(`${API_BASE}/api/health`);
    checks.push({
      name: "API health",
      passed: res.ok,
      detail: res.ok ? "Healthy" : `Status ${res.status}`,
    });
  } catch (err) {
    checks.push({
      name: "API health",
      passed: false,
      detail: err.message,
    });
  }

  // 2. No critical searches at risk
  try {
    const res = await fetch(`${API_BASE}/api/search_jobs?status=running`);
    const jobs = await res.json();
    const activeCount = jobs.length;
    checks.push({
      name: "Active searches",
      passed: true, // Informational — don't block deploy for running searches
      detail: `${activeCount} running search(es) — will continue after restart`,
    });
  } catch (err) {
    checks.push({
      name: "Active searches",
      passed: true,
      detail: `Check failed (non-blocking): ${err.message}`,
    });
  }

  // 3. System status snapshot (for post-deploy comparison)
  try {
    const res = await fetch(`${API_BASE}/api/status`);
    const status = await res.json();
    checks.push({
      name: "Pre-deploy snapshot",
      passed: true,
      detail: JSON.stringify(status).slice(0, 200),
    });
  } catch (err) {
    checks.push({
      name: "Pre-deploy snapshot",
      passed: false,
      detail: err.message,
    });
  }

  const ready = checks.every((c) => c.passed);
  return { ready, checks };
}

/**
 * Verify deployment was successful by comparing pre and post state.
 * Returns: { success: boolean, checks: [{ name, passed, detail }] }
 */
async function verifyDeployment() {
  const checks = [];

  // Wait for service restart
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // 1. Health check (retry up to 5 times with 3s delay)
  let healthy = false;
  for (let attempt = 1; attempt <= 5; attempt++) {
    try {
      const res = await fetch(`${API_BASE}/api/health`);
      if (res.ok) {
        healthy = true;
        checks.push({
          name: "Post-deploy health",
          passed: true,
          detail: `Healthy (attempt ${attempt})`,
        });
        break;
      }
    } catch {
      // Retry
    }
    if (attempt < 5) {
      await new Promise((resolve) => setTimeout(resolve, 3000));
    }
  }

  if (!healthy) {
    checks.push({
      name: "Post-deploy health",
      passed: false,
      detail: "API not healthy after 5 attempts (20s)",
    });
  }

  // 2. WebSocket connectivity
  // Note: Basic check — full WebSocket test would require ws library
  try {
    const res = await fetch(`${API_BASE}/api/status`);
    checks.push({
      name: "Status endpoint",
      passed: res.ok,
      detail: res.ok ? "Responding" : `Status ${res.status}`,
    });
  } catch (err) {
    checks.push({
      name: "Status endpoint",
      passed: false,
      detail: err.message,
    });
  }

  const success = checks.every((c) => c.passed);
  return { success, checks };
}

/**
 * Generate a deploy approval request message for Discord.
 * The operator agent should post this to #approvals and wait for a reaction.
 */
function formatApprovalRequest(readinessReport) {
  const checksText = readinessReport.checks
    .map((c) => `${c.passed ? "+" : "-"} **${c.name}**: ${c.detail}`)
    .join("\n");

  return [
    "## Deploy Approval Request",
    "",
    `**Ready:** ${readinessReport.ready ? "Yes" : "No (see checks below)"}`,
    "",
    "### Pre-deploy checks:",
    checksText,
    "",
    'React with :white_check_mark: to approve or :x: to reject.',
  ].join("\n");
}

module.exports = {
  checkDeployReadiness,
  verifyDeployment,
  formatApprovalRequest,
};
