//! # Dashboard — Web Server and Fleet Coordination Hub
//!
//! Runs an Axum HTTP server that serves the Next.js frontend, provides REST API
//! endpoints for prime data, and coordinates the distributed worker fleet via
//! WebSocket and HTTP heartbeat.

pub(crate) mod middleware_auth;
mod openapi;
mod routes_agents;
mod routes_audit;
mod routes_auth;
mod routes_docs;
mod routes_fleet;
mod routes_health;
mod routes_jobs;
mod routes_notifications;
mod routes_observability;
mod routes_operator;
mod routes_prime_verification;
mod routes_primes;
mod routes_projects;
mod routes_releases;
mod routes_resources;
pub(crate) mod response;
mod routes_schedules;
mod routes_searches;
mod routes_sieve;
mod routes_status;
mod routes_strategy;
mod routes_verify;
mod websocket;

use crate::{agent, ai_engine, db, events, fleet, metrics, project, prom_metrics, verify};
use anyhow::Result;
use axum::extract::Request;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Timelike, Utc};
use governor::clock::{Clock, DefaultClock};
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Instrument};

use std::path::Path;
use std::sync::Arc;

/// Lock a mutex, recovering from poisoning.
pub(super) fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ── Per-route rate limiting ────────────────────────────────────────────
//
// Requests are classified into tiers based on HTTP method and path. Each
// tier carries its own governor rate limiter keyed by client IP, so bursty
// operator heartbeats (600/min) can't starve public read traffic (300/min)
// and vice-versa. A high-limit global limiter (1000/min) acts as a safety
// net across all tiers.

/// Alias for a governor rate limiter keyed by IP address string.
type KeyedLimiter = RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>;

/// Rate limit tiers — each maps to an independent token-bucket limiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateLimitTier {
    /// GET /api/primes/*, GET /api/stats/*, /healthz, /readyz, /metrics — 300/min
    PublicRead,
    /// POST /api/v1/operators/register — 30/min
    PublicWrite,
    /// /api/auth/* and POST /api/v1/operators/rotate-key — 60/min
    Auth,
    /// /api/v1/nodes/heartbeat, /api/v1/nodes/work, /api/v1/nodes/result — 600/min
    Operator,
    /// All other admin routes under /api/ — 180/min
    Admin,
    /// Everything else (static files, /ws, etc.) — 120/min
    Default,
}

/// Classify an incoming request into a rate limit tier based on method and path.
fn classify_rate_limit(method: &Method, path: &str) -> RateLimitTier {
    match (method, path) {
        // Health / readiness / metrics probes
        (&Method::GET, "/healthz" | "/readyz" | "/metrics") => RateLimitTier::PublicRead,
        // Public read endpoints — primes, stats, and their sub-routes
        (&Method::GET, p)
            if p.starts_with("/api/primes") || p.starts_with("/api/stats") =>
        {
            RateLimitTier::PublicRead
        }
        // Operator registration (unauthenticated, expensive) — tightest limit
        (&Method::POST, "/api/v1/operators/register" | "/api/v1/register") => {
            RateLimitTier::PublicWrite
        }
        // Auth-related routes
        (_, p) if p.starts_with("/api/auth/") => RateLimitTier::Auth,
        (&Method::POST, "/api/v1/operators/rotate-key") => RateLimitTier::Auth,
        // Operator node hot-path: heartbeat, work claiming, result submission
        (_, "/api/v1/nodes/heartbeat" | "/api/v1/nodes/work" | "/api/v1/nodes/result") => {
            RateLimitTier::Operator
        }
        // Legacy worker routes that map to the same handlers
        (_, "/api/v1/worker/heartbeat" | "/api/v1/work" | "/api/v1/result") => {
            RateLimitTier::Operator
        }
        // Everything else under /api/ is admin
        (_, p) if p.starts_with("/api/") => RateLimitTier::Admin,
        // Non-API requests (static files, WebSocket upgrade, index)
        _ => RateLimitTier::Default,
    }
}

/// Collection of per-tier rate limiters, each a governor token-bucket keyed
/// by client IP address.
pub struct RateLimiters {
    public_read: Arc<KeyedLimiter>,
    public_write: Arc<KeyedLimiter>,
    auth: Arc<KeyedLimiter>,
    operator: Arc<KeyedLimiter>,
    admin: Arc<KeyedLimiter>,
    default: Arc<KeyedLimiter>,
    /// Safety-net global limiter applied to every request regardless of tier.
    global: Arc<KeyedLimiter>,
}

impl RateLimiters {
    /// Build all tier limiters with their configured quotas.
    fn new() -> Self {
        Self {
            public_read: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(300).unwrap()),
            )),
            public_write: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(30).unwrap()),
            )),
            auth: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(60).unwrap()),
            )),
            operator: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(600).unwrap()),
            )),
            admin: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(180).unwrap()),
            )),
            default: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(120).unwrap()),
            )),
            global: Arc::new(RateLimiter::keyed(
                Quota::per_minute(NonZeroU32::new(1000).unwrap()),
            )),
        }
    }

    /// Return the tier-specific limiter for a given tier.
    fn limiter_for(&self, tier: RateLimitTier) -> &KeyedLimiter {
        match tier {
            RateLimitTier::PublicRead => &self.public_read,
            RateLimitTier::PublicWrite => &self.public_write,
            RateLimitTier::Auth => &self.auth,
            RateLimitTier::Operator => &self.operator,
            RateLimitTier::Admin => &self.admin,
            RateLimitTier::Default => &self.default,
        }
    }
}

pub struct AppState {
    pub db: db::Database,
    pub database_url: String,
    pub checkpoint_path: PathBuf,
    pub coordinator_hostname: String,
    pub coordinator_metrics: Mutex<Option<metrics::HardwareMetrics>>,
    pub event_bus: events::EventBus,
    pub agents: Mutex<agent::AgentManager>,
    pub prom_metrics: prom_metrics::Metrics,
    pub ai_engine: tokio::sync::Mutex<ai_engine::AiEngine>,
    pub rate_limiters: RateLimiters,
}

