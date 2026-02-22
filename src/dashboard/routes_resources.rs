//! Resource API — fleet resource summary and credit conversion rates.
//!
//! Provides endpoints for viewing aggregate fleet resource capacity
//! and the current credit conversion rates per resource type.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use std::sync::Arc;

use super::response::{api_err, api_ok};
use super::AppState;

/// `GET /api/resources/summary` — fleet-wide resource capacity and totals.
pub(super) async fn handler_resources_summary(
    State(state): State<Arc<AppState>>,
) -> Response {
    let snapshot = match state.db.get_fleet_resource_snapshot().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to get fleet resource snapshot");
            return api_err(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
        }
    };
    api_ok(StatusCode::OK, snapshot)
}

/// `GET /api/resources/rates` — credit conversion rates per resource type.
pub(super) async fn handler_resources_rates(
    State(state): State<Arc<AppState>>,
) -> Response {
    match state.db.get_resource_credit_rates().await {
        Ok(rates) => api_ok(StatusCode::OK, rates),
        Err(e) => {
            tracing::error!(error = %e, "failed to get resource credit rates");
            api_err(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}
