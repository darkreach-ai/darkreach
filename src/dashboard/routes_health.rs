//! # Health & Observability Endpoints
//!
//! Kubernetes-compatible health, readiness, and Prometheus metrics endpoints.
//!
//! | Endpoint | Purpose | K8s Probe |
//! |----------|---------|-----------|
//! | `GET /healthz` | Liveness — process is alive | `livenessProbe` |
//! | `GET /readyz` | Readiness — database connected, accepting traffic | `readinessProbe` |
//! | `GET /metrics` | Prometheus scraping endpoint | `ServiceMonitor` |
//!
//! The readiness probe performs a `SELECT 1` with a 2-second timeout. If the database
//! is unreachable, the coordinator returns 503 so the load balancer stops routing
//! traffic to it until connectivity is restored.

use super::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

#[utoipa::path(get, path = "/healthz", tag = "health",
    responses((status = 200, description = "Process is alive"))
)]
/// Liveness probe: returns 200 if the process is running.
///
/// K8s uses this to determine if the container needs to be restarted.
/// No dependencies checked — if the binary is serving HTTP, it's alive.
pub async fn handler_healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[utoipa::path(get, path = "/readyz", tag = "health",
    responses((status = 200, description = "Coordinator ready"), (status = 503, description = "Database unreachable"))
)]
/// Readiness probe: returns 200 if the coordinator can serve requests.
///
/// Checks database connectivity (primary + read replica + Redis) with a
/// 2-second timeout. Returns 503 Service Unavailable if any critical
/// component is unreachable.
pub async fn handler_readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let timeout = std::time::Duration::from_secs(2);

    // Check primary pool
    let primary_check = tokio::time::timeout(timeout, state.db.health_check()).await;
    match primary_check {
        Ok(Ok(())) => {}
        Ok(Err(_)) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "primary database unreachable",
            )
        }
        Err(_) => return (StatusCode::SERVICE_UNAVAILABLE, "primary database timeout"),
    }

    // Check read replica pool (may be same as primary if no replica configured)
    let read_check = tokio::time::timeout(timeout, async {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(state.db.read_pool())
            .await
    })
    .await;
    match read_check {
        Ok(Ok(_)) => {}
        Ok(Err(_)) => return (StatusCode::SERVICE_UNAVAILABLE, "read replica unreachable"),
        Err(_) => return (StatusCode::SERVICE_UNAVAILABLE, "read replica timeout"),
    }

    // Check Redis (non-critical — degrade gracefully)
    if let Some(redis) = state.db.redis() {
        let mut conn = redis.clone();
        let redis_check = tokio::time::timeout(timeout, async {
            redis::cmd("PING").query_async::<String>(&mut conn).await
        })
        .await;
        match redis_check {
            Ok(Ok(_)) => {}
            _ => {
                // Redis is optional — warn but don't fail readiness
                tracing::warn!("readyz: Redis health check failed (degraded mode)");
            }
        }
    }

    (StatusCode::OK, "ok")
}

/// Individual check result for the deep health endpoint.
#[derive(Serialize)]
struct CheckResult {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    free_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    age_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    corruption_count: Option<u64>,
}

/// Deep health check response with individual component statuses.
#[derive(Serialize)]
struct DeepHealthResponse {
    status: &'static str,
    checks: DeepHealthChecks,
}

#[derive(Serialize)]
struct DeepHealthChecks {
    database: CheckResult,
    disk: CheckResult,
    memory: CheckResult,
    checkpoint: CheckResult,
}