impl AppState {
    pub(super) async fn get_workers_from_pg(&self) -> Vec<fleet::WorkerState> {
        // Prefer Redis for real-time worker state (sub-ms reads, automatic TTL expiry).
        // Falls back to PostgreSQL when Redis is not configured.
        // Read from Redis first (sub-ms), fall back to PG on error or empty result.
        // Workers that register via PG heartbeat (not HTTP) only appear in PG,
        // so we also check PG when Redis returns an empty set.
        let rows = if self.db.redis().is_some() {
            match self.db.redis_get_all_workers().await {
                Ok(rows) if !rows.is_empty() => rows,
                Ok(_empty) => {
                    // Redis has no workers — check PG (workers may register via PG RPC directly)
                    match self.db.get_all_workers().await {
                        Ok(rows) => rows,
                        Err(e) => {
                            warn!(error = %e, "failed to read workers from PG");
                            return Vec::new();
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "redis worker read failed, falling back to PG");
                    match self.db.get_all_workers().await {
                        Ok(rows) => rows,
                        Err(e2) => {
                            warn!(error = %e2, "failed to read workers from PG");
                            return Vec::new();
                        }
                    }
                }
            }
        } else {
            match self.db.get_all_workers().await {
                Ok(rows) => rows,
                Err(e) => {
                    warn!(error = %e, "failed to read workers from PG");
                    return Vec::new();
                }
            }
        };
        rows.into_iter()
            .map(|r| {
                let now = chrono::Utc::now();
                let heartbeat_age = (now - r.last_heartbeat).num_seconds().max(0) as u64;
                let uptime = (now - r.registered_at).num_seconds().max(0) as u64;
                fleet::WorkerState {
                    worker_id: r.worker_id,
                    hostname: r.hostname,
                    cores: r.cores as usize,
                    search_type: r.search_type,
                    search_params: r.search_params,
                    tested: r.tested as u64,
                    found: r.found as u64,
                    current: r.current,
                    checkpoint: r.checkpoint,
                    metrics: r.metrics.and_then(|v| serde_json::from_value(v).ok()),
                    uptime_secs: uptime,
                    last_heartbeat_secs_ago: heartbeat_age,
                    last_heartbeat: std::time::Instant::now(),
                    registered_at: std::time::Instant::now(),
                }
            })
            .collect()
    }

    pub fn with_db(db: db::Database, database_url: &str, checkpoint_path: PathBuf) -> Arc<Self> {
        Arc::new(AppState {
            db,
            database_url: database_url.to_string(),
            checkpoint_path,
            coordinator_hostname: gethostname(),
            coordinator_metrics: Mutex::new(None),
            event_bus: events::EventBus::new(),
            agents: Mutex::new(agent::AgentManager::new()),
            prom_metrics: prom_metrics::Metrics::new(),
            ai_engine: tokio::sync::Mutex::new(ai_engine::AiEngine::new()),
            rate_limiters: RateLimiters::new(),
        })
    }
}

pub fn gethostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .or_else(|_| sysinfo::System::host_name().ok_or(std::env::VarError::NotPresent))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Middleware that records HTTP request duration into the Prometheus histogram,
/// generates (or propagates) a request ID for correlation, and wraps the
/// request in a tracing span using `.instrument()` for proper async propagation.
async fn metrics_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let method = req.method().to_string();
    let raw_path = req.uri().path().to_string();
    let norm_path = normalize_path(&raw_path);
    let start = std::time::Instant::now();

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %method,
        path = %raw_path,
    );
    let response = next.run(req).instrument(span).await;

    let duration = start.elapsed().as_secs_f64();
    state
        .prom_metrics
        .http_request_duration
        .get_or_create(&prom_metrics::HttpLabel {
            method,
            path: norm_path,
        })
        .observe(duration);

    let mut response = response;
    response.headers_mut().insert(
        "x-request-id",
        request_id
            .parse()
            .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );
    response
}

/// Normalize URL path to collapse high-cardinality segments (UUIDs, numeric IDs)
/// into placeholders, preventing histogram label explosion.
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if seg.is_empty() {
                seg.to_string()
            } else if seg.chars().all(|c| c.is_ascii_digit()) {
                ":id".to_string()
            } else if seg.len() == 36 && seg.chars().filter(|c| *c == '-').count() == 4 {
                ":uuid".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Extract client IP from the request for rate-limit keying.
///
/// Checks (in order): `X-Forwarded-For` first entry, `X-Real-Ip`, then
/// falls back to the connected peer address from axum's `ConnectInfo`.
/// Returns `"unknown"` if none are available.
fn extract_client_ip(req: &Request) -> String {
    // X-Forwarded-For may contain a comma-separated list; take the first (client) IP.
    if let Some(xff) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = xff.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if let Some(real_ip) = req
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
    {
        return real_ip.trim().to_string();
    }
    // Fallback: peer address from the socket (only available with ConnectInfo).
    req.extensions()
        .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rate-limit middleware that enforces per-tier and global token-bucket limits.
///
/// The request is classified into a [`RateLimitTier`] via [`classify_rate_limit`],
/// and both the tier-specific and global limiters are checked against the
/// client's IP address. If either rejects, a `429 Too Many Requests` response
/// is returned with a `Retry-After` header indicating seconds until the next
/// token is available.
async fn rate_limit_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let client_ip = extract_client_ip(&req);
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let tier = classify_rate_limit(&method, &path);
    let limiter = state.rate_limiters.limiter_for(tier);

    // Check the tier-specific limiter first.
    if let Err(not_until) = limiter.check_key(&client_ip) {
        let retry_after = not_until.wait_time_from(DefaultClock::default().now());
        let secs = retry_after.as_secs().max(1);
        tracing::debug!(
            ip = %client_ip,
            tier = ?tier,
            retry_after_secs = secs,
            "rate limited"
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("retry-after", secs.to_string()),
                ("x-ratelimit-tier", format!("{:?}", tier)),
            ],
            Json(serde_json::json!({
                "error": "Too many requests",
                "tier": format!("{:?}", tier),
                "retry_after_secs": secs,
            })),
        )
            .into_response();
    }

    // Check the global safety-net limiter.
    if let Err(not_until) = state.rate_limiters.global.check_key(&client_ip) {
        let retry_after = not_until.wait_time_from(DefaultClock::default().now());
        let secs = retry_after.as_secs().max(1);
        tracing::debug!(
            ip = %client_ip,
            tier = ?tier,
            retry_after_secs = secs,
            "global rate limited"
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("retry-after", secs.to_string()),
                ("x-ratelimit-tier", "Global".to_string()),
            ],
            Json(serde_json::json!({
                "error": "Too many requests",
                "tier": "Global",
                "retry_after_secs": secs,
            })),
        )
            .into_response();
    }

    next.run(req).await
}

/// Middleware that inserts the `X-API-Version: 1` header into every HTTP response.
/// This allows clients to detect which API version they are communicating with,
/// enabling forward-compatible version negotiation.
async fn api_version_middleware(req: Request, next: Next) -> axum::response::Response {
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert("X-API-Version", HeaderValue::from_static("1"));
    response
}

/// Handler for `/api/version` — returns the current API version metadata.
///
/// Response format:
/// ```json
/// {"data": {"version": "1", "supported": ["1"], "deprecated": []}}
/// ```
///
/// Clients can use the `supported` array to discover available API versions
/// and the `deprecated` array to plan migrations away from sunset versions.
async fn handler_api_version() -> impl IntoResponse {
    Json(serde_json::json!({
        "data": {
            "version": "1",
            "supported": ["1"],
            "deprecated": []
        }
    }))
}

