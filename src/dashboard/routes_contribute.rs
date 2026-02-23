//! Browser Contribute API — endpoints for browser-based compute contribution.
//!
//! Authenticated operators can run prime searches directly in their browser tab.
//! The browser acts as a "virtual node": it claims work blocks via JWT-authed REST,
//! runs trial division + Miller-Rabin in a Web Worker, and submits results back.
//! Browser primes are tagged `"browser"` for downstream quorum verification.
//!
//! ## Endpoints
//!
//! | Path | Method | Purpose |
//! |------|--------|---------|
//! | `/api/v1/contribute/work` | GET | Claim a work block (browser caps) |
//! | `/api/v1/contribute/result` | POST | Submit completed block results |
//! | `/api/v1/contribute/heartbeat` | POST | Keep browser "node" alive |

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use super::middleware_auth::RequireAuth;
use super::routes_operator::get_operator_uuid;
use super::AppState;
use crate::db::operators::NodeCapabilities;

/// Per-form complexity multiplier for browser block sizing.
///
/// Different search forms have vastly different per-candidate cost. A block of
/// 1000 factorial candidates takes much longer than 1000 twin candidates.
/// Baseline: twin/sophie_germain/kbn = 1.0 (fastest sieve-heavy forms).
/// Lower multiplier = fewer candidates per block (more expensive forms).
fn form_block_multiplier(search_type: &str) -> f64 {
    match search_type {
        "twin" | "sophie_germain" => 1.0,
        "kbn" => 1.0,
        "palindromic" | "near_repdigit" => 0.8,
        "factorial" | "primorial" => 0.3,
        "cullen_woodall" | "carol_kynea" => 0.5,
        "wagstaff" | "gen_fermat" | "repunit" => 0.3,
        _ => 0.5,
    }
}

// ── GET /api/v1/contribute/work ───────────────────────────────────
//
// Claims a work block sized for browser execution. Hard-coded to 1 core,
// no RAM, no GPU, OS "browser" (or "browser-wasm" when ?engine=wasm).
// WASM engine supports all 12 search forms; JS fallback supports 9
// (excludes factorial, palindromic, near_repdigit which need WASM).

#[derive(Deserialize, Default)]
pub(super) struct ContributeWorkQuery {
    #[serde(default)]
    engine: Option<String>,
    /// Number of blocks to claim atomically (1-3). Browser batches are capped
    /// lower than operator batches since browser tabs are less reliable and all
    /// blocks carry a lease TTL.
    #[serde(default)]
    batch_size: Option<i32>,
}

