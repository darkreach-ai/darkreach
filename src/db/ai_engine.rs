//! AI engine database operations.
//!
//! Manages the `ai_engine_state` singleton and `ai_engine_decisions` audit trail.
//! Also provides the `cost_observations` query for the LEARN phase OLS fitting,
//! and helper queries for momentum scoring and agent result integration.

use super::Database;
use anyhow::Result;
use serde::Serialize;

/// Row from the `ai_engine_state` singleton table.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AiEngineStateRow {
    pub id: i64,
    pub scoring_weights: serde_json::Value,
    pub cost_model_version: i32,
    pub last_tick_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_learn_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tick_count: i64,
    pub config: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Row from the `ai_engine_decisions` audit trail.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AiEngineDecisionRow {
    pub id: i64,
    pub tick_id: i64,
    pub decision_type: String,
    pub form: Option<String>,
    pub action: String,
    pub reasoning: String,
    pub confidence: Option<f64>,
    pub snapshot_hash: Option<String>,
    pub params: Option<serde_json::Value>,
    pub outcome: Option<serde_json::Value>,
    pub outcome_measured_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    /// Get the AI engine state singleton. Returns None if no row exists.
    pub async fn get_ai_engine_state(&self) -> Result<Option<AiEngineStateRow>> {
        let row = sqlx::query_as::<_, AiEngineStateRow>(
            "SELECT id, scoring_weights, cost_model_version, last_tick_at,
                    last_learn_at, tick_count, config, updated_at
             FROM ai_engine_state
             LIMIT 1",
        )
        .fetch_optional(&self.read_pool)
        .await?;
        Ok(row)
    }

    /// Upsert the AI engine state (scoring weights, tick count, cost model version).
    pub async fn upsert_ai_engine_state(
        &self,
        scoring_weights: &serde_json::Value,
        cost_model_version: i32,
        tick_count: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO ai_engine_state (id, scoring_weights, cost_model_version, tick_count, last_tick_at, updated_at)
             OVERRIDING SYSTEM VALUE
             VALUES (1, $1, $2, $3, NOW(), NOW())
             ON CONFLICT (id) DO UPDATE SET
               scoring_weights = EXCLUDED.scoring_weights,
               cost_model_version = EXCLUDED.cost_model_version,
               tick_count = EXCLUDED.tick_count,
               last_tick_at = NOW(),
               updated_at = NOW()",
        )
        .bind(scoring_weights)
        .bind(cost_model_version)
        .bind(tick_count)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Insert a decision into the AI engine audit trail.
    pub async fn insert_ai_engine_decision(
        &self,
        tick_id: i64,
        decision_type: &str,
        form: Option<&str>,
        action: &str,
        reasoning: &str,
        confidence: f64,
        params: Option<&serde_json::Value>,
    ) -> Result<i64> {
        self.insert_ai_engine_decision_with_scores(
            tick_id,
            decision_type,
            form,
            action,
            reasoning,
            confidence,
            params,
            None,
        )
        .await
    }

    /// Insert a decision with component scores for outcome correlation.
    pub async fn insert_ai_engine_decision_with_scores(
        &self,
        tick_id: i64,
        decision_type: &str,
        form: Option<&str>,
        action: &str,
        reasoning: &str,
        confidence: f64,
        params: Option<&serde_json::Value>,
        component_scores: Option<&serde_json::Value>,
    ) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO ai_engine_decisions
                (tick_id, decision_type, form, action, reasoning, confidence, params, component_scores)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
        )
        .bind(tick_id)
        .bind(decision_type)
        .bind(form)
        .bind(action)
        .bind(reasoning)
        .bind(confidence)
        .bind(params)
        .bind(component_scores)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Get recent AI engine decisions, newest first.
    pub async fn get_ai_engine_decisions(&self, limit: i64) -> Result<Vec<AiEngineDecisionRow>> {
        let rows = sqlx::query_as::<_, AiEngineDecisionRow>(
            "SELECT id, tick_id, decision_type, form, action, reasoning,
                    confidence, snapshot_hash, params, outcome,
                    outcome_measured_at, created_at
             FROM ai_engine_decisions
             ORDER BY created_at DESC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Get cost observations for OLS fitting in the LEARN phase.
    /// Returns (digits, secs_per_candidate) pairs from completed work blocks.
    pub async fn get_cost_observations(
        &self,
        form: &str,
        limit: i64,
    ) -> Result<Vec<crate::ai_engine::CostObservation>> {
        let rows = sqlx::query_as::<_, crate::ai_engine::CostObservation>(
            "SELECT digits::float8 as digits, secs::float8 as secs
             FROM cost_observations
             WHERE form = $1
               AND digits > 0
               AND secs > 0
               AND secs < 86400
             ORDER BY completed_at DESC
             LIMIT $2",
        )
        .bind(form)
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Get recent prime discoveries for momentum scoring.
    /// Returns primes found in the last N days, grouped by form.
    pub async fn get_recent_primes_for_momentum(
        &self,
        days: i32,
    ) -> Result<Vec<crate::ai_engine::RecentDiscovery>> {
        let rows = sqlx::query_as::<_, RecentPrimeRow>(
            "SELECT form, digits, found_at
             FROM primes
             WHERE found_at > NOW() - ($1 || ' days')::interval
             ORDER BY found_at DESC
             LIMIT 100",
        )
        .bind(days)
        .fetch_all(&self.read_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| crate::ai_engine::RecentDiscovery {
                form: r.form,
                digits: r.digits,
                found_at: r.found_at,
            })
            .collect())
    }

    /// Get recent completed agent task results for feedback integration.
    pub async fn get_recent_agent_results(
        &self,
        limit: i64,
    ) -> Result<Vec<super::AgentTaskRow>> {
        let rows = sqlx::query_as::<_, super::AgentTaskRow>(
            "SELECT id, title, description, status, priority, agent_model,
                    assigned_agent, source, result, tokens_used, cost_usd,
                    created_at, started_at, completed_at, parent_task_id,
                    max_cost_usd, permission_level, template_name,
                    on_child_failure, role_name
             FROM agent_tasks
             WHERE status IN ('completed', 'failed')
               AND completed_at > NOW() - INTERVAL '24 hours'
             ORDER BY completed_at DESC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    // ── Phase 6: Outcome measurement ─────────────────────────────

    /// Get create_project decisions that need outcome measurement.
    /// Joins with projects by form + creation time proximity to find
    /// the associated project for each decision.
    pub async fn get_decisions_needing_outcomes(
        &self,
        limit: i64,
    ) -> Result<Vec<DecisionOutcomeCandidate>> {
        let rows = sqlx::query_as::<_, DecisionOutcomeCandidate>(
            "SELECT d.id AS decision_id,
                    d.form AS decision_form,
                    d.component_scores,
                    d.created_at AS decision_created_at,
                    p.id AS project_id,
                    p.status AS project_status,
                    p.total_found AS primes_found,
                    p.total_cost_usd AS cost_usd,
                    p.total_core_hours AS core_hours
             FROM ai_engine_decisions d
             JOIN projects p ON p.form = d.form
               AND p.created_at BETWEEN d.created_at - INTERVAL '5 minutes'
                                     AND d.created_at + INTERVAL '5 minutes'
             WHERE d.outcome IS NULL
               AND d.decision_type = 'create_project'
               AND p.status IN ('completed', 'failed', 'cancelled')
             ORDER BY d.created_at ASC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Update a decision's outcome after measurement.
    pub async fn update_decision_outcome(
        &self,
        decision_id: i64,
        outcome: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE ai_engine_decisions
             SET outcome = $2, outcome_measured_at = NOW()
             WHERE id = $1",
        )
        .bind(decision_id)
        .bind(outcome)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get recent decisions that have outcomes and component scores,
    /// for weight learning via EWA.
    pub async fn get_outcomes_with_scores(
        &self,
        limit: i64,
    ) -> Result<Vec<DecisionWithOutcome>> {
        let rows = sqlx::query_as::<_, DecisionWithOutcome>(
            "SELECT id, form, component_scores, outcome
             FROM ai_engine_decisions
             WHERE outcome IS NOT NULL
               AND component_scores IS NOT NULL
               AND decision_type = 'create_project'
             ORDER BY outcome_measured_at DESC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    // ── Phase 8: Worker speed ────────────────────────────────────

    /// Get per-worker, per-form speed statistics from the materialized view.
    pub async fn get_worker_speeds(&self) -> Result<Vec<WorkerSpeedRow>> {
        let rows = sqlx::query_as::<_, WorkerSpeedRow>(
            "SELECT worker_id, form, blocks_completed, avg_block_secs, candidates_per_sec
             FROM worker_speed",
        )
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Count available work blocks for a specific search job.
    pub async fn get_available_block_count(&self, job_id: i64) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM work_blocks
             WHERE search_job_id = $1 AND status = 'available'",
        )
        .bind(job_id)
        .fetch_one(&self.read_pool)
        .await?;
        Ok(count)
    }

    /// Count workers with a recent heartbeat (active in last 120s).
    pub async fn count_active_workers(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM workers
             WHERE last_heartbeat > NOW() - INTERVAL '120 seconds'",
        )
        .fetch_one(&self.read_pool)
        .await?;
        Ok(count)
    }

    /// Release all claimed blocks for a worker back to available status.
    /// Used by fleet rebalancing to free up blocks from overprovisioned workers.
    pub async fn release_worker_blocks(&self, worker_id: &str) -> Result<i64> {
        let result = sqlx::query(
            "UPDATE work_blocks SET status = 'available', claimed_by = NULL, claimed_at = NULL
             WHERE claimed_by = $1 AND status = 'claimed'",
        )
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }
}

/// A decision that needs outcome measurement, joined with project results.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DecisionOutcomeCandidate {
    pub decision_id: i64,
    pub decision_form: Option<String>,
    pub component_scores: Option<serde_json::Value>,
    pub decision_created_at: chrono::DateTime<chrono::Utc>,
    pub project_id: i64,
    pub project_status: String,
    pub primes_found: i64,
    pub cost_usd: f64,
    pub core_hours: f64,
}

/// A decision with measured outcome and component scores, for weight learning.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DecisionWithOutcome {
    pub id: i64,
    pub form: Option<String>,
    pub component_scores: Option<serde_json::Value>,
    pub outcome: Option<serde_json::Value>,
}

/// Worker speed statistics from the materialized view.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WorkerSpeedRow {
    pub worker_id: String,
    pub form: String,
    pub blocks_completed: i64,
    pub avg_block_secs: f64,
    pub candidates_per_sec: f64,
}

/// Helper row type for the recent primes query.
#[derive(sqlx::FromRow)]
struct RecentPrimeRow {
    form: String,
    digits: i64,
    found_at: chrono::DateTime<chrono::Utc>,
}