/// Build the public API routes shared between `/api` (unversioned) and `/api/v1` (versioned).
///
/// Returns a `Router` with relative paths (e.g., `/status`, `/primes`, `/stats`).
/// The caller nests this under the desired prefix via `Router::nest()` so that the
/// same handler functions serve both `/api/status` and `/api/v1/status`, etc.
fn public_api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(routes_status::handler_api_status))
        .route("/docs", get(routes_docs::handler_api_docs))
        .route("/docs/search", get(routes_docs::handler_api_docs_search))
        .route(
            "/docs/roadmaps/{slug}",
            get(routes_docs::handler_api_doc_roadmap),
        )
        .route(
            "/docs/agent/{slug}",
            get(routes_docs::handler_api_doc_agent),
        )
        .route("/docs/{slug}", get(routes_docs::handler_api_doc))
        .route("/export", get(routes_status::handler_api_export))
        .route("/ws-snapshot", get(routes_status::handler_api_ws_snapshot))
        .route("/fleet", get(routes_fleet::handler_api_fleet))
        .route(
            "/searches",
            get(routes_searches::handler_api_searches_list)
                .post(routes_searches::handler_api_searches_create),
        )
        .route(
            "/searches/{id}",
            get(routes_searches::handler_api_searches_get)
                .delete(routes_searches::handler_api_searches_stop),
        )
        .route(
            "/searches/{id}/pause",
            post(routes_searches::handler_api_searches_pause),
        )
        .route(
            "/searches/{id}/resume",
            post(routes_searches::handler_api_searches_resume),
        )
        .route(
            "/fleet/workers/{worker_id}/stop",
            post(routes_fleet::handler_fleet_worker_stop),
        )
        .route(
            "/search_jobs",
            get(routes_jobs::handler_api_search_jobs_list)
                .post(routes_jobs::handler_api_search_jobs_create),
        )
        .route(
            "/search_jobs/{id}",
            get(routes_jobs::handler_api_search_job_get),
        )
        .route(
            "/search_jobs/{id}/cancel",
            post(routes_jobs::handler_api_search_job_cancel),
        )
        .route(
            "/notifications",
            get(routes_notifications::handler_api_notifications),
        )
        .route("/events", get(routes_notifications::handler_api_events))
        .route(
            "/observability/metrics",
            get(routes_observability::handler_metrics),
        )
        .route(
            "/observability/logs",
            get(routes_observability::handler_logs),
        )
        .route(
            "/observability/report",
            get(routes_observability::handler_report),
        )
        .route(
            "/observability/workers/top",
            get(routes_observability::handler_top_workers),
        )
        .route(
            "/observability/catalog",
            get(routes_observability::handler_catalog),
        )
        .route(
            "/agents/tasks",
            get(routes_agents::handler_api_agent_tasks)
                .post(routes_agents::handler_api_agent_task_create),
        )
        .route(
            "/agents/tasks/{id}",
            get(routes_agents::handler_api_agent_task_get),
        )
        .route(
            "/agents/tasks/{id}/cancel",
            post(routes_agents::handler_api_agent_task_cancel),
        )
        .route(
            "/agents/events",
            get(routes_agents::handler_api_agent_events),
        )
        .route(
            "/agents/templates",
            get(routes_agents::handler_api_agent_templates),
        )
        .route(
            "/agents/templates/{name}/expand",
            post(routes_agents::handler_api_agent_template_expand),
        )
        .route(
            "/agents/tasks/{id}/children",
            get(routes_agents::handler_api_agent_task_children),
        )
        .route(
            "/agents/budgets",
            get(routes_agents::handler_api_agent_budgets)
                .put(routes_agents::handler_api_agent_budget_update),
        )
        .route(
            "/primes/{id}/verify",
            post(routes_verify::handler_api_prime_verify),
        )
        .route(
            "/agents/memory",
            get(routes_agents::handler_api_agent_memory_list)
                .post(routes_agents::handler_api_agent_memory_upsert),
        )
        .route(
            "/agents/memory/{key}",
            axum::routing::delete(routes_agents::handler_api_agent_memory_delete),
        )
        .route("/agents/roles", get(routes_agents::handler_api_agent_roles))
        .route(
            "/agents/roles/{name}",
            get(routes_agents::handler_api_agent_role_get),
        )
        .route(
            "/agents/roles/{name}/templates",
            get(routes_agents::handler_api_agent_role_templates),
        )
        .route(
            "/projects",
            get(routes_projects::handler_api_projects_list)
                .post(routes_projects::handler_api_projects_create),
        )
        .route(
            "/projects/import",
            post(routes_projects::handler_api_projects_import),
        )
        .route(
            "/projects/{slug}",
            get(routes_projects::handler_api_project_get),
        )
        .route(
            "/projects/{slug}/activate",
            post(routes_projects::handler_api_project_activate),
        )
        .route(
            "/projects/{slug}/pause",
            post(routes_projects::handler_api_project_pause),
        )
        .route(
            "/projects/{slug}/cancel",
            post(routes_projects::handler_api_project_cancel),
        )
        .route(
            "/projects/{slug}/events",
            get(routes_projects::handler_api_project_events),
        )
        .route(
            "/projects/{slug}/cost",
            get(routes_projects::handler_api_project_cost),
        )
        .route(
            "/releases/worker",
            get(routes_releases::handler_releases_list)
                .post(routes_releases::handler_releases_upsert),
        )
        .route(
            "/releases/events",
            get(routes_releases::handler_releases_events),
        )
        .route(
            "/releases/health",
            get(routes_releases::handler_releases_health),
        )
        .route(
            "/releases/rollout",
            post(routes_releases::handler_releases_rollout),
        )
        .route(
            "/releases/rollback",
            post(routes_releases::handler_releases_rollback),
        )
        // Audit log (admin-only)
        .route("/audit", get(routes_audit::handler_audit_list))
        // Strategy engine
        .route(
            "/strategy/status",
            get(routes_strategy::handler_strategy_status),
        )
        .route(
            "/strategy/decisions",
            get(routes_strategy::handler_strategy_decisions),
        )
        .route(
            "/strategy/scores",
            get(routes_strategy::handler_strategy_scores),
        )
        .route(
            "/strategy/config",
            get(routes_strategy::handler_strategy_config_get)
                .put(routes_strategy::handler_strategy_config_put),
        )
        .route(
            "/strategy/decisions/{id}/override",
            post(routes_strategy::handler_strategy_override),
        )
        .route(
            "/strategy/tick",
            post(routes_strategy::handler_strategy_tick),
        )
        .route(
            "/strategy/ai-engine",
            get(routes_strategy::handler_ai_engine_status),
        )
        .route(
            "/strategy/ai-decisions",
            get(routes_strategy::handler_ai_engine_decisions),
        )
        // Prime data API
        .route("/stats", get(routes_primes::handler_api_stats))
        .route("/stats/timeline", get(routes_primes::handler_api_timeline))
        .route(
            "/stats/distribution",
            get(routes_primes::handler_api_distribution),
        )
        .route(
            "/stats/leaderboard",
            get(routes_primes::handler_api_leaderboard),
        )
        .route(
            "/stats/tags",
            get(routes_primes::handler_api_tag_distribution),
        )
        .route("/primes", get(routes_primes::handler_api_primes_list))
        .route("/primes/{id}", get(routes_primes::handler_api_prime_get))
        .route(
            "/primes/{id}/verifications",
            get(routes_prime_verification::handler_prime_verifications),
        )
        // Distributed prime verification queue
        .route(
            "/prime-verification/stats",
            get(routes_prime_verification::handler_stats),
        )
        .route(
            "/prime-verification/claim",
            post(routes_prime_verification::handler_claim),
        )
        .route(
            "/prime-verification/{id}/submit",
            post(routes_prime_verification::handler_submit),
        )
        .route(
            "/prime-verification/reclaim",
            post(routes_prime_verification::handler_reclaim),
        )
        // Schedule CRUD API
        .route(
            "/schedules",
            get(routes_schedules::handler_api_schedules_list)
                .post(routes_schedules::handler_api_schedules_create),
        )
        .route(
            "/schedules/{id}",
            axum::routing::put(routes_schedules::handler_api_schedules_update)
                .delete(routes_schedules::handler_api_schedules_delete),
        )
        .route(
            "/schedules/{id}/toggle",
            axum::routing::put(routes_schedules::handler_api_schedules_toggle),
        )
        .route("/records", get(routes_projects::handler_api_records))
        .route(
            "/records/refresh",
            post(routes_projects::handler_api_records_refresh),
        )
        .route("/auth/profile", get(routes_auth::handler_api_profile))
        .route("/auth/me", get(routes_auth::handler_api_me))
}

