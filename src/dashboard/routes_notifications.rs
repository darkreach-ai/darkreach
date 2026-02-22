//! Event bus and notification endpoints.

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;

use super::AppState;

#[utoipa::path(get, path = "/api/notifications", tag = "notifications", security(("bearer_jwt" = [])),
    responses((status = 200, description = "Recent notifications"))
)]
pub(super) async fn handler_api_notifications(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let notifications = state.event_bus.recent_notifications(50);
    Json(serde_json::json!({ "notifications": notifications }))
}

#[utoipa::path(get, path = "/api/events", tag = "notifications", security(("bearer_jwt" = [])),
    responses((status = 200, description = "Recent events"))
)]
pub(super) async fn handler_api_events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let events = state.event_bus.recent_events(200);
    Json(serde_json::json!({ "events": events }))
}
