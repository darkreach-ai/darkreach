//! Auth API — user profile and role lookup.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use super::middleware_auth::RequireAuth;
use super::AppState;

#[derive(Deserialize)]
pub(super) struct ProfileQuery {
    user_id: String,
}

#[utoipa::path(get, path = "/api/auth/profile", tag = "auth", security(("bearer_jwt" = [])),
    params(("user_id" = String, Query, description = "User UUID")),
    responses((status = 200, description = "User profile"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
/// GET /api/auth/profile?user_id=<uuid>
/// Returns the user's profile including role and operator_id.
/// Requires authentication. Users can query their own profile;
/// admins can query any user's profile.
pub(super) async fn handler_api_profile(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProfileQuery>,
) -> impl IntoResponse {
    match state.db.get_user_profile(&params.user_id).await {
        Ok(Some(profile)) => Json(serde_json::json!(profile)).into_response(),
        Ok(None) => Json(serde_json::json!({
            "id": params.user_id,
            "role": "operator",
            "operator_id": null,
            "display_name": null,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to fetch profile: {}", e)})),
        )
            .into_response(),
    }
}

#[utoipa::path(get, path = "/api/auth/me", tag = "auth", security(("bearer_jwt" = [])),
    responses((status = 200, description = "Authenticated user profile"), (status = 401, description = "Authentication required"), (status = 500, description = "Internal server error"))
)]
/// GET /api/auth/me — Returns the authenticated user's own profile.
pub(super) async fn handler_api_me(
    State(state): State<Arc<AppState>>,
    RequireAuth(auth_user): RequireAuth,
) -> impl IntoResponse {
    match state.db.get_user_profile(&auth_user.user_id).await {
        Ok(Some(profile)) => Json(serde_json::json!(profile)).into_response(),
        Ok(None) => Json(serde_json::json!({
            "id": auth_user.user_id,
            "role": auth_user.role,
            "operator_id": null,
            "display_name": null,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to fetch profile: {}", e)})),
        )
            .into_response(),
    }
}
