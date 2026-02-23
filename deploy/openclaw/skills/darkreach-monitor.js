// darkreach-monitor — Fleet monitoring skill for the Operator agent.
//
// Checks fleet health, node status, search progress, and system resources.
// Posts alerts to Discord when thresholds are exceeded.
//
// Endpoints used:
//   GET /api/fleet      — Node list with heartbeat timestamps
//   GET /api/health     — API health check
//   GET /api/status     — System status summary
//   GET /api/search_jobs — Active search progress
//   GET /metrics        — Prometheus metrics (text format)

const API_BASE = process.env.DARKREACH_API_URL || "https://api.darkreach.ai";

// Alert thresholds
const STALE_WORKER_SECONDS = 300; // 5 minutes
const ERROR_RATE_THRESHOLD = 0.05; // 5%
const DISK_USAGE_THRESHOLD = 0.90; // 90%
const SEARCH_STALL_MINUTES = 30;

/**
 * Get fleet status with health classification.
 * Returns: { nodes: [{ id, status, last_heartbeat, ... }], summary: { online, stale, offline } }
 */
async function fleetStatus() {
  const res = await fetch(`${API_BASE}/api/fleet`);
  if (!res.ok) throw new Error(`Fleet status failed: ${res.status}`);
  const fleet = await res.json();

  const now = Date.now();
  const nodes = (fleet.workers || fleet.nodes || []).map((node) => {
    const lastSeen = new Date(node.last_heartbeat || node.last_seen).getTime();
    const ageSec = (now - lastSeen) / 1000;

    let status = "online";
    if (ageSec > STALE_WORKER_SECONDS * 2) status = "offline";
    else if (ageSec > STALE_WORKER_SECONDS) status = "stale";

    return { ...node, status, age_seconds: Math.round(ageSec) };
  });

  const summary = {
    online: nodes.filter((n) => n.status === "online").length,
    stale: nodes.filter((n) => n.status === "stale").length,
    offline: nodes.filter((n) => n.status === "offline").length,
    total: nodes.length,
  };

  return { nodes, summary };
}

/**
 * Check API health.
 * Returns: { healthy: boolean, response_time_ms: number }
 */
async function healthCheck() {
  const start = Date.now();
  try {
    const res = await fetch(`${API_BASE}/api/health`);
    const elapsed = Date.now() - start;
    return {
      healthy: res.ok,
      status: res.status,
      response_time_ms: elapsed,
    };
  } catch (err) {
    return {
      healthy: false,
      error: err.message,
      response_time_ms: Date.now() - start,
    };
  }
}

/**
 * Check for stalled searches (no progress for SEARCH_STALL_MINUTES).
 * Returns: { stalled: [{ id, form, last_progress, stall_minutes }] }
 */
async function checkStalledSearches() {
  const res = await fetch(`${API_BASE}/api/search_jobs?status=running`);
  if (!res.ok) throw new Error(`Search jobs failed: ${res.status}`);
  const jobs = await res.json();

  const now = Date.now();
  const stalled = [];

  for (const job of jobs) {
    const lastProgress = new Date(
      job.last_progress || job.updated_at
    ).getTime();
    const stallMin = (now - lastProgress) / 60000;

    if (stallMin > SEARCH_STALL_MINUTES) {
      stalled.push({
        id: job.id,
        form: job.form,
        last_progress: job.last_progress || job.updated_at,
        stall_minutes: Math.round(stallMin),
      });
    }
  }

  return { stalled };
}

/**
 * Run all checks and return a combined alert report.
 * Returns: { alerts: [{ severity, category, message }], status: "ok" | "warning" | "critical" }
 */
async function runAllChecks() {
  const alerts = [];

  // Fleet check
  try {
    const fleet = await fleetStatus();
    if (fleet.summary.total === 0) {
      alerts.push({
        severity: "critical",
        category: "fleet",
        message: "No nodes registered",
      });
    } else if (fleet.summary.online === 0) {
      alerts.push({
        severity: "critical",
        category: "fleet",
        message: `All ${fleet.summary.total} nodes are offline/stale`,
      });
    } else if (fleet.summary.stale > 0) {
      alerts.push({
        severity: "warning",
        category: "fleet",
        message: `${fleet.summary.stale} stale node(s) (heartbeat > ${STALE_WORKER_SECONDS}s)`,
      });
    }
  } catch (err) {
    alerts.push({
      severity: "critical",
      category: "fleet",
      message: `Fleet check failed: ${err.message}`,
    });
  }

  // Health check
  try {
    const health = await healthCheck();
    if (!health.healthy) {
      alerts.push({
        severity: "critical",
        category: "api",
        message: `API unhealthy: ${health.error || `status ${health.status}`}`,
      });
    } else if (health.response_time_ms > 5000) {
      alerts.push({
        severity: "warning",
        category: "api",
        message: `API slow: ${health.response_time_ms}ms response time`,
      });
    }
  } catch (err) {
    alerts.push({
      severity: "critical",
      category: "api",
      message: `Health check failed: ${err.message}`,
    });
  }

  // Search stall check
  try {
    const stalls = await checkStalledSearches();
    for (const s of stalls.stalled) {
      alerts.push({
        severity: "warning",
        category: "search",
        message: `Search ${s.id} (${s.form}) stalled for ${s.stall_minutes} min`,
      });
    }
  } catch (err) {
    alerts.push({
      severity: "warning",
      category: "search",
      message: `Stall check failed: ${err.message}`,
    });
  }

  // Determine overall status
  const hasCritical = alerts.some((a) => a.severity === "critical");
  const hasWarning = alerts.some((a) => a.severity === "warning");
  const status = hasCritical ? "critical" : hasWarning ? "warning" : "ok";

  return { alerts, status };
}

module.exports = {
  fleetStatus,
  healthCheck,
  checkStalledSearches,
  runAllChecks,
};
