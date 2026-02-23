// darkreach-db — Database query skill for all agents.
//
// Thin wrapper around the darkreach REST API for querying prime records,
// search stats, cost data, AI engine state, and project campaigns.
//
// Endpoints used:
//   GET /api/primes          — Prime database (filterable by form, digits, etc.)
//   GET /api/records         — World records per form
//   GET /api/search_jobs     — Active and completed searches
//   GET /api/projects        — Campaign projects
//   GET /api/fleet           — Fleet status (nodes, heartbeats)
//   GET /api/observability/* — Metrics, cost data, performance
//   GET /api/status          — System status summary

const API_BASE = process.env.DARKREACH_API_URL || "https://api.darkreach.ai";

/**
 * Helper: fetch JSON from darkreach API with error handling.
 */
async function apiGet(path, params = {}) {
  const url = new URL(`${API_BASE}${path}`);
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      url.searchParams.set(key, String(value));
    }
  }

  const res = await fetch(url.toString());
  if (!res.ok) {
    throw new Error(`darkreach API ${path}: ${res.status} ${res.statusText}`);
  }
  return res.json();
}

/**
 * Query primes with optional filters.
 * @param {Object} opts - { form, min_digits, max_digits, limit, offset, sort }
 */
async function getPrimes(opts = {}) {
  return apiGet("/api/primes", opts);
}

/**
 * Get world records per form (our best prime for each search form).
 */
async function getRecords() {
  return apiGet("/api/records");
}

/**
 * Get search jobs with optional status filter.
 * @param {string} [status] - "running", "completed", "stopped", "failed"
 */
async function getSearchJobs(status) {
  return apiGet("/api/search_jobs", status ? { status } : {});
}

/**
 * Get a specific search job by ID.
 */
async function getSearchJob(id) {
  return apiGet(`/api/search_jobs/${id}`);
}

/**
 * Get project campaigns.
 */
async function getProjects() {
  return apiGet("/api/projects");
}

/**
 * Get a specific project by ID.
 */
async function getProject(id) {
  return apiGet(`/api/projects/${id}`);
}

/**
 * Get fleet status (online nodes, heartbeats, capacity).
 */
async function getFleet() {
  return apiGet("/api/fleet");
}

/**
 * Get system status summary.
 */
async function getStatus() {
  return apiGet("/api/status");
}

/**
 * Get observability metrics (cost data, performance stats).
 */
async function getObservability() {
  return apiGet("/api/observability/metrics");
}

/**
 * Get AI engine state (scoring weights, recent decisions).
 */
async function getAiEngineState() {
  return apiGet("/api/ai_engine/state");
}

/**
 * Get recent AI engine decisions.
 * @param {number} [limit=20] - Number of decisions to return
 */
async function getAiEngineDecisions(limit = 20) {
  return apiGet("/api/ai_engine/decisions", { limit });
}

/**
 * Get agent budget summaries.
 */
async function getAgentBudgets() {
  return apiGet("/api/agents/budgets");
}

/**
 * Get agent tasks with optional status filter.
 */
async function getAgentTasks(status) {
  return apiGet("/api/agents/tasks", status ? { status } : {});
}

module.exports = {
  getPrimes,
  getRecords,
  getSearchJobs,
  getSearchJob,
  getProjects,
  getProject,
  getFleet,
  getStatus,
  getObservability,
  getAiEngineState,
  getAiEngineDecisions,
  getAgentBudgets,
  getAgentTasks,
};
