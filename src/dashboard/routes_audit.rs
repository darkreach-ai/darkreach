//! Audit log query endpoint (admin-only).
//!
//! Provides a read-only API for reviewing audit log entries with
//! filtering by user, action type, and time range. Used by the
//! dashboard audit page for security monitoring and compliance.

use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use std::sync::Arc;

use super::middleware_auth::RequireAdmin;
use super::response::{api_err, api_ok};
use super::AppState;
use axum::http::StatusCode;

/// Query parameters for the GET /api/audit endpoint.
///
/// All filters are optional. The endpoint returns paginated results
/// ordered by most recent first.
#[derive(Deserialize)]
pub(super) struct AuditQuery {
    /// Maximum number of entries to return (default 100, max 1000).
    #[serde(default = "default_limit")]
    limit: i64,
    /// Number of entries to skip for pagination (default 0).
    #[serde(default)]
    offset: i64,
    /// Filter by user ID (exact match).
    #[serde(default)]
    user_id: Option<String>,
    /// Filter by action type (exact match, e.g. "search.create").
    #[serde(default)]
    action: Option<String>,
    /// Filter entries created after this ISO 8601 datetime.
    #[serde(default)]
    since: Option<String>,
}

fn default_limit() -> i64 {
    100
}

/// GET /api/audit — List audit log entries with optional filters.
///
/// Requires admin authentication. Returns paginated audit log entries
/// with total count for pagination support.
///
/// Response: `{ "data": { "entries": [...], "total": N } }`
#[utoipa::path(
    get,
    path = "/api/audit",
    tag = "audit",
    security(("bearer_jwt" = [])),
    params(
        ("limit" = Option<i64>, Query, description = "Max entries to return (default 100, max 1000)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
        ("user_id" = Option<String>, Query, description = "Filter by user ID"),
        ("action" = Option<String>, Query, description = "Filter by action type"),
        ("since" = Option<String>, Query, description = "Filter entries after this ISO 8601 datetime"),
    ),
    responses(
        (status = 200, description = "Paginated audit log entries"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error"),
    )
)]
pub(super) async fn handler_audit_list(
    _admin: RequireAdmin,
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditQuery>,
) -> Response {
    let limit = query.limit.clamp(1, 1000);
    let offset = query.offset.max(0);

    // Parse the optional `since` parameter as an ISO 8601 datetime.
    let since = match query.since.as_deref() {
        Some(s) if !s.is_empty() => match s.parse::<chrono::DateTime<chrono::Utc>>() {
            Ok(dt) => Some(dt),
            Err(_) => {
                return api_err(
                    StatusCode::BAD_REQUEST,
                    "Invalid 'since' parameter: expected ISO 8601 datetime",
                );
            }
        },
        _ => None,
    };

    let user_id_ref = query.user_id.as_deref();
    let action_ref = query.action.as_deref();

    // Fetch entries and total count in parallel for efficiency.
    let (entries_result, count_result) = tokio::join!(
        state
            .db
            .get_audit_log(limit, offset, user_id_ref, action_ref, since),
        state
            .db
            .count_audit_log(user_id_ref, action_ref, since),
    );

    let entries = match entries_result {
        Ok(v) => v,
        Err(e) => {
            return api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to query audit log: {}", e),
            );
        }
    };

    let total = match count_result {
        Ok(v) => v,
        Err(e) => {
            return api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to count audit log: {}", e),
            );
        }
    };

    api_ok(
        StatusCode::OK,
        serde_json::json!({
            "entries": entries,
            "total": total,
        }),
    )
}
