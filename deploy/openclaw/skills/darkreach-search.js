// darkreach-search — Search management skill for the Developer agent.
//
// Create, monitor, and manage search campaigns via the darkreach REST API.
//
// Endpoints used:
//   POST /api/searches        — Create a new search
//   GET  /api/search_jobs     — List searches
//   GET  /api/search_jobs/:id — Get search details
//   PUT  /api/search_jobs/:id — Update search (stop/pause/resume)

const API_BASE = process.env.DARKREACH_API_URL || "https://api.darkreach.ai";

/**
 * Create a new search campaign.
 *
 * @param {Object} params
 * @param {string} params.form - Search form (factorial, kbn, palindromic, etc.)
 * @param {Object} params.config - Form-specific configuration
 *
 * Example configs by form:
 *   factorial:    { start: 1000, end: 2000 }
 *   kbn:          { k: 3, base: 2, min_n: 1000, max_n: 5000 }
 *   palindromic:  { base: 10, min_digits: 11, max_digits: 15 }
 *   primorial:    { start: 100, end: 1000 }
 *   twin:         { k: 1, base: 2, min_n: 1000, max_n: 5000 }
 */
async function createSearch(params) {
  const res = await fetch(`${API_BASE}/api/searches`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(params),
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Create search failed: ${res.status} — ${body}`);
  }

  return res.json();
}

/**
 * List search jobs with optional status filter.
 * @param {string} [status] - "running", "completed", "stopped", "failed"
 */
async function listSearches(status) {
  const url = new URL(`${API_BASE}/api/search_jobs`);
  if (status) url.searchParams.set("status", status);

  const res = await fetch(url.toString());
  if (!res.ok) throw new Error(`List searches failed: ${res.status}`);
  return res.json();
}

/**
 * Get detailed status of a specific search.
 */
async function getSearch(id) {
  const res = await fetch(`${API_BASE}/api/search_jobs/${id}`);
  if (!res.ok) throw new Error(`Get search ${id} failed: ${res.status}`);
  return res.json();
}

/**
 * Stop a running search.
 */
async function stopSearch(id) {
  const res = await fetch(`${API_BASE}/api/search_jobs/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action: "stop" }),
  });

  if (!res.ok) throw new Error(`Stop search ${id} failed: ${res.status}`);
  return res.json();
}

/**
 * Pause a running search (can be resumed later).
 */
async function pauseSearch(id) {
  const res = await fetch(`${API_BASE}/api/search_jobs/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action: "pause" }),
  });

  if (!res.ok) throw new Error(`Pause search ${id} failed: ${res.status}`);
  return res.json();
}

/**
 * Resume a paused search.
 */
async function resumeSearch(id) {
  const res = await fetch(`${API_BASE}/api/search_jobs/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action: "resume" }),
  });

  if (!res.ok) throw new Error(`Resume search ${id} failed: ${res.status}`);
  return res.json();
}

module.exports = {
  createSearch,
  listSearches,
  getSearch,
  stopSearch,
  pauseSearch,
  resumeSearch,
};