pub(super) async fn handler_contribute_work(
    State(state): State<Arc<AppState>>,
    RequireAuth(auth_user): RequireAuth,
    Query(query): Query<ContributeWorkQuery>,
) -> impl IntoResponse {
    let operator_id = match get_operator_uuid(&state, &auth_user.user_id).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let is_wasm = query.engine.as_deref() == Some("wasm");
    let os_cap = if is_wasm { "browser-wasm" } else { "browser" };

    let caps = NodeCapabilities {
        cores: 1,
        ram_gb: 0,
        has_gpu: false,
        os: Some(os_cap.to_string()),
        arch: None,
        gpu_runtime: None,
        gpu_vram_gb: None,
    };

    // Let the scheduler pick from all available forms (no preference bias)
    let preferred: Vec<String> = vec![];
    // JS fallback can't handle factorial, palindromic, near_repdigit —
    // exclude them unless the browser is running the WASM engine.
    let excluded: Vec<String> = if !is_wasm {
        vec![
            "factorial".to_string(),
            "palindromic".to_string(),
            "near_repdigit".to_string(),
        ]
    } else {
        vec![]
    };

    // Browser batches capped at 3 (less reliable than native nodes)
    let batch_size = query.batch_size.unwrap_or(1).clamp(1, 3);

    // Batch path: claim multiple blocks with atomic lease setting
    if batch_size > 1 {
        let claim_result = state
            .db
            .claim_operator_blocks(
                operator_id,
                &caps,
                &preferred,
                &excluded,
                batch_size,
                Some(5), // 5-minute lease set atomically in PG
            )
            .await;

        return match claim_result {
            Ok(blocks) if blocks.is_empty() => {
                (StatusCode::NO_CONTENT, Json(serde_json::json!(null)))
            }
            Ok(blocks) => {
                // Set quorum on each block
                let trust = state.db.get_operator_trust(operator_id).await.ok().flatten();
                let trust_level = trust.map(|t| t.trust_level).unwrap_or(1);
                for block in &blocks {
                    if let Some(ref search_type) = block.search_type {
                        let quorum = crate::verify::required_quorum(trust_level, search_type);
                        let _ = state.db.set_block_quorum(block.block_id, quorum).await;
                    }
                }

                let block_json: Vec<serde_json::Value> = blocks
                    .iter()
                    .map(|b| {
                        let search_type_str = b.search_type.as_deref().unwrap_or("unknown");
                        let multiplier = form_block_multiplier(search_type_str);
                        let raw_size = (b.block_end - b.block_start) as f64;
                        let adjusted_size = (raw_size * multiplier).round() as i64;
                        serde_json::json!({
                            "block_id": b.block_id,
                            "search_job_id": b.search_job_id,
                            "search_type": b.search_type,
                            "params": b.params,
                            "block_start": b.block_start,
                            "block_end": b.block_end,
                            "block_size_multiplier": multiplier,
                            "adjusted_block_size": adjusted_size,
                        })
                    })
                    .collect();

                (
                    StatusCode::OK,
                    Json(serde_json::json!({ "blocks": block_json })),
                )
            }
            Err(e) => {
                tracing::warn!(error = %e, "browser contribute batch work claim failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Work claim failed"})),
                )
            }
        };
    }

    // Single-block path (backward compatible): flat JSON response
    match state
        .db
        .claim_operator_block_with_prefs(operator_id, &caps, &preferred, &excluded)
        .await
    {
        Ok(Some(block)) => {
            // Set quorum — browser results need extra verification
            if let Some(ref search_type) = block.search_type {
                let trust = state.db.get_operator_trust(operator_id).await.ok().flatten();
                let trust_level = trust.map(|t| t.trust_level).unwrap_or(1);
                let quorum = crate::verify::required_quorum(trust_level, search_type);
                let _ = state.db.set_block_quorum(block.block_id, quorum).await;
            }

            // Set 5-minute initial lease — must be renewed by heartbeat
            let _ = state.db.set_block_lease(block.block_id, 5).await;

            // Compute form-adjusted block size for the response
            let search_type_str = block.search_type.as_deref().unwrap_or("unknown");
            let multiplier = form_block_multiplier(search_type_str);
            let raw_size = (block.block_end - block.block_start) as f64;
            let adjusted_size = (raw_size * multiplier).round() as i64;

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "block_id": block.block_id,
                    "search_job_id": block.search_job_id,
                    "search_type": block.search_type,
                    "params": block.params,
                    "block_start": block.block_start,
                    "block_end": block.block_end,
                    "block_size_multiplier": multiplier,
                    "adjusted_block_size": adjusted_size,
                })),
            )
        }
        Ok(None) => (StatusCode::NO_CONTENT, Json(serde_json::json!(null))),
        Err(e) => {
            tracing::warn!(error = %e, "browser contribute work claim failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Work claim failed"})),
            )
        }
    }
}

// ── POST /api/v1/contribute/result ────────────────────────────────
//
// Accepts completed block results from the browser Web Worker. Browser
// primes are tagged `["browser"]` and use `proof_method: "miller_rabin_10_browser"`.
// Browser blocks earn 50% credit vs native workers.
//
// Includes server-side sanity checks, hash verification feedback,
// enriched response with credits/badges/trust, and idempotency protection.

#[derive(Deserialize)]
pub(super) struct ContributeResultPayload {
    block_id: i64,
    tested: i64,
    found: i64,
    #[serde(default)]
    primes: Vec<ContributePrimePayload>,
    /// SHA-256 hash of canonical result string for tamper detection.
    /// Computed by the WASM engine: `"{type}:{start}:{end}:{tested}:{primes_json}"`.
    #[serde(default)]
    result_hash: Option<String>,
    /// Client-reported block completion time in milliseconds.
    #[serde(default)]
    duration_ms: Option<i64>,
}

#[derive(Deserialize, serde::Serialize)]
pub(super) struct ContributePrimePayload {
    digits: u64,
    expression: String,
    form: String,
    proof_method: String,
}

