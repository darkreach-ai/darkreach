//! OpenAPI specification for the darkreach REST API.
//!
//! Generates an OpenAPI 3.1 document from Rust types via `utoipa`.
//! Swagger UI is served at `/api/docs/openapi` in non-production builds;
//! the raw JSON spec is at `/api/docs/openapi.json`.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::Server;
use utoipa::{Modify, OpenApi};

/// Security scheme modifier that adds JWT Bearer auth to the spec.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(schema) = openapi.components.as_mut() {
            schema.add_security_scheme(
                "bearer_jwt",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Supabase-issued JWT token. Obtain via Supabase Auth login.",
                        ))
                        .build(),
                ),
            );
            schema.add_security_scheme(
                "api_key",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .description(Some(
                            "Operator API key. Obtain via POST /api/v1/operators/register.",
                        ))
                        .build(),
                ),
            );
        }
    }
}

/// Root OpenAPI document for the darkreach API.
///
/// Paths are grouped by tag. The full list of endpoints is registered here;
/// individual handler annotations are added incrementally as the codebase
/// adopts `#[utoipa::path]` decorators.
#[derive(OpenApi)]
#[openapi(
    paths(
        // Health endpoints
        super::routes_health::handler_healthz,
        super::routes_health::handler_readyz,
        // Prime data endpoints
        super::routes_primes::handler_api_stats,
        super::routes_primes::handler_api_timeline,
        super::routes_primes::handler_api_distribution,
        super::routes_primes::handler_api_leaderboard,
        super::routes_primes::handler_api_primes_list,
        super::routes_primes::handler_api_prime_get,
        super::routes_primes::handler_api_tag_distribution,
        // Search endpoints
        super::routes_searches::handler_api_searches_list,
        super::routes_searches::handler_api_searches_create,
        super::routes_searches::handler_api_searches_get,
        super::routes_searches::handler_api_searches_stop,
        super::routes_searches::handler_api_searches_pause,
        super::routes_searches::handler_api_searches_resume,
        // Search job endpoints
        super::routes_jobs::handler_api_search_jobs_list,
        super::routes_jobs::handler_api_search_jobs_create,
        super::routes_jobs::handler_api_search_job_get,
        super::routes_jobs::handler_api_search_job_cancel,
        // Fleet endpoints
        super::routes_fleet::handler_api_fleet,
        super::routes_fleet::handler_fleet_worker_stop,
        // Agent endpoints
        super::routes_agents::handler_api_agent_tasks,
        super::routes_agents::handler_api_agent_task_create,
        super::routes_agents::handler_api_agent_task_get,
        super::routes_agents::handler_api_agent_task_cancel,
        super::routes_agents::handler_api_agent_events,
        super::routes_agents::handler_api_agent_budgets,
        super::routes_agents::handler_api_agent_budget_update,
        super::routes_agents::handler_api_agent_memory_list,
        super::routes_agents::handler_api_agent_memory_upsert,
        super::routes_agents::handler_api_agent_memory_delete,
        super::routes_agents::handler_api_agent_roles,
        super::routes_agents::handler_api_agent_role_get,
        super::routes_agents::handler_api_agent_role_templates,
        super::routes_agents::handler_api_agent_templates,
        super::routes_agents::handler_api_agent_template_expand,
        super::routes_agents::handler_api_agent_task_children,
        // Project endpoints
        super::routes_projects::handler_api_projects_list,
        super::routes_projects::handler_api_projects_create,
        super::routes_projects::handler_api_projects_import,
        super::routes_projects::handler_api_project_get,
        super::routes_projects::handler_api_project_activate,
        super::routes_projects::handler_api_project_pause,
        super::routes_projects::handler_api_project_cancel,
        super::routes_projects::handler_api_project_events,
        super::routes_projects::handler_api_project_cost,
        super::routes_projects::handler_api_records,
        super::routes_projects::handler_api_records_refresh,
        // Release endpoints
        super::routes_releases::handler_releases_list,
        super::routes_releases::handler_releases_events,
        super::routes_releases::handler_releases_health,
        super::routes_releases::handler_releases_upsert,
        super::routes_releases::handler_releases_rollout,
        super::routes_releases::handler_releases_rollback,
        // Observability endpoints
        super::routes_observability::handler_metrics,
        super::routes_observability::handler_logs,
        super::routes_observability::handler_report,
        super::routes_observability::handler_top_workers,
        super::routes_observability::handler_catalog,
        // Notification endpoints
        super::routes_notifications::handler_api_notifications,
        super::routes_notifications::handler_api_events,
        // Verification endpoints
        super::routes_verify::handler_api_prime_verify,
        super::routes_prime_verification::handler_stats,
        super::routes_prime_verification::handler_claim,
        super::routes_prime_verification::handler_submit,
        super::routes_prime_verification::handler_prime_verifications,
        super::routes_prime_verification::handler_reclaim,
        // Schedule endpoints
        super::routes_schedules::handler_api_schedules_list,
        super::routes_schedules::handler_api_schedules_create,
        super::routes_schedules::handler_api_schedules_update,
        super::routes_schedules::handler_api_schedules_toggle,
        super::routes_schedules::handler_api_schedules_delete,
        // Auth endpoints
        super::routes_auth::handler_api_profile,
        super::routes_auth::handler_api_me,
        // Strategy endpoints
        super::routes_strategy::handler_strategy_status,
        super::routes_strategy::handler_strategy_decisions,
        super::routes_strategy::handler_strategy_scores,
        super::routes_strategy::handler_strategy_config_get,
        super::routes_strategy::handler_strategy_config_put,
        super::routes_strategy::handler_strategy_override,
        super::routes_strategy::handler_strategy_tick,
        super::routes_strategy::handler_ai_engine_status,
        super::routes_strategy::handler_ai_engine_decisions,
    ),
    info(
        title = "darkreach API",
        version = "1.0.0",
        description = "Distributed prime number discovery platform — REST API for fleet coordination, search management, and prime data access.",
        contact(name = "darkreach", url = "https://darkreach.ai"),
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
    ),
    servers(
        (url = "https://api.darkreach.ai", description = "Production"),
        (url = "http://localhost:7001", description = "Local development"),
    ),
    tags(
        (name = "health", description = "Health checks and readiness probes"),
        (name = "primes", description = "Prime number data — queries, stats, charts"),
        (name = "searches", description = "Search job management (admin)"),
        (name = "fleet", description = "Fleet overview and worker control (admin)"),
        (name = "agents", description = "AI agent tasks, budgets, memory, roles (admin)"),
        (name = "projects", description = "Project campaign management (admin)"),
        (name = "releases", description = "Worker release channels (admin)"),
        (name = "observability", description = "Metrics, logs, performance reports (admin)"),
        (name = "strategy", description = "AI strategy engine (admin)"),
        (name = "schedules", description = "Agent schedule automation (admin)"),
        (name = "notifications", description = "Event bus and push notifications"),
        (name = "operators", description = "Operator registration and node management"),
        (name = "verification", description = "Distributed prime verification queue"),
        (name = "auth", description = "User authentication and profile management"),
    ),
    modifiers(&SecurityAddon),
)]
pub(super) struct ApiDoc;

/// Build the OpenAPI JSON document as a string.
pub(super) fn openapi_json() -> String {
    let mut doc = ApiDoc::openapi();

    // Add servers that may be configured at runtime
    if let Ok(url) = std::env::var("API_PUBLIC_URL") {
        let mut configured = Server::new(url);
        configured.description = Some("Configured".to_string());
        let mut local = Server::new("http://localhost:7001");
        local.description = Some("Local".to_string());
        doc.servers = Some(vec![configured, local]);
    }

    doc.to_pretty_json().unwrap_or_else(|_| "{}".to_string())
}
