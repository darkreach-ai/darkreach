//! # Sieve Cache API — Worker-Coordinator Sieve Exchange
//!
//! Three endpoints for the shared sieve blob store:
//! - PUT `/api/v1/sieve/{hash}` — worker uploads a computed sieve blob
//! - GET `/api/v1/sieve/{hash}` — worker downloads a cached sieve blob
//! - GET `/api/v1/sieves` — list cached sieves for a form (dashboard)
//!
//! Blobs are hash-addressed (SHA-256 of sieve parameters) and immutable.
//! Size limit: 50 MB per blob.

use axum::body::Bytes;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use super::AppState;

/// Maximum sieve blob size: 50 MB.
const MAX_SIEVE_BLOB_SIZE: usize = 50 * 1024 * 1024;

#[derive(Deserialize)]
pub(super) struct SieveUploadParams {
    form: String,
    k: u64,
    base: u32,
    min_n: u64,
    max_n: u64,
    sieve_limit: u64,
    uploaded_by: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct SieveListParams {
    form: Option<String>,
    limit: Option<i64>,
}

/// PUT `/api/v1/sieve/{hash}` — upload a computed sieve blob.
pub(super) async fn handler_v1_sieve_upload(
    State(state): State<Arc<AppState>>,
    AxumPath(hash): AxumPath<String>,
    Query(params): Query<SieveUploadParams>,
    body: Bytes,
) -> impl IntoResponse {
    if body.len() > MAX_SIEVE_BLOB_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Sieve blob exceeds 50 MB limit"})),
        )
            .into_response();
    }

    // Verify the hash matches the blob content to prevent cache poisoning.
    // The hash in the URL path is the expected SHA-256 of the blob.
    let computed_hash = format!("{:x}", Sha256::digest(&body));
    if computed_hash != hash {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Hash mismatch: computed hash does not match URL hash",
                "expected": hash,
                "computed": computed_hash,
            })),
        )
            .into_response();
    }

    match state
        .db
        .insert_shared_sieve(
            &hash,
            &params.form,
            params.k,
            params.base,
            params.min_n,
            params.max_n,
            params.sieve_limit,
            &body,
            params.uploaded_by.as_deref(),
        )
        .await
    {
        Ok(()) => Json(serde_json::json!({"ok": true, "hash": hash})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET `/api/v1/sieve/{hash}` — download a cached sieve blob.
pub(super) async fn handler_v1_sieve_download(
    State(state): State<Arc<AppState>>,
    AxumPath(hash): AxumPath<String>,
) -> impl IntoResponse {
    match state.db.get_shared_sieve(&hash).await {
        Ok(Some(blob)) => {
            // Increment hit count (best-effort)
            let _ = state.db.increment_sieve_hit_count(&hash).await;

            let mut headers = HeaderMap::new();
            headers.insert("content-type", "application/octet-stream".parse().unwrap());
            (StatusCode::OK, headers, blob).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST `/api/v1/sieve/{hash}/relay` — relay announces it has cached this sieve.
/// Called by relay nodes after they download and cache a sieve locally.
pub(super) async fn handler_v1_sieve_relay_announce(
    State(state): State<Arc<AppState>>,
    AxumPath(hash): AxumPath<String>,
    Json(payload): Json<RelayAnnouncePayload>,
) -> impl IntoResponse {
    match state
        .db
        .register_relay_sieve_cache(&payload.relay_worker_id, &hash)
        .await
    {
        Ok(()) => Json(serde_json::json!({"ok": true, "hash": hash})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub(super) struct RelayAnnouncePayload {
    relay_worker_id: String,
}

/// GET `/api/v1/sieves` — list cached sieves for a form.
pub(super) async fn handler_v1_sieves_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SieveListParams>,
) -> impl IntoResponse {
    let form = params.form.as_deref().unwrap_or("kbn");
    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    match state.db.list_shared_sieves(form, limit).await {
        Ok(rows) => Json(serde_json::json!({"sieves": rows})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