/// Build the CORS layer from environment or sensible defaults.
///
/// Set `CORS_ORIGINS` to a comma-separated list of allowed origins.
/// Defaults to darkreach.ai production domains plus localhost dev servers.
fn build_cors_layer() -> CorsLayer {
    let origins_str = std::env::var("CORS_ORIGINS").unwrap_or_else(|_| {
        "https://darkreach.ai,https://app.darkreach.ai,http://localhost:3000,http://localhost:3001"
            .to_string()
    });
    let allowed: Vec<HeaderValue> = origins_str
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();
    if allowed.is_empty() {
        CorsLayer::permissive()
    } else {
        CorsLayer::new()
            .allow_origin(allowed)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    }
}

pub fn build_router(state: Arc<AppState>, static_dir: Option<&Path>) -> Router {
    // Public API routes are registered under both /api (unversioned, backward-compatible)
    // and /api/v1 (versioned, for forward compatibility). Both prefixes serve the
    // exact same handlers — no code duplication.
    let mut app = Router::new()
        .route("/ws", get(websocket::handler_ws))
        // API version discovery endpoint (not nested — lives at /api/version only)
        .route("/api/version", get(handler_api_version))
        // Public API: unversioned routes at /api/*
        .nest("/api", public_api_routes())
        // Public API: versioned aliases at /api/v1/*
        .nest("/api/v1", public_api_routes())
        .route("/healthz", get(routes_health::handler_healthz))
        .route("/readyz", get(routes_health::handler_readyz))
        .route("/metrics", get(routes_health::handler_metrics))
        // Operator public API (v1) — domain-specific routes
        .route(
            "/api/v1/operators/register",
            post(routes_operator::handler_v1_register),
        )
        .route(
            "/api/v1/nodes/register",
            post(routes_operator::handler_v1_worker_register),
        )
        .route(
            "/api/v1/nodes/heartbeat",
            post(routes_operator::handler_v1_worker_heartbeat),
        )
        .route(
            "/api/v1/nodes/latest",
            get(routes_operator::handler_worker_latest),
        )
        .route("/api/v1/nodes/work", get(routes_operator::handler_v1_work))
        .route(
            "/api/v1/nodes/result",
            post(routes_operator::handler_v1_result),
        )
        .route(
            "/api/v1/operators/stats",
            get(routes_operator::handler_v1_stats),
        )
        .route(
            "/api/v1/operators/leaderboard",
            get(routes_operator::handler_v1_leaderboard),
        )
        .route(
            "/api/v1/operators/me/nodes",
            get(routes_operator::handler_v1_operator_nodes),
        )
        .route(
            "/api/v1/operators/rotate-key",
            post(routes_operator::handler_v1_rotate_key),
        )
        // Legacy operator routes (kept for backward compatibility, 2 release cycles).
        // Note: /api/v1/stats was a legacy alias for operator stats but now serves
        // public API stats via the versioned prefix. Use /api/v1/operators/stats
        // for operator-specific statistics instead.
        .route(
            "/api/v1/register",
            post(routes_operator::handler_v1_register),
        )
        .route(
            "/api/v1/worker/register",
            post(routes_operator::handler_v1_worker_register),
        )
        .route(
            "/api/v1/worker/heartbeat",
            post(routes_operator::handler_v1_worker_heartbeat),
        )
        .route(
            "/api/v1/worker/latest",
            get(routes_operator::handler_worker_latest),
        )
        .route("/api/v1/work", get(routes_operator::handler_v1_work))
        .route("/api/v1/result", post(routes_operator::handler_v1_result))
        .route(
            "/api/v1/leaderboard",
            get(routes_operator::handler_v1_leaderboard),
        )
        .route(
            "/api/volunteer/worker/latest",
            get(routes_operator::handler_worker_latest),
        )
        // Shared sieve cache endpoints (50 MB body limit for uploads)
        .route(
            "/api/v1/sieve/{hash}",
            axum::routing::put(routes_sieve::handler_v1_sieve_upload)
                .get(routes_sieve::handler_v1_sieve_download)
                .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)),
        )
        .route(
            "/api/v1/sieve/{hash}/relay",
            post(routes_sieve::handler_v1_sieve_relay_announce),
        )
        .route(
            "/api/v1/sieves",
            get(routes_sieve::handler_v1_sieves_list),
        )
        // Resource endpoints
        .route(
            "/api/resources/summary",
            get(routes_resources::handler_resources_summary),
        )
        .route(
            "/api/resources/rates",
            get(routes_resources::handler_resources_rates),
        );

    if let Some(dir) = static_dir {
        app = app.fallback_service(ServeDir::new(dir).append_index_html_on_directories(true));
    } else {
        app = app.route("/", get(routes_status::handler_index));
    }

    app.layer(build_cors_layer())
    .layer(CatchPanicLayer::new())
    .layer(axum::middleware::from_fn(api_version_middleware))
    .layer(axum::middleware::from_fn_with_state(
        state.clone(),
        rate_limit_middleware,
    ))
    .layer(axum::middleware::from_fn_with_state(
        state.clone(),
        metrics_middleware,
    ))
    .layer(TraceLayer::new_for_http())
    .layer(RequestBodyLimitLayer::new(1024 * 1024))
    .layer(TimeoutLayer::with_status_code(
        StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(30),
    ))
    .with_state(state)
}

