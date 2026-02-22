//! Agent management API — tasks, events, budgets, memory, roles, templates.

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use super::response::ValidatedJson;
use super::{lock_or_recover, AppState};

#[derive(Deserialize)]
pub(super) struct AgentTasksQuery {
    status: Option<String>,
    limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/agents/tasks",
    tag = "agents",
    security(("bearer_jwt" = [])),
    params(
        ("status" = Option<String>, Query, description = "Filter by task status"),
        ("limit" = Option<i64>, Query, description = "Max results (default 100, max 1000)"),
    ),
    responses(
        (status = 200, description = "List of agent tasks"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error"),
    )
)]
pub(super) async fn handler_api_agent_tasks(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AgentTasksQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100);
    match state
        .db
        .get_agent_tasks(params.status.as_deref(), limit)
        .await
    {
        Ok(tasks) => Json(serde_json::json!({ "tasks": tasks })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(super) struct CreateAgentTaskPayload {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_priority")]
    priority: String,
    agent_model: Option<String>,
    #[serde(default = "default_source")]
    source: String,
    max_cost_usd: Option<f64>,
    #[serde(default = "default_permission_level")]
    permission_level: i32,
    role_name: Option<String>,
}

fn default_permission_level() -> i32 {
    1
}

fn default_priority() -> String {
    "normal".to_string()
}

fn default_source() -> String {
    "manual".to_string()
}

#[utoipa::path(
    post,
    path = "/api/agents/tasks",
    tag = "agents",
    security(("bearer_jwt" = [])),
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "Agent task created"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error"),
    )
)]
pub(super) async fn handler_api_agent_task_create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateAgentTaskPayload>,
) -> impl IntoResponse {
    // If a role is specified, look it up and apply defaults for unset fields
    let mut agent_model = payload.agent_model.clone();
    let mut max_cost_usd = payload.max_cost_usd;
    let mut permission_level = payload.permission_level;

    if let Some(ref role_name) = payload.role_name {
        if let Ok(Some(role)) = state.db.get_role_by_name(role_name).await {
            // Apply role defaults only when the payload uses the default values
            if agent_model.is_none() {
                agent_model = Some(role.default_model.clone());
            }
            if max_cost_usd.is_none() {
                max_cost_usd = role.default_max_cost_usd;
            }
            if permission_level == 1 {
                // 1 is the default — override with role default
                permission_level = role.default_permission_level;
            }
        }
    }

    match state
        .db
        .create_agent_task(
            &payload.title,
            &payload.description,
            &payload.priority,
            agent_model.as_deref(),
            &payload.source,
            max_cost_usd,
            permission_level,
            payload.role_name.as_deref(),
        )
        .await
    {
        Ok(task) => {
            // Insert a "created" event
            let _ = state
                .db
                .insert_agent_event(
                    Some(task.id),
                    "created",
                    None,
                    &format!("Task created: {}", task.title),
                    None,
                )
                .await;
            (StatusCode::CREATED, Json(serde_json::json!(task))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(get, path = "/api/agents/tasks/{id}", tag = "agents", security(("bearer_jwt" = [])),
    params(("id" = i64, Path, description = "Agent task ID")),
    responses((status = 200, description = "Agent task details"), (status = 401, description = "Authentication required"), (status = 404, description = "Task not found"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_task_get(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    match state.db.get_agent_task(id).await {
        Ok(Some(task)) => Json(serde_json::json!(task)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(post, path = "/api/agents/tasks/{id}/cancel", tag = "agents", security(("bearer_jwt" = [])),
    params(("id" = i64, Path, description = "Agent task ID to cancel")),
    responses((status = 200, description = "Task cancelled"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_task_cancel(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    match state.db.cancel_agent_task(id).await {
        Ok(()) => {
            lock_or_recover(&state.agents).cancel_agent(id);
            let _ = state
                .db
                .insert_agent_event(Some(id), "cancelled", None, "Task cancelled", None)
                .await;
            Json(serde_json::json!({"ok": true, "id": id})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub(super) struct AgentEventsQuery {
    task_id: Option<i64>,
    limit: Option<i64>,
}

#[utoipa::path(get, path = "/api/agents/events", tag = "agents", security(("bearer_jwt" = [])),
    params(("task_id" = Option<i64>, Query, description = "Filter events by task ID"), ("limit" = Option<i64>, Query, description = "Max results (default 100, max 1000)")),
    responses((status = 200, description = "List of agent events"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AgentEventsQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100);
    match state.db.get_agent_events(params.task_id, limit).await {
        Ok(events) => Json(serde_json::json!({ "events": events })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(get, path = "/api/agents/budgets", tag = "agents", security(("bearer_jwt" = [])),
    responses((status = 200, description = "Agent budget information"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_budgets(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_agent_budgets().await {
        Ok(budgets) => Json(serde_json::json!({ "budgets": budgets })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(super) struct UpdateBudgetPayload {
    id: i64,
    budget_usd: f64,
}

#[utoipa::path(put, path = "/api/agents/budgets", tag = "agents", security(("bearer_jwt" = [])),
    request_body = serde_json::Value,
    responses((status = 200, description = "Budget updated"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_budget_update(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateBudgetPayload>,
) -> impl IntoResponse {
    match state
        .db
        .update_agent_budget(payload.id, payload.budget_usd)
        .await
    {
        Ok(()) => Json(serde_json::json!({"ok": true, "id": payload.id})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// --- Agent Memory API ---

#[utoipa::path(get, path = "/api/agents/memory", tag = "agents", security(("bearer_jwt" = [])),
    responses((status = 200, description = "List of agent memory entries"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_memory_list(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_all_agent_memory().await {
        Ok(entries) => Json(serde_json::json!({ "memories": entries })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize, garde::Validate)]
pub(super) struct UpsertMemoryPayload {
    #[garde(length(min = 1, max = 200))]
    key: String,
    #[garde(length(min = 1, max = 50_000))]
    value: String,
    #[serde(default = "default_memory_category")]
    #[garde(length(min = 1, max = 50))]
    category: String,
}

fn default_memory_category() -> String {
    "general".to_string()
}

#[utoipa::path(post, path = "/api/agents/memory", tag = "agents", security(("bearer_jwt" = [])),
    request_body = serde_json::Value,
    responses((status = 200, description = "Memory entry upserted"), (status = 400, description = "Invalid key"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_memory_upsert(
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<UpsertMemoryPayload>,
) -> impl IntoResponse {
    match state
        .db
        .upsert_agent_memory(&payload.key, &payload.value, &payload.category, None)
        .await
    {
        Ok(entry) => (StatusCode::OK, Json(serde_json::json!(entry))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(delete, path = "/api/agents/memory/{key}", tag = "agents", security(("bearer_jwt" = [])),
    params(("key" = String, Path, description = "Memory entry key")),
    responses((status = 200, description = "Memory entry deleted"), (status = 401, description = "Authentication required"), (status = 404, description = "Memory entry not found"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_agent_memory_delete(
    State(state): State<Arc<AppState>>,
    AxumPath(key): AxumPath<String>,
) -> impl IntoResponse {
    match state.db.delete_agent_memory(&key).await {
        Ok(true) => Json(serde_json::json!({"ok": true, "key": key})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Memory entry not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// --- Agent role endpoints ---

#[utoipa::path(get, path = "/api/agents/roles", tag = "agents", security(("bearer_jwt" = [])),
    responses((status = 200, description = "List of agent roles"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
/// GET /api/agents/roles — List all agent roles.
pub(super) async fn handler_api_agent_roles(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_all_roles().await {
        Ok(roles) => Json(serde_json::json!(roles)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(get, path = "/api/agents/roles/{name}", tag = "agents", security(("bearer_jwt" = [])),
    params(("name" = String, Path, description = "Role name")),
    responses((status = 200, description = "Role details"), (status = 401, description = "Authentication required"), (status = 404, description = "Role not found"), (status = 500, description = "Internal server error"))
)]
/// GET /api/agents/roles/{name} — Get a single role by name.
pub(super) async fn handler_api_agent_role_get(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    match state.db.get_role_by_name(&name).await {
        Ok(Some(role)) => Json(serde_json::json!(role)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Role '{}' not found", name)})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(get, path = "/api/agents/roles/{name}/templates", tag = "agents", security(("bearer_jwt" = [])),
    params(("name" = String, Path, description = "Role name")),
    responses((status = 200, description = "Templates for the role"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
/// GET /api/agents/roles/{name}/templates — Get templates associated with a role.
pub(super) async fn handler_api_agent_role_templates(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    match state.db.get_role_templates(&name).await {
        Ok(templates) => Json(serde_json::json!(templates)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// --- Agent template & decomposition endpoints ---

#[utoipa::path(get, path = "/api/agents/templates", tag = "agents", security(("bearer_jwt" = [])),
    responses((status = 200, description = "List of workflow templates"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
/// GET /api/agents/templates — List all workflow templates.
pub(super) async fn handler_api_agent_templates(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_all_templates().await {
        Ok(templates) => Json(serde_json::json!(templates)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(post, path = "/api/agents/templates/{name}/expand", tag = "agents", security(("bearer_jwt" = [])),
    params(("name" = String, Path, description = "Template name")),
    responses((status = 200, description = "Template expanded into task tree"), (status = 400, description = "Template not found or invalid"), (status = 401, description = "Authentication required"))
)]
/// POST /api/agents/templates/{name}/expand — Expand a template into parent + child tasks.
pub(super) async fn handler_api_agent_template_expand(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let title = body
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Untitled");
    let description = body
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("");
    let priority = body
        .get("priority")
        .and_then(|p| p.as_str())
        .unwrap_or("normal");
    let max_cost_usd = body.get("max_cost_usd").and_then(|c| c.as_f64());
    let permission_level = body
        .get("permission_level")
        .and_then(|l| l.as_i64())
        .unwrap_or(1) as i32;
    let role_name = body.get("role_name").and_then(|r| r.as_str());

    match state
        .db
        .expand_template(
            &name,
            title,
            description,
            priority,
            max_cost_usd,
            permission_level,
            role_name,
        )
        .await
    {
        Ok(parent_id) => {
            let _ = state
                .db
                .insert_agent_event(
                    Some(parent_id),
                    "created",
                    None,
                    &format!("Template '{}' expanded into task tree", name),
                    None,
                )
                .await;
            Json(serde_json::json!({"ok": true, "parent_task_id": parent_id})).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[utoipa::path(get, path = "/api/agents/tasks/{id}/children", tag = "agents", security(("bearer_jwt" = [])),
    params(("id" = i64, Path, description = "Parent task ID")),
    responses((status = 200, description = "Child tasks of the parent"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
/// GET /api/agents/tasks/{id}/children — Get child tasks of a parent task.
pub(super) async fn handler_api_agent_task_children(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    match state.db.get_child_tasks(id).await {
        Ok(children) => Json(serde_json::json!(children)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