pub(super) async fn handler_contribute_result(
    State(state): State<Arc<AppState>>,
    RequireAuth(auth_user): RequireAuth,
    Json(payload): Json<ContributeResultPayload>,
) -> axum::response::Response {
    let operator_id = match get_operator_uuid(&state, &auth_user.user_id).await {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    // Verify the block was claimed by this operator
    if let Ok(Some(claimer)) = state.db.get_block_claimer_operator(payload.block_id).await {
        if claimer != operator_id {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Block not claimed by this operator"})),
            )
                .into_response();
        }
    }

    // ── Step 5: Idempotency — check if block is already completed ──
    if let Ok(Some(status)) = state.db.get_block_status(payload.block_id).await {
        if status == "completed" {
            let hash_verified = state
                .db
                .is_block_hash_verified(payload.block_id)
                .await
                .unwrap_or(false);
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "already_completed",
                    "hash_verified": hash_verified,
                })),
            )
                .into_response();
        }
    }

    // ── Step 1: Server-side sanity checks ──

    // Get block range for validation
    let (block_start, block_end) = match state.db.get_block_search_info(payload.block_id).await {
        Ok(Some((_search_type, start, end))) => (start, end),
        _ => (0, i64::MAX), // fallback: skip range check
    };

    // 1. tested count must be within block range
    let block_size = (block_end - block_start).max(0);
    if payload.tested < 0 || payload.tested > block_size {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "tested count out of range"})),
        )
            .into_response();
    }

    // 2. found count must match primes array length
    if payload.found as usize != payload.primes.len() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "found count doesn't match primes array"})),
        )
            .into_response();
    }

    // 3. primes array bounded (no form produces >100 primes per 10K block)
    if payload.primes.len() > 200 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "too many primes reported"})),
        )
            .into_response();
    }

    // 4. each prime's digit count must be reasonable for the block range
    for prime in &payload.primes {
        if prime.digits < 1 || prime.digits > 1_000_000 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid prime digit count"})),
            )
                .into_response();
        }
    }

    // 5. duration_ms sanity: must be positive and < 24 hours
    if let Some(ms) = payload.duration_ms {
        if ms < 0 || ms > 86_400_000 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid duration"})),
            )
                .into_response();
        }
    }

    // ── Process the result ──

    // Complete the work block
    if let Err(e) = state
        .db
        .submit_operator_result(payload.block_id, payload.tested, payload.found)
        .await
    {
        tracing::warn!(error = %e, "browser contribute result submission failed");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Result submission failed"})),
        )
            .into_response();
    }

    // Track warnings and feedback for enriched response
    let mut warnings: Vec<String> = Vec::new();
    let mut hash_matched = false;

    // Store content-addressed result hash and verify against server recomputation
    if let Some(ref hash) = payload.result_hash {
        let _ = state
            .db
            .set_block_result_hash(payload.block_id, hash)
            .await;

        // Server-side hash verification: recompute the canonical hash and compare
        if let Ok(Some((search_type, bs, be))) =
            state.db.get_block_search_info(payload.block_id).await
        {
            let primes_json = serde_json::to_string(&payload.primes).unwrap_or_default();
            let canonical = format!(
                "{}:{}:{}:{}:{}",
                search_type, bs, be, payload.tested, primes_json
            );
            let mut hasher = Sha256::new();
            hasher.update(canonical.as_bytes());
            let expected_hash = format!("{:x}", hasher.finalize());

            if *hash == expected_hash {
                let _ = state
                    .db
                    .mark_block_hash_verified(payload.block_id)
                    .await;
                hash_matched = true;
            } else {
                tracing::warn!(
                    block_id = payload.block_id,
                    expected = %expected_hash,
                    actual = %hash,
                    "browser result hash mismatch — possible tampered result"
                );
                let _ = state.db.record_invalid_result(operator_id).await;
                warnings.push("hash mismatch — no credits for this block".to_string());
            }
        }
    } else {
        // No hash provided — acceptable but noted
        hash_matched = false;
    }

    // Store client-reported block duration for calibration
    if let Some(duration_ms) = payload.duration_ms {
        let _ = state
            .db
            .set_block_duration(payload.block_id, duration_ms)
            .await;
    }

    // Track credits granted for enriched response
    let mut credits_granted: i64 = 0;

    // Record any discovered primes — tagged "browser" for quorum verification
    for prime in &payload.primes {
        let tags: &[&str] = &[prime.form.as_str(), "browser"];
        match state
            .db
            .insert_prime_ignore(
                &prime.form,
                &prime.expression,
                prime.digits,
                "",
                &prime.proof_method,
                None,
                tags,
            )
            .await
        {
            Ok(_) => {
                let _ = state.db.increment_operator_primes(operator_id).await;
                // Bonus credit for browser discoveries
                let _ = state
                    .db
                    .grant_credit(
                        operator_id,
                        payload.block_id as i32,
                        1000,
                        "prime_discovered",
                    )
                    .await;
                credits_granted += 1000;
            }
            Err(e) => {
                tracing::warn!(
                    expression = %prime.expression,
                    error = %e,
                    "failed to insert browser prime"
                );
            }
        }
    }

    // Grant 50% credit for browser block completion (skip if hash mismatch)
    if !hash_matched && payload.result_hash.is_some() {
        // Hash was provided but didn't match — no block completion credit
    } else {
        let credit = (payload.tested.max(1)) / 2;
        let credit = credit.max(1);
        let _ = state
            .db
            .grant_credit(
                operator_id,
                payload.block_id as i32,
                credit,
                "browser_block_completed",
            )
            .await;
        credits_granted += credit;
    }

    // Record valid result for trust scoring
    let _ = state.db.record_valid_result(operator_id).await;

    // Check and grant any newly earned badges
    let new_badges_count = state
        .db
        .check_and_grant_badges(operator_id)
        .await
        .unwrap_or(0);

    // Fetch current trust level for response
    let current_trust_level = state
        .db
        .get_operator_trust(operator_id)
        .await
        .ok()
        .flatten()
        .map(|t| t.trust_level)
        .unwrap_or(1);

    // ── Step 4: Enriched response ──
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "hash_verified": hash_matched,
            "credits_earned": credits_granted,
            "trust_level": current_trust_level,
            "badges_earned": new_badges_count,
            "warnings": warnings,
        })),
    )
        .into_response()
}