pub async fn run(
    port: u16,
    database_url: &str,
    checkpoint_path: &Path,
    static_dir: Option<&Path>,
) -> Result<()> {
    let database = db::Database::connect(database_url).await?;
    let (ws_tx, _) = tokio::sync::broadcast::channel::<String>(256);
    let state = AppState::with_db(database, database_url, checkpoint_path.to_path_buf());
    state.event_bus.set_ws_sender(ws_tx.clone());
    let app = build_router(state.clone(), static_dir);

    // Background task: prune stale workers, reclaim stale blocks, poll searches, collect metrics
    let prune_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut sys = sysinfo::System::new();
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut last_metrics_sample = std::time::Instant::now() - Duration::from_secs(60);
        let mut last_worker_sample = std::time::Instant::now() - Duration::from_secs(120);
        let mut last_housekeeping = std::time::Instant::now() - Duration::from_secs(3600);
        let mut last_reliability_refresh = std::time::Instant::now() - Duration::from_secs(300);
        let mut last_worker_speed_refresh = std::time::Instant::now() - Duration::from_secs(300);
        let mut last_event_id: u64 = 0;
        let mut last_tick = std::time::Instant::now();
        let mut event_counts = std::collections::HashMap::<String, i64>::new();
        let log_retention_days: i64 = std::env::var("OBS_LOG_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        let metric_retention_days: i64 = std::env::var("OBS_METRIC_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);
        let rollup_retention_days: i64 = std::env::var("OBS_ROLLUP_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(365);
        let daily_rollup_retention_days: i64 = std::env::var("OBS_DAILY_ROLLUP_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1825);
        loop {
            interval.tick().await;
            let tick_now = std::time::Instant::now();
            let tick_interval_ms = tick_now
                .duration_since(last_tick)
                .as_millis()
                .min(i64::MAX as u128) as i64;
            let tick_drift_ms = tick_interval_ms - 30_000;
            last_tick = tick_now;
            if let Err(e) = prune_state.db.prune_stale_workers(120).await {
                warn!(error = %e, "failed to prune stale workers");
            }
            match prune_state.db.rotate_agent_budget_periods().await {
                Ok(n) if n > 0 => info!(count = n, "rotated budget periods"),
                Err(e) => warn!(error = %e, "failed to rotate budget periods"),
                _ => {}
            }
            match prune_state.db.reclaim_stale_blocks(120).await {
                Ok(n) if n > 0 => info!(count = n, "reclaimed stale work blocks"),
                Err(e) => warn!(error = %e, "failed to reclaim stale blocks"),
                _ => {}
            }
            // Operator blocks get a 24-hour timeout (86400s) vs 2-min for internal workers
            match prune_state.db.reclaim_stale_operator_blocks(86400).await {
                Ok(n) if n > 0 => info!(count = n, "reclaimed stale operator blocks"),
                Err(e) => warn!(error = %e, "failed to reclaim stale operator blocks"),
                _ => {}
            }

            // ── Verification pipeline: queue unverified operator blocks ──
            match prune_state.db.get_unverified_operator_blocks(20).await {
                Ok(blocks) => {
                    for block in blocks {
                        // Look up operator trust level
                        let trust_level = if let Some(vol_id) = block.volunteer_id {
                            prune_state
                                .db
                                .get_operator_trust(vol_id)
                                .await
                                .ok()
                                .flatten()
                                .map(|t| t.trust_level)
                                .unwrap_or(1)
                        } else {
                            1
                        };

                        let quorum = verify::required_quorum(trust_level, &block.search_type);

                        if quorum >= 2 {
                            // Check if already queued for verification
                            let already_queued = prune_state
                                .db
                                .has_pending_verification(block.block_id as i64)
                                .await
                                .unwrap_or(false);
                            if !already_queued {
                                // Fetch block details for verification queue
                                if let Ok(Some(wb)) = prune_state
                                    .db
                                    .get_work_block_details(block.block_id as i64)
                                    .await
                                {
                                    if let Err(e) = prune_state
                                        .db
                                        .queue_verification(
                                            block.block_id as i64,
                                            block.search_job_id,
                                            wb.block_start,
                                            wb.block_end,
                                            wb.tested,
                                            wb.found,
                                            &wb.claimed_by,
                                            block.volunteer_id,
                                        )
                                        .await
                                    {
                                        warn!(
                                            block_id = block.block_id,
                                            error = %e,
                                            "failed to queue verification"
                                        );
                                    }
                                }
                            }
                        } else {
                            // Trusted or provable form: mark verified directly
                            if let Err(e) = prune_state.db.mark_block_verified(block.block_id).await
                            {
                                warn!(
                                    block_id = block.block_id,
                                    error = %e,
                                    "failed to mark block verified"
                                );
                            }
                            // Record valid result for trust advancement
                            if let Some(vol_id) = block.volunteer_id {
                                let _ = prune_state.db.record_valid_result(vol_id).await;
                            }
                        }
                    }
                }
                Err(e) => warn!(error = %e, "failed to fetch unverified operator blocks"),
            }

            // Refresh node reliability scores every 5 minutes
            if last_reliability_refresh.elapsed() >= Duration::from_secs(300) {
                last_reliability_refresh = std::time::Instant::now();
                // Node reliability is computed on-the-fly via SQL function,
                // so no explicit refresh needed. This is a placeholder for
                // future batch refresh of materialized reliability data.
            }

            // Refresh worker_speed materialized view every 5 minutes
            // (more frequent than hourly housekeeping since work blocks complete often)
            if last_worker_speed_refresh.elapsed() >= Duration::from_secs(300) {
                last_worker_speed_refresh = std::time::Instant::now();
                if let Err(e) = prune_state.db.refresh_worker_speed_view().await {
                    warn!(error = %e, "failed to refresh worker_speed view");
                }
            }

            let fleet_workers = prune_state.get_workers_from_pg().await;
            // Unified AI engine tick: replaces orchestrate_tick + strategy_tick
            // The AI engine runs its own OODA loop (observe → orient → decide → act → learn)
            // and internally calls orchestrate_tick for phase advancement.
            {
                let mut engine = prune_state.ai_engine.lock().await;
                match engine.tick(&prune_state.db).await {
                    Ok(outcome) => {
                        let decision_count = outcome.decisions.len();
                        if decision_count > 0
                            && !matches!(
                                outcome.decisions.first(),
                                Some(ai_engine::Decision::NoAction { .. })
                            )
                        {
                            info!(
                                tick_id = outcome.tick_id,
                                decisions = decision_count,
                                duration_ms = outcome.duration_ms,
                                "ai_engine tick complete"
                            );
                        }
                    }
                    Err(e) => warn!(error = %e, "ai_engine tick failed"),
                }
            }
            prune_state.event_bus.flush();
            {
                let events = prune_state
                    .event_bus
                    .recent_events_since(last_event_id, 200);
                if let Some(last) = events.last() {
                    last_event_id = last.id;
                }
                if !events.is_empty() {
                    for e in &events {
                        *event_counts.entry(e.kind.clone()).or_insert(0) += 1;
                    }
                    let logs: Vec<db::SystemLogEntry> = events
                        .into_iter()
                        .map(|e| {
                            let level = match e.kind.as_str() {
                                "error" => "error",
                                "warning" => "warn",
                                _ => "info",
                            };
                            let ts = std::time::SystemTime::UNIX_EPOCH
                                + std::time::Duration::from_millis(e.timestamp_ms);
                            db::SystemLogEntry {
                                ts: DateTime::<Utc>::from(ts),
                                level: level.to_string(),
                                source: "coordinator".to_string(),
                                component: "event_bus".to_string(),
                                message: e.message,
                                worker_id: None,
                                search_job_id: None,
                                search_id: None,
                                context: Some(serde_json::json!({"kind": e.kind, "elapsed_secs": e.elapsed_secs})),
                            }
                        })
                        .collect();
                    if let Err(e) = prune_state.db.insert_system_logs(&logs).await {
                        warn!(error = %e, "failed to persist event logs");
                    }
                }
            }
            sys.refresh_cpu_all();
            sys.refresh_memory();
            let hw = metrics::collect(&sys);

            // Update Prometheus gauges from hardware metrics and fleet state
            prune_state
                .prom_metrics
                .cpu_usage_percent
                .set(hw.cpu_usage_percent as f64);
            prune_state
                .prom_metrics
                .memory_usage_percent
                .set(hw.memory_usage_percent as f64);
            prune_state
                .prom_metrics
                .workers_connected
                .set(fleet_workers.len() as i64);

            // Connection pool stats
            let pool_size = prune_state.db.pool().size();
            let pool_idle = prune_state.db.pool().num_idle();
            prune_state
                .prom_metrics
                .db_pool_active
                .set((pool_size as i64) - (pool_idle as i64));
            prune_state.prom_metrics.db_pool_idle.set(pool_idle as i64);
            prune_state
                .prom_metrics
                .db_pool_max
                .set(prune_state.db.max_connections() as i64);

            // Read replica pool stats
            let read_pool_size = prune_state.db.read_pool().size();
            let read_pool_idle = prune_state.db.read_pool().num_idle();
            prune_state
                .prom_metrics
                .db_read_pool_active
                .set((read_pool_size as i64) - (read_pool_idle as i64));
            prune_state
                .prom_metrics
                .db_read_pool_idle
                .set(read_pool_idle as i64);

            if let Ok(jobs) = prune_state.db.get_search_jobs().await {
                let active = jobs.iter().filter(|j| j.status == "running").count();
                prune_state
                    .prom_metrics
                    .search_jobs_active
                    .set(active as i64);
            }
            let mut block_summary = None;
            if let Ok(summary) = prune_state.db.get_all_block_summary().await {
                prune_state
                    .prom_metrics
                    .work_blocks_available
                    .set(summary.available);
                prune_state
                    .prom_metrics
                    .work_blocks_claimed
                    .set(summary.claimed);
                block_summary = Some(summary);
            }

            *lock_or_recover(&prune_state.coordinator_metrics) = Some(hw.clone());

            if last_metrics_sample.elapsed() >= Duration::from_secs(60) {
                last_metrics_sample = std::time::Instant::now();
                let now = Utc::now();
                let mut samples: Vec<db::MetricSample> = Vec::new();

                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.cpu_usage_percent".to_string(),
                    value: hw.cpu_usage_percent as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.tick_interval_ms".to_string(),
                    value: tick_interval_ms as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.tick_drift_ms".to_string(),
                    value: tick_drift_ms as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.memory_usage_percent".to_string(),
                    value: hw.memory_usage_percent as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.load_avg_1m".to_string(),
                    value: hw.load_avg_1m,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.load_avg_5m".to_string(),
                    value: hw.load_avg_5m,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "coordinator".to_string(),
                    metric: "coordinator.load_avg_15m".to_string(),
                    value: hw.load_avg_15m,
                    labels: None,
                });

                let total_cores: i64 = fleet_workers.iter().map(|w| w.cores as i64).sum();
                let total_tested: i64 = fleet_workers.iter().map(|w| w.tested as i64).sum();
                let total_found: i64 = fleet_workers.iter().map(|w| w.found as i64).sum();
                let max_heartbeat_age: i64 = fleet_workers
                    .iter()
                    .map(|w| w.last_heartbeat_secs_ago as i64)
                    .max()
                    .unwrap_or(0);
                let avg_heartbeat_age: f64 = if fleet_workers.is_empty() {
                    0.0
                } else {
                    fleet_workers
                        .iter()
                        .map(|w| w.last_heartbeat_secs_ago as f64)
                        .sum::<f64>()
                        / fleet_workers.len() as f64
                };

                samples.push(db::MetricSample {
                    ts: now,
                    scope: "fleet".to_string(),
                    metric: "fleet.workers_connected".to_string(),
                    value: fleet_workers.len() as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "fleet".to_string(),
                    metric: "fleet.total_cores".to_string(),
                    value: total_cores as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "fleet".to_string(),
                    metric: "fleet.total_tested".to_string(),
                    value: total_tested as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "fleet".to_string(),
                    metric: "fleet.total_found".to_string(),
                    value: total_found as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "fleet".to_string(),
                    metric: "fleet.max_heartbeat_age_secs".to_string(),
                    value: max_heartbeat_age as f64,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "fleet".to_string(),
                    metric: "fleet.avg_heartbeat_age_secs".to_string(),
                    value: avg_heartbeat_age,
                    labels: None,
                });

                if let Some(summary) = &block_summary {
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.work_blocks_available".to_string(),
                        value: summary.available as f64,
                        labels: None,
                    });
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.work_blocks_claimed".to_string(),
                        value: summary.claimed as f64,
                        labels: None,
                    });
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.work_blocks_completed".to_string(),
                        value: summary.completed as f64,
                        labels: None,
                    });
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.work_blocks_failed".to_string(),
                        value: summary.failed as f64,
                        labels: None,
                    });
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.block_total_tested".to_string(),
                        value: summary.total_tested as f64,
                        labels: None,
                    });
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.block_total_found".to_string(),
                        value: summary.total_found as f64,
                        labels: None,
                    });
                }

                if let Ok(jobs) = prune_state.db.get_search_jobs().await {
                    let active = jobs.iter().filter(|j| j.status == "running").count();
                    samples.push(db::MetricSample {
                        ts: now,
                        scope: "fleet".to_string(),
                        metric: "fleet.search_jobs_active".to_string(),
                        value: active as f64,
                        labels: None,
                    });
                }

                if let Ok(jobs) = prune_state.db.get_recent_search_jobs(24, 50).await {
                    for job in jobs {
                        let summary = match prune_state.db.get_job_block_summary(job.id).await {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                        let total_blocks = summary.available
                            + summary.claimed
                            + summary.completed
                            + summary.failed;
                        let completion_pct = if total_blocks > 0 {
                            (summary.completed as f64 / total_blocks as f64) * 100.0
                        } else {
                            0.0
                        };
                        let labels = serde_json::json!({
                            "job_id": job.id.to_string(),
                            "search_type": job.search_type,
                            "status": job.status,
                        });

                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.blocks_available".to_string(),
                            value: summary.available as f64,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.blocks_claimed".to_string(),
                            value: summary.claimed as f64,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.blocks_completed".to_string(),
                            value: summary.completed as f64,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.blocks_failed".to_string(),
                            value: summary.failed as f64,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.completion_pct".to_string(),
                            value: completion_pct,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.total_tested".to_string(),
                            value: summary.total_tested as f64,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "search_job".to_string(),
                            metric: "search_job.total_found".to_string(),
                            value: summary.total_found as f64,
                            labels: Some(labels.clone()),
                        });
                    }
                }

                let error_count = *event_counts.get("error").unwrap_or(&0) as f64;
                let warning_count = *event_counts.get("warning").unwrap_or(&0) as f64;
                let prime_count = *event_counts.get("prime").unwrap_or(&0) as f64;
                let milestone_count = *event_counts.get("milestone").unwrap_or(&0) as f64;
                let search_start_count = *event_counts.get("search_start").unwrap_or(&0) as f64;
                let search_done_count = *event_counts.get("search_done").unwrap_or(&0) as f64;
                let total_events: f64 = event_counts.values().copied().sum::<i64>() as f64;
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.total_count".to_string(),
                    value: total_events,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.error_count".to_string(),
                    value: error_count,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.warning_count".to_string(),
                    value: warning_count,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.prime_count".to_string(),
                    value: prime_count,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.milestone_count".to_string(),
                    value: milestone_count,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.search_start_count".to_string(),
                    value: search_start_count,
                    labels: None,
                });
                samples.push(db::MetricSample {
                    ts: now,
                    scope: "events".to_string(),
                    metric: "events.search_done_count".to_string(),
                    value: search_done_count,
                    labels: None,
                });
                event_counts.clear();

                if last_worker_sample.elapsed() >= Duration::from_secs(120) {
                    last_worker_sample = std::time::Instant::now();
                    for w in &fleet_workers {
                        if let Some(m) = &w.metrics {
                            let labels = serde_json::json!({
                                "worker_id": w.worker_id,
                                "hostname": w.hostname,
                                "search_type": w.search_type,
                            });
                            samples.push(db::MetricSample {
                                ts: now,
                                scope: "worker".to_string(),
                                metric: "worker.cpu_usage_percent".to_string(),
                                value: m.cpu_usage_percent as f64,
                                labels: Some(labels.clone()),
                            });
                            samples.push(db::MetricSample {
                                ts: now,
                                scope: "worker".to_string(),
                                metric: "worker.memory_usage_percent".to_string(),
                                value: m.memory_usage_percent as f64,
                                labels: Some(labels.clone()),
                            });
                            samples.push(db::MetricSample {
                                ts: now,
                                scope: "worker".to_string(),
                                metric: "worker.disk_usage_percent".to_string(),
                                value: m.disk_usage_percent as f64,
                                labels: Some(labels.clone()),
                            });
                        }
                        let labels = serde_json::json!({
                            "worker_id": w.worker_id,
                            "hostname": w.hostname,
                            "search_type": w.search_type,
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "worker".to_string(),
                            metric: "worker.tested".to_string(),
                            value: w.tested as f64,
                            labels: Some(labels.clone()),
                        });
                        samples.push(db::MetricSample {
                            ts: now,
                            scope: "worker".to_string(),
                            metric: "worker.found".to_string(),
                            value: w.found as f64,
                            labels: Some(labels.clone()),
                        });
                    }
                }

                if let Err(e) = prune_state.db.insert_metric_samples(&samples).await {
                    warn!(error = %e, count = samples.len(), "failed to persist metric samples");
                }
            }

            if last_housekeeping.elapsed() >= Duration::from_secs(3600) {
                last_housekeeping = std::time::Instant::now();
                let now = Utc::now();
                let hour_start = now
                    .with_minute(0)
                    .and_then(|t| t.with_second(0))
                    .and_then(|t| t.with_nanosecond(0))
                    .unwrap_or(now);
                let prev_hour = hour_start - chrono::Duration::hours(1);
                if let Err(e) = prune_state.db.rollup_metrics_hour(prev_hour).await {
                    warn!(error = %e, "failed to roll up hourly metrics");
                }
                let day_start = now
                    .with_hour(0)
                    .and_then(|t| t.with_minute(0))
                    .and_then(|t| t.with_second(0))
                    .and_then(|t| t.with_nanosecond(0))
                    .unwrap_or(now);
                let prev_day = day_start - chrono::Duration::days(1);
                if let Err(e) = prune_state.db.rollup_metrics_day(prev_day).await {
                    warn!(error = %e, "failed to roll up daily metrics");
                }
                if let Err(e) = prune_state
                    .db
                    .prune_metric_samples(metric_retention_days)
                    .await
                {
                    warn!(error = %e, "failed to prune metric samples");
                }
                if let Err(e) = prune_state
                    .db
                    .prune_metric_rollups(rollup_retention_days)
                    .await
                {
                    warn!(error = %e, "failed to prune metric rollups");
                }
                if let Err(e) = prune_state
                    .db
                    .prune_metric_rollups_daily(daily_rollup_retention_days)
                    .await
                {
                    warn!(error = %e, "failed to prune daily rollups");
                }
                if let Err(e) = prune_state.db.prune_system_logs(log_retention_days).await {
                    warn!(error = %e, "failed to prune system logs");
                }
                if let Err(e) = prune_state.db.refresh_materialized_views().await {
                    warn!(error = %e, "failed to refresh materialized views");
                }
            }
        }
    });

    // Background task: auto-verify newly discovered primes (60s interval)
    let verify_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await;
        loop {
            interval.tick().await;
            let primes = match verify_state.db.get_unverified_primes(10).await {
                Ok(p) => p,
                Err(e) => {
                    warn!(error = %e, "auto-verify: failed to fetch unverified primes");
                    continue;
                }
            };
            if primes.is_empty() {
                continue;
            }
            info!(count = primes.len(), "auto-verify: checking primes");
            for prime in &primes {
                let prime_clone = prime.clone();
                let result =
                    tokio::task::spawn_blocking(move || verify::verify_prime(&prime_clone)).await;
                match result {
                    Ok(ref vr @ verify::VerifyResult::Verified { ref method, tier }) => {
                        info!(prime_id = prime.id, expression = %prime.expression, method = %method, tier, "auto-verified prime");
                        if let Err(e) = verify_state
                            .db
                            .mark_verified(prime.id, method, tier as i16)
                            .await
                        {
                            warn!(prime_id = prime.id, error = %e, "failed to mark prime verified");
                        }
                        let new_tags = verify::classify_after_verification(prime, vr);
                        if !new_tags.is_empty() {
                            let tag_refs: Vec<&str> = new_tags.iter().map(|s| s.as_str()).collect();
                            if let Err(e) =
                                verify_state.db.add_prime_tags(prime.id, &tag_refs).await
                            {
                                warn!(prime_id = prime.id, error = %e, "failed to add verification tags");
                            }
                        }
                    }
                    Ok(verify::VerifyResult::Failed { reason }) => {
                        warn!(prime_id = prime.id, reason = %reason, "auto-verify failed");
                        if let Err(e) = verify_state
                            .db
                            .mark_verification_failed(prime.id, &reason)
                            .await
                        {
                            warn!(prime_id = prime.id, error = %e, "failed to mark prime verification failed");
                        }
                    }
                    Ok(verify::VerifyResult::Skipped { reason }) => {
                        tracing::debug!(prime_id = prime.id, reason = %reason, "auto-verify skipped");
                    }
                    Err(e) => {
                        warn!(prime_id = prime.id, error = %e, "auto-verify task panicked");
                    }
                }
            }
        }
    });

    // Background task: reclaim stale prime verification tasks (5min interval)
    let pv_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        interval.tick().await;
        loop {
            interval.tick().await;
            match pv_state.db.reclaim_stale_prime_verifications(600).await {
                Ok(n) if n > 0 => info!(count = n, "reclaimed stale prime verification tasks"),
                Err(e) => warn!(error = %e, "failed to reclaim stale prime verifications"),
                _ => {}
            }
        }
    });

    // Background task: agent execution engine (10s interval)
    let agent_state = Arc::clone(&state);
    tokio::spawn(async move {
        let agent_name = format!("coordinator@{}", gethostname());
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.tick().await;
        loop {
            interval.tick().await;
            let completed = lock_or_recover(&agent_state.agents).poll_completed();
            for c in completed {
                let status_str = match &c.status {
                    agent::AgentStatus::Completed => "completed",
                    agent::AgentStatus::Failed { .. } => "failed",
                    agent::AgentStatus::TimedOut => "failed",
                    agent::AgentStatus::Cancelled => "cancelled",
                    agent::AgentStatus::Running => "in_progress",
                };
                let reason = match &c.status {
                    agent::AgentStatus::Failed { reason } => Some(reason.clone()),
                    agent::AgentStatus::TimedOut => Some("Timed out".to_string()),
                    _ => None,
                };
                let (result_json, tokens, cost) = match c.result {
                    Some(ref r) => (
                        Some(serde_json::json!({"text": r.result_text})),
                        r.tokens_used,
                        r.cost_usd,
                    ),
                    None => (
                        reason.as_ref().map(|r| serde_json::json!({"error": r})),
                        0,
                        0.0,
                    ),
                };
                if let Err(e) = agent_state
                    .db
                    .complete_agent_task(c.task_id, status_str, result_json.as_ref(), tokens, cost)
                    .await
                {
                    warn!(task_id = c.task_id, error = %e, "agent: failed to complete task");
                }
                let summary = match &c.status {
                    agent::AgentStatus::Completed => "Task completed".to_string(),
                    agent::AgentStatus::Failed { reason } => format!("Task failed: {}", reason),
                    agent::AgentStatus::TimedOut => "Task timed out".to_string(),
                    agent::AgentStatus::Cancelled => "Task cancelled".to_string(),
                    _ => "Task finished".to_string(),
                };
                let _ = agent_state
                    .db
                    .insert_agent_event(Some(c.task_id), status_str, Some("system"), &summary, None)
                    .await;
                if tokens > 0 || cost > 0.0 {
                    let _ = agent_state
                        .db
                        .update_agent_budget_spending(tokens, cost)
                        .await;
                }
                info!(
                    task_id = c.task_id,
                    status = status_str,
                    tokens,
                    cost,
                    "agent task finished"
                );
                if let Ok(Some(completed_task)) = agent_state.db.get_agent_task(c.task_id).await {
                    if let Some(parent_id) = completed_task.parent_task_id {
                        if status_str == "failed" {
                            if let Ok(Some(parent)) = agent_state.db.get_agent_task(parent_id).await
                            {
                                if parent.on_child_failure == "fail" {
                                    let cancelled = agent_state
                                        .db
                                        .cancel_pending_siblings(parent_id)
                                        .await
                                        .unwrap_or(0);
                                    if cancelled > 0 {
                                        info!(
                                            parent_id,
                                            cancelled, "agent: cancelled pending siblings"
                                        );
                                    }
                                }
                            }
                        }
                        if let Ok(Some(parent)) =
                            agent_state.db.try_complete_parent(parent_id).await
                        {
                            let event_type = if parent.status == "failed" {
                                "parent_failed"
                            } else {
                                "parent_completed"
                            };
                            let _ = agent_state
                                .db
                                .insert_agent_event(
                                    Some(parent_id),
                                    event_type,
                                    None,
                                    &format!(
                                        "Parent task '{}' auto-{}",
                                        parent.title, parent.status
                                    ),
                                    None,
                                )
                                .await;
                            info!(parent_id, status = %parent.status, "agent: parent task auto-completed");
                        }
                    }
                }
            }
            let budget_ok = agent_state.db.check_agent_budget().await.unwrap_or(true);
            if !budget_ok {
                let killed = lock_or_recover(&agent_state.agents).kill_all();
                for task_id in &killed {
                    let _ = agent_state
                        .db
                        .complete_agent_task(
                            *task_id,
                            "failed",
                            Some(&serde_json::json!({"error": "Global budget exceeded"})),
                            0,
                            0.0,
                        )
                        .await;
                    let _ = agent_state
                        .db
                        .insert_agent_event(
                            Some(*task_id),
                            "budget_exceeded",
                            Some("system"),
                            "Killed: global budget exceeded",
                            None,
                        )
                        .await;
                }
                if !killed.is_empty() {
                    warn!(count = killed.len(), task_ids = ?killed, "agent: global budget exceeded, killed agents");
                }
                continue;
            }
            let active = lock_or_recover(&agent_state.agents).active_count();
            if active >= agent::MAX_AGENTS {
                continue;
            }
            let task = match agent_state.db.claim_pending_agent_task(&agent_name).await {
                Ok(Some(t)) => t,
                Ok(None) => continue,
                Err(e) => {
                    warn!(error = %e, "agent: failed to claim task");
                    continue;
                }
            };
            info!(task_id = task.id, title = %task.title, priority = task.priority, model = ?task.agent_model, "agent: claimed task");
            let _ = agent_state
                .db
                .insert_agent_event(
                    Some(task.id),
                    "claimed",
                    Some(&agent_name),
                    &format!("Task claimed by {}", agent_name),
                    None,
                )
                .await;
            let role = if let Some(ref rn) = task.role_name {
                agent_state.db.get_role_by_name(rn).await.ok().flatten()
            } else {
                None
            };
            let context_prompts =
                agent::assemble_context(&task, &agent_state.db, role.as_ref()).await;
            let db_clone = agent_state.db.clone();
            let spawn_result = {
                lock_or_recover(&agent_state.agents).spawn_agent(
                    &task,
                    db_clone,
                    task.max_cost_usd,
                    context_prompts,
                )
            };
            match spawn_result {
                Ok(_info) => {}
                Err(e) => {
                    warn!(task_id = task.id, error = %e, "agent: failed to spawn");
                    let _ = agent_state
                        .db
                        .complete_agent_task(
                            task.id,
                            "failed",
                            Some(&serde_json::json!({"error": e})),
                            0,
                            0.0,
                        )
                        .await;
                    let _ = agent_state
                        .db
                        .insert_agent_event(
                            Some(task.id),
                            "failed",
                            Some("system"),
                            &format!("Failed to spawn: {}", e),
                            None,
                        )
                        .await;
                }
            }
        }
    });

    // Background task: refresh world records from t5k.org (24h interval)
    let records_state = Arc::clone(&state);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        info!("records: initial refresh from t5k.org");
        match project::refresh_all_records(&records_state.db).await {
            Ok(n) => info!(count = n, "records: refreshed forms"),
            Err(e) => warn!(error = %e, "records: refresh failed"),
        }
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
        interval.tick().await;
        loop {
            interval.tick().await;
            info!("records: 24h refresh from t5k.org");
            match project::refresh_all_records(&records_state.db).await {
                Ok(n) => info!(count = n, "records: refreshed forms"),
                Err(e) => warn!(error = %e, "records: refresh failed"),
            }
        }
    });

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    info!(port, "dashboard running");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("dashboard shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! { _ = ctrl_c => info!("received SIGINT, shutting down"), _ = sigterm.recv() => info!("received SIGTERM, shutting down") }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        info!("received SIGINT, shutting down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_preserves_api_routes() {
        assert_eq!(normalize_path("/api/status"), "/api/status");
        assert_eq!(normalize_path("/api/fleet"), "/api/fleet");
        assert_eq!(normalize_path("/metrics"), "/metrics");
    }

    #[test]
    fn normalize_path_collapses_numeric_ids() {
        assert_eq!(
            normalize_path("/api/search_jobs/42"),
            "/api/search_jobs/:id"
        );
        assert_eq!(
            normalize_path("/api/primes/12345/verify"),
            "/api/primes/:id/verify"
        );
    }

    #[test]
    fn normalize_path_collapses_uuids() {
        assert_eq!(
            normalize_path("/api/agents/tasks/550e8400-e29b-41d4-a716-446655440000"),
            "/api/agents/tasks/:uuid"
        );
    }

    #[test]
    fn normalize_path_handles_empty_and_root() {
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn classify_public_read_endpoints() {
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/primes"),
            RateLimitTier::PublicRead,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/primes/42"),
            RateLimitTier::PublicRead,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/stats"),
            RateLimitTier::PublicRead,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/stats/timeline"),
            RateLimitTier::PublicRead,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/healthz"),
            RateLimitTier::PublicRead,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/readyz"),
            RateLimitTier::PublicRead,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/metrics"),
            RateLimitTier::PublicRead,
        );
    }

    #[test]
    fn classify_public_write_endpoints() {
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/operators/register"),
            RateLimitTier::PublicWrite,
        );
        // Legacy alias
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/register"),
            RateLimitTier::PublicWrite,
        );
    }

    #[test]
    fn classify_auth_endpoints() {
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/auth/profile"),
            RateLimitTier::Auth,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/auth/me"),
            RateLimitTier::Auth,
        );
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/operators/rotate-key"),
            RateLimitTier::Auth,
        );
    }

    #[test]
    fn classify_operator_endpoints() {
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/nodes/heartbeat"),
            RateLimitTier::Operator,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/v1/nodes/work"),
            RateLimitTier::Operator,
        );
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/nodes/result"),
            RateLimitTier::Operator,
        );
        // Legacy aliases
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/worker/heartbeat"),
            RateLimitTier::Operator,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/v1/work"),
            RateLimitTier::Operator,
        );
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/v1/result"),
            RateLimitTier::Operator,
        );
    }

    #[test]
    fn classify_admin_endpoints() {
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/fleet"),
            RateLimitTier::Admin,
        );
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/searches"),
            RateLimitTier::Admin,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/strategy/status"),
            RateLimitTier::Admin,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/api/projects"),
            RateLimitTier::Admin,
        );
    }

    #[test]
    fn classify_default_tier() {
        assert_eq!(
            classify_rate_limit(&Method::GET, "/"),
            RateLimitTier::Default,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/ws"),
            RateLimitTier::Default,
        );
        assert_eq!(
            classify_rate_limit(&Method::GET, "/favicon.ico"),
            RateLimitTier::Default,
        );
    }

    #[test]
    fn classify_post_to_primes_is_admin_not_public_read() {
        // POST to /api/primes/* should be Admin, not PublicRead (only GET is public read)
        assert_eq!(
            classify_rate_limit(&Method::POST, "/api/primes/42/verify"),
            RateLimitTier::Admin,
        );
    }
}
