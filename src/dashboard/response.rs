//! Standardized API response envelope and validated JSON extractor.
//!
//! ## Response envelope
//!
//! All REST API responses use a consistent JSON envelope:
//!
//! **Success:** `{ "data": <T> }`
//! **Error:**   `{ "error": "<message>" }`
//!
//! The `ApiOk` and `ApiErr` helpers produce these envelopes with correct
//! HTTP status codes.
//!
//! ## Validated JSON extractor
//!
//! `ValidatedJson<T>` replaces `axum::Json<T>` for request bodies that
//! derive `garde::Validate`. It deserializes and validates in one step,
//! returning 400 with field-level error details on validation failure.

use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::sync::Arc;

use super::AppState;

// ── Response envelope helpers ────────────────────────────────────────

/// Success envelope: `{ "data": <T> }` with the given status code.
pub(crate) fn api_ok<T: serde::Serialize>(status: StatusCode, data: T) -> Response {
    (status, Json(serde_json::json!({ "data": data }))).into_response()
}

/// Error envelope: `{ "error": "<message>" }` with the given status code.
pub(crate) fn api_err(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

// ── Validated JSON extractor ─────────────────────────────────────────

/// Axum extractor that deserializes JSON and runs `garde::Validate`.
///
/// On success, yields `ValidatedJson(T)`. On failure, returns 400 with
/// a structured error listing every validation violation.
///
/// # Usage
///
/// ```rust,ignore
/// #[derive(Deserialize, garde::Validate)]
/// struct Payload {
///     #[garde(length(min = 1, max = 200))]
///     name: String,
/// }
///
/// async fn handler(ValidatedJson(payload): ValidatedJson<Payload>) { ... }
/// ```
pub(super) struct ValidatedJson<T>(pub T);

impl<T> FromRequest<Arc<AppState>> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + garde::Validate<Context = ()>,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid request body: {}", e),
                })),
            )
                .into_response()
        })?;

        if let Err(report) = value.validate() {
            let errors: Vec<String> = report
                .iter()
                .map(|(path, error)| format!("{}: {}", path, error))
                .collect();
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Validation failed",
                    "details": errors,
                })),
            )
                .into_response());
        }

        Ok(ValidatedJson(value))
    }
}
