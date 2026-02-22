//! Prime verification endpoint — triggers manual re-verification via GMP.

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;

use super::middleware_auth::RequireAdmin;
use super::AppState;
use crate::verify;

#[utoipa::path(post, path = "/api/primes/{id}/verify", tag = "verification", security(("bearer_jwt" = [])),
    params(("id" = i64, Path, description = "Prime ID to verify")),
    responses((status = 200, description = "Verification result"), (status = 401, description = "Authentication required"), (status = 404, description = "Prime not found"), (status = 500, description = "Internal server error"))
)]
pub(super) async fn handler_api_prime_verify(
    _admin: RequireAdmin,
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    let prime = match state.db.get_prime_by_id(id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Prime not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let prime_clone = prime.clone();
    let result = match tokio::task::spawn_blocking(move || verify::verify_prime(&prime_clone)).await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Verification panicked: {}", e)})),
            )
                .into_response()
        }
    };
    match result {
        verify::VerifyResult::Verified { ref method, tier } => {
            if let Err(e) = state.db.mark_verified(id, method, tier as i16).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
            let new_tags = verify::classify_after_verification(&prime, &result);
            if !new_tags.is_empty() {
                let tag_refs: Vec<&str> = new_tags.iter().map(|s| s.as_str()).collect();
                let _ = state.db.add_prime_tags(id, &tag_refs).await;
            }
            Json(serde_json::json!({"ok": true, "result": "verified", "method": method, "tier": tier})).into_response()
        }
        verify::VerifyResult::Failed { reason } => {
            let _ = state.db.mark_verification_failed(id, &reason).await;
            Json(serde_json::json!({"ok": true, "result": "failed", "reason": reason}))
                .into_response()
        }
        verify::VerifyResult::Skipped { reason } => {
            Json(serde_json::json!({"ok": true, "result": "skipped", "reason": reason}))
                .into_response()
        }
    }
}