// ── POST /api/v1/contribute/heartbeat ─────────────────────────────
//
// Keeps the browser "node" alive. Called every 30s while the contribute
// page is running. Uses worker ID `"browser-{operator_id}"`.
// Optionally accepts a JSON body with browser speed for adaptive block sizing.
// Also extends the lease on any claimed browser blocks.

#[derive(Deserialize, Default)]
pub(super) struct ContributeHeartbeatPayload {
    /// Browser compute speed in candidates/sec.
    #[serde(default)]
    speed: Option<f64>,
    /// Current search form (used for block size hint scaling).
    #[serde(default)]
    search_type: Option<String>,
}

pub(super) async fn handler_contribute_heartbeat(
    State(state): State<Arc<AppState>>,
    RequireAuth(auth_user): RequireAuth,
    body: Option<Json<ContributeHeartbeatPayload>>,
) -> impl IntoResponse {
    let operator_id = match get_operator_uuid(&state, &auth_user.user_id).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let node_id = format!("browser-{}", operator_id);

    // Store browser speed if provided
    let speed = body.as_ref().and_then(|b| b.speed);
    if let Some(s) = speed {
        if s > 0.0 {
            let _ = state.db.update_browser_speed(&node_id, s).await;
        }
    }

    // Extend lease on any claimed browser blocks (5 more minutes from now)
    let _ = state
        .db
        .extend_block_lease(&operator_id.to_string(), 5)
        .await;

    // Compute block size hint based on browser speed.
    // Native baseline ~1000 cand/s. Browser blocks default to 1000 candidates,
    // scale linearly with speed ratio (clamped to 100..10_000).
    let speed_scaled_size = if let Some(s) = speed {
        let native_baseline = 1000.0_f64;
        let ratio = (s / native_baseline).clamp(0.1, 10.0);
        1000.0 * ratio
    } else {
        1000.0
    };

    // Apply per-form complexity multiplier if search_type provided
    let search_type = body.as_ref().and_then(|b| b.search_type.as_deref());
    let multiplier = search_type.map(form_block_multiplier).unwrap_or(1.0);
    let block_size_hint = (speed_scaled_size * multiplier).round() as i64;

    match state.db.operator_node_heartbeat(&node_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "block_size_hint": block_size_hint,
            })),
        ),
        Err(e) => {
            tracing::warn!(error = %e, node_id = %node_id, "browser heartbeat failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Heartbeat failed"})),
            )
        }
    }
}
