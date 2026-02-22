//! Status, export, and index handlers.

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::websocket;
use super::AppState;
use crate::{checkpoint, db};

pub(super) async fn handler_index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../dashboard.html"),
    )
}

#[derive(Serialize)]
pub(super) struct StatusResponse {
    pub active: bool,
    pub checkpoint: Option<serde_json::Value>,
}

pub(super) async fn handler_api_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let cp = checkpoint::load(&state.checkpoint_path);
    let has_running_jobs = state
        .db
        .get_search_jobs()
        .await
        .unwrap_or_default()
        .iter()
        .any(|j| j.status == "running");
    let has_workers = !state.get_workers_from_pg().await.is_empty();
    Json(StatusResponse {
        active: cp.is_some() || has_running_jobs || has_workers,
        checkpoint: cp.and_then(|c| serde_json::to_value(&c).ok()),
    })
}

#[derive(Deserialize)]
pub(super) struct ExportQuery {
    format: Option<String>,
    form: Option<String>,
    search: Option<String>,
    min_digits: Option<i64>,
    max_digits: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    tags: Option<String>,
}

pub(super) async fn handler_api_export(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ExportQuery>,
) -> impl IntoResponse {
    let tags = params
        .tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
    let filter = db::PrimeFilter {
        form: params.form.clone(),
        search: params.search,
        min_digits: params.min_digits,
        max_digits: params.max_digits,
        sort_by: params.sort_by,
        sort_dir: params.sort_dir,
        tags,
    };
    let format = params.format.unwrap_or_else(|| "csv".to_string());
    let primes = match state.db.get_primes_filtered(100_000, 0, &filter).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    let date = chrono::Utc::now().format("%Y%m%d");
    let form_suffix = params
        .form
        .as_deref()
        .map(|f| format!("-{f}"))
        .unwrap_or_default();
    if format == "json" {
        let body = serde_json::to_string_pretty(&primes).unwrap_or_default();
        let filename =
            format!("attachment; filename=\"darkreach-primes{form_suffix}-{date}.json\"");
        (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CONTENT_DISPOSITION, &*filename),
            ],
            body,
        )
            .into_response()
    } else {
        let mut csv = String::from("id,form,expression,digits,found_at,proof_method,tags\n");
        for p in &primes {
            csv.push_str(&format!(
                "{},\"{}\",\"{}\",{},{},\"{}\",\"{}\"\n",
                p.id,
                p.form.replace('"', "\"\""),
                p.expression.replace('"', "\"\""),
                p.digits,
                p.found_at.to_rfc3339(),
                p.proof_method.replace('"', "\"\""),
                p.tags.join(";")
            ));
        }
        let filename = format!("attachment; filename=\"darkreach-primes{form_suffix}-{date}.csv\"");
        (
            [
                (header::CONTENT_TYPE, "text/csv"),
                (header::CONTENT_DISPOSITION, &*filename),
            ],
            csv,
        )
            .into_response()
    }
}

/// Returns the same JSON snapshot that the WebSocket pushes every 2 seconds.
/// Used by the Vercel frontend (polling mode) since Vercel rewrites cannot proxy WebSocket.
pub(super) async fn handler_api_ws_snapshot(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match websocket::build_update(&state).await {
        Some(json) => ([(header::CONTENT_TYPE, "application/json")], json).into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to build snapshot",
        )
            .into_response(),
    }
}
