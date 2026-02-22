//! Worker release management endpoints (operator control plane).
//!
//! Mutating endpoints (upsert, rollout, rollback) require admin authentication.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use super::middleware_auth::RequireAdmin;
use super::response::ValidatedJson;
use super::AppState;

#[derive(Deserialize)]
pub(super) struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[utoipa::path(get, path = "/api/releases/worker", tag = "releases", security(("bearer_jwt" = [])),
    params(("limit" = Option<i64>, Query, description = "Max releases to return (default 50, max 200)")),
    responses((status = 200, description = "List of releases and channels"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_releases_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let limit = query.limit.clamp(1, 200);
    let releases = match state.db.list_worker_releases(limit).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to list releases: {}", e)})),
            );
        }
    };
    let channels = match state.db.list_worker_release_channels().await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to list channels: {}", e)})),
            );
        }
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "releases": releases,
            "channels": channels,
        })),
    )
}

#[derive(Deserialize)]
pub(super) struct EventsQuery {
    #[serde(default)]
    channel: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

#[utoipa::path(get, path = "/api/releases/events", tag = "releases", security(("bearer_jwt" = [])),
    params(("channel" = Option<String>, Query, description = "Filter by channel"), ("limit" = Option<i64>, Query, description = "Max events to return (default 50, max 500)")),
    responses((status = 200, description = "List of release events"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_releases_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.clamp(1, 500);
    match state
        .db
        .list_worker_release_events(query.channel.as_deref(), limit)
        .await
    {
        Ok(events) => (
            StatusCode::OK,
            Json(serde_json::json!({ "events": events })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to list events: {}", e)})),
        ),
    }
}

#[derive(Deserialize)]
pub(super) struct HealthQuery {
    #[serde(default = "default_active_hours")]
    active_hours: i64,
}

fn default_active_hours() -> i64 {
    24
}

#[utoipa::path(get, path = "/api/releases/health", tag = "releases", security(("bearer_jwt" = [])),
    params(("active_hours" = Option<i64>, Query, description = "Window for active workers (default 24h)")),
    responses((status = 200, description = "Release health and adoption stats"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_releases_health(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HealthQuery>,
) -> impl IntoResponse {
    let active_hours = query.active_hours.clamp(1, 24 * 30);
    let adoption = match state.db.worker_release_adoption(active_hours).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to fetch adoption: {}", e)})),
            );
        }
    };
    let channels = match state.db.list_worker_release_channels().await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to list channels: {}", e)})),
            );
        }
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "active_hours": active_hours,
            "adoption": adoption,
            "channels": channels,
        })),
    )
}

#[derive(Deserialize, garde::Validate)]
pub(super) struct UpsertReleasePayload {
    #[garde(length(min = 1, max = 50))]
    version: String,
    #[garde(skip)]
    artifacts: serde_json::Value,
    #[garde(length(max = 5000))]
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    #[garde(skip)]
    published_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(post, path = "/api/releases/worker", tag = "releases", security(("bearer_jwt" = [])),
    request_body = serde_json::Value,
    responses((status = 200, description = "Release upserted"), (status = 400, description = "Invalid artifacts"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_releases_upsert(
    _admin: RequireAdmin,
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<UpsertReleasePayload>,
) -> impl IntoResponse {
    if !payload.artifacts.is_array() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "artifacts must be a JSON array"})),
        );
    }
    match state
        .db
        .upsert_worker_release(
            &payload.version,
            &payload.artifacts,
            payload.notes.as_deref(),
            payload.published_at,
        )
        .await
    {
        Ok(row) => (StatusCode::OK, Json(serde_json::json!({ "release": row }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to upsert release: {}", e)})),
        ),
    }
}

#[derive(Deserialize, garde::Validate)]
pub(super) struct RolloutPayload {
    #[garde(length(min = 1, max = 50))]
    channel: String,
    #[garde(length(min = 1, max = 50))]
    version: String,
    #[serde(default = "default_rollout")]
    #[garde(range(min = 0, max = 100))]
    rollout_percent: i32,
    #[serde(default)]
    #[garde(length(max = 200))]
    changed_by: Option<String>,
}

fn default_rollout() -> i32 {
    100
}

#[utoipa::path(post, path = "/api/releases/rollout", tag = "releases", security(("bearer_jwt" = [])),
    request_body = serde_json::Value,
    responses((status = 200, description = "Rollout configured"), (status = 400, description = "Invalid channel or version"), (status = 401, description = "Authentication required"))
)]
pub(super) async fn handler_releases_rollout(
    _admin: RequireAdmin,
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<RolloutPayload>,
) -> impl IntoResponse {
    match state
        .db
        .set_worker_release_channel(
            &payload.channel,
            &payload.version,
            payload.rollout_percent,
            payload.changed_by.as_deref(),
        )
        .await
    {
        Ok(row) => (StatusCode::OK, Json(serde_json::json!({ "channel": row }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

#[derive(Deserialize, garde::Validate)]
pub(super) struct RollbackPayload {
    #[garde(length(min = 1, max = 50))]
    channel: String,
    #[serde(default)]
    #[garde(length(max = 200))]
    changed_by: Option<String>,
}

#[utoipa::path(post, path = "/api/releases/rollback", tag = "releases", security(("bearer_jwt" = [])),
    request_body = serde_json::Value,
    responses((status = 200, description = "Rollback executed"), (status = 400, description = "Rollback failed"), (status = 401, description = "Authentication required"))
)]
pub(super) async fn handler_releases_rollback(
    _admin: RequireAdmin,
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<RollbackPayload>,
) -> impl IntoResponse {
    match state
        .db
        .rollback_worker_release_channel(&payload.channel, payload.changed_by.as_deref())
        .await
    {
        Ok(row) => (StatusCode::OK, Json(serde_json::json!({ "channel": row }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}