#[utoipa::path(get, path = "/healthz/deep", tag = "health",
    responses((status = 200, description = "Deep health check with component statuses"))
)]
/// Deep health probe: checks database, disk space, memory, and checkpoint freshness.
///
/// Returns a JSON object with per-component status (`ok`, `degraded`, `unhealthy`).
/// The top-level `status` is `ok` if all checks pass, `degraded` if any is degraded,
/// or `unhealthy` if any critical check fails.
pub async fn handler_healthz_deep(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut overall_degraded = false;
    let mut overall_unhealthy = false;

    // 0. Checkpoint recovery stats (non-blocking, always ok)
    let recovery = crate::checkpoint::recovery_count();
    let corruption = crate::checkpoint::corruption_count();

    // 1. Database check — SELECT 1 with 2s timeout
    let db_start = std::time::Instant::now();
    let db_check =
        tokio::time::timeout(std::time::Duration::from_secs(2), state.db.health_check()).await;
    let database = match db_check {
        Ok(Ok(())) => CheckResult {
            status: "ok",
            latency_ms: Some(db_start.elapsed().as_millis() as u64),
            free_gb: None,
            usage_percent: None,
            age_seconds: None,
            recovery_count: None,
            corruption_count: None,
        },
        _ => {
            overall_unhealthy = true;
            CheckResult {
                status: "unhealthy",
                latency_ms: None,
                free_gb: None,
                usage_percent: None,
                age_seconds: None,
                recovery_count: None,
                corruption_count: None,
            }
        }
    };

    // 2. Disk space check — at least 1 GB free on /
    let disk = match crate::metrics::disk_free_bytes("/") {
        Some(free_bytes) => {
            let free_gb = free_bytes as f64 / 1_073_741_824.0;
            if free_gb < 1.0 {
                overall_unhealthy = true;
                CheckResult {
                    status: "unhealthy",
                    latency_ms: None,
                    free_gb: Some((free_gb * 10.0).round() / 10.0),
                    usage_percent: None,
                    age_seconds: None,
                    recovery_count: None,
                    corruption_count: None,
                }
            } else if free_gb < 5.0 {
                overall_degraded = true;
                CheckResult {
                    status: "degraded",
                    latency_ms: None,
                    free_gb: Some((free_gb * 10.0).round() / 10.0),
                    usage_percent: None,
                    age_seconds: None,
                    recovery_count: None,
                    corruption_count: None,
                }
            } else {
                CheckResult {
                    status: "ok",
                    latency_ms: None,
                    free_gb: Some((free_gb * 10.0).round() / 10.0),
                    usage_percent: None,
                    age_seconds: None,
                    recovery_count: None,
                    corruption_count: None,
                }
            }
        }
        None => {
            overall_degraded = true;
            CheckResult {
                status: "degraded",
                latency_ms: None,
                free_gb: None,
                usage_percent: None,
                age_seconds: None,
                recovery_count: None,
                corruption_count: None,
            }
        }
    };

    // 3. Memory check — under 90% usage
    let mem_percent = crate::metrics::memory_usage_percent();
    let memory = if mem_percent > 90.0 {
        overall_unhealthy = true;
        CheckResult {
            status: "unhealthy",
            latency_ms: None,
            free_gb: None,
            usage_percent: Some((mem_percent * 10.0).round() / 10.0),
            age_seconds: None,
            recovery_count: None,
            corruption_count: None,
        }
    } else if mem_percent > 80.0 {
        overall_degraded = true;
        CheckResult {
            status: "degraded",
            latency_ms: None,
            free_gb: None,
            usage_percent: Some((mem_percent * 10.0).round() / 10.0),
            age_seconds: None,
            recovery_count: None,
            corruption_count: None,
        }
    } else {
        CheckResult {
            status: "ok",
            latency_ms: None,
            free_gb: None,
            usage_percent: Some((mem_percent * 10.0).round() / 10.0),
            age_seconds: None,
            recovery_count: None,
            corruption_count: None,
        }
    };

    // 4. Checkpoint freshness — file modified within last 5 minutes (if it exists)
    let checkpoint = match std::fs::metadata(&state.checkpoint_path) {
        Ok(meta) => {
            let age_secs = meta
                .modified()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if age_secs > 300 {
                overall_degraded = true;
                CheckResult {
                    status: "degraded",
                    latency_ms: None,
                    free_gb: None,
                    usage_percent: None,
                    age_seconds: Some(age_secs),
                    recovery_count: Some(recovery),
                    corruption_count: Some(corruption),
                }
            } else {
                CheckResult {
                    status: "ok",
                    latency_ms: None,
                    free_gb: None,
                    usage_percent: None,
                    age_seconds: Some(age_secs),
                    recovery_count: Some(recovery),
                    corruption_count: Some(corruption),
                }
            }
        }
        Err(_) => {
            // No checkpoint file — not necessarily a problem (no active search)
            CheckResult {
                status: "ok",
                latency_ms: None,
                free_gb: None,
                usage_percent: None,
                age_seconds: None,
                recovery_count: Some(recovery),
                corruption_count: Some(corruption),
            }
        }
    };

    let status = if overall_unhealthy {
        "unhealthy"
    } else if overall_degraded {
        "degraded"
    } else {
        "ok"
    };

    let code = if overall_unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        code,
        Json(DeepHealthResponse {
            status,
            checks: DeepHealthChecks {
                database,
                disk,
                memory,
                checkpoint,
            },
        }),
    )
}

/// Prometheus metrics endpoint: returns all metrics in text exposition format.
///
/// Scraped by Prometheus every 15-30 seconds (configurable via ServiceMonitor).
/// Metrics are updated in the dashboard's 30-second background loop.
pub async fn handler_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let body = state.prom_metrics.encode();
    (
        StatusCode::OK,
        [(
            "content-type",
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )],
        body,
    )
}
