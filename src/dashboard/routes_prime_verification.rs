//! Distributed prime verification queue API endpoints.
//!
//! Exposes the prime-level verification queue to network nodes and the
//! dashboard. Nodes claim tasks, run `verify_prime()`, and submit results.
//! The dashboard can view queue stats and per-prime verification history.

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use super::AppState;

/// Request body for claiming a verification task.
#[derive(Deserialize)]
pub(super) struct ClaimRequest {
    worker_id: String,
}

/// Request body for submitting a verification result.
#[derive(Deserialize)]
pub(super) struct SubmitRequest {
    worker_id: String,
    tier: i16,
    method: String,
    result: Option<serde_json::Value>,
    success: bool,
    error_reason: Option<String>,
}

/// `GET /api/prime-verification/stats` — queue depth, completion rate.
pub(super) async fn handler_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_prime_verification_stats().await {
        Ok(stats) => Json(serde_json::json!({"ok": true, "stats": stats})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// `POST /api/prime-verification/claim` — claim next verification task.
pub(super) async fn handler_claim(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ClaimRequest>,
) -> impl IntoResponse {
    match state.db.claim_prime_verification(&body.worker_id).await {
        Ok(Some(task)) => Json(serde_json::json!({"ok": true, "task": task})).into_response(),
        Ok(None) => Json(serde_json::json!({"ok": true, "task": null})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// `POST /api/prime-verification/{id}/submit` — submit verification result.
pub(super) async fn handler_submit(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
    Json(body): Json<SubmitRequest>,
) -> impl IntoResponse {
    match state
        .db
        .submit_prime_verification(
            id,
            &body.worker_id,
            body.tier,
            &body.method,
            body.result.as_ref(),
            body.success,
            body.error_reason.as_deref(),
        )
        .await
    {
        Ok(quorum_met) => {
            Json(serde_json::json!({"ok": true, "quorum_met": quorum_met})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// `GET /api/primes/{id}/verifications` — verification history for a prime.
pub(super) async fn handler_prime_verifications(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    match state.db.get_prime_verification_results(id).await {
        Ok(results) => Json(serde_json::json!({"ok": true, "results": results})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// `POST /api/prime-verification/reclaim` — trigger stale recovery (admin).
pub(super) async fn handler_reclaim(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.reclaim_stale_prime_verifications(600).await {
        Ok(count) => Json(serde_json::json!({"ok": true, "reclaimed": count})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
