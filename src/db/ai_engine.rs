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
    pub async fn get_recent_agent_results(&self, limit: i64) -> Result<Vec<super::AgentTaskRow>> {
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

    /// Get cost observations for GPU-accelerated workers, for fitting separate
    /// GPU timing curves in the LEARN phase. Reconstructs the same digit/secs
    /// estimates as the `cost_observations` view but filtered to work blocks
    /// completed by workers registered in `operator_nodes` with a GPU runtime.
    pub async fn get_gpu_cost_observations(
        &self,
        form: &str,
        limit: i64,
    ) -> Result<Vec<crate::ai_engine::CostObservation>> {
        let rows = sqlx::query_as::<_, crate::ai_engine::CostObservation>(
            "SELECT
                CASE
                    WHEN sj.search_type IN ('palindromic', 'near_repdigit', 'repunit') THEN
                        ((wb.block_start + wb.block_end) / 2.0)
                    WHEN sj.search_type = 'factorial' THEN
                        ((wb.block_start + wb.block_end) / 2.0)
                        * LN((wb.block_start + wb.block_end) / 2.0 / EXP(1.0)) / LN(10.0)
                    WHEN sj.search_type = 'primorial' THEN
                        ((wb.block_start + wb.block_end) / 2.0) / LN(10.0)
                    ELSE
                        ((wb.block_start + wb.block_end) / 2.0) * 0.301
                END::float8 AS digits,
                (EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at))::float8 / wb.tested) AS secs
             FROM work_blocks wb
             JOIN search_jobs sj ON sj.id = wb.search_job_id
             JOIN operator_nodes on_ ON on_.worker_id = wb.claimed_by
             WHERE sj.search_type = $1
               AND wb.status = 'completed'
               AND wb.tested > 0
               AND wb.completed_at IS NOT NULL
               AND wb.claimed_at IS NOT NULL
               AND wb.completed_at > wb.claimed_at
               AND on_.gpu_runtime IS NOT NULL
               AND on_.gpu_runtime <> 'none'
             ORDER BY wb.completed_at DESC
             LIMIT $2",
        )
        .bind(form)
        .bind(limit)
        .fetch_all(&self.read_pool)
        .await?;
        // Filter out invalid results (negative or zero digits/secs)
        Ok(rows
            .into_iter()
            .filter(|o| o.digits > 0.0 && o.secs > 0.0 && o.secs < 86400.0)
            .collect())
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
    pub async fn get_outcomes_with_scores(&self, limit: i64) -> Result<Vec<DecisionWithOutcome>> {
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

    /// Count available work blocks grouped by search form (search_type).
    ///
    /// Joins `work_blocks` with `search_jobs` to map blocks back to their form.
    /// Used by fleet rebalancing to detect forms with queued work vs exhausted forms.
    pub async fn get_available_blocks_by_form(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT j.search_type, COUNT(*)::bigint
             FROM work_blocks b
             JOIN search_jobs j ON j.id = b.search_job_id
             WHERE b.status = 'available' AND j.status = 'running'
             GROUP BY j.search_type",
        )
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows.into_iter().collect())
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

// ── ML Engine Database Operations ──────────────────────────────

impl Database {
    /// Save all bandit arm states to `ml_bandit_arms`.
    pub async fn save_bandit_arms(
        &self,
        arms: &std::collections::HashMap<String, crate::ml::bandits::FormArm>,
    ) -> anyhow::Result<()> {
        for arm in arms.values() {
            let context_weights = serde_json::to_value(&[0.2f64; 5])?;
            sqlx::query(
                "INSERT INTO ml_bandit_arms (form, alpha, beta, mean_reward, reward_var,
                    n_obs, window_alpha, window_beta, window_n, context_weights, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
                 ON CONFLICT (form) DO UPDATE SET
                    alpha = EXCLUDED.alpha, beta = EXCLUDED.beta,
                    mean_reward = EXCLUDED.mean_reward, reward_var = EXCLUDED.reward_var,
                    n_obs = EXCLUDED.n_obs, window_alpha = EXCLUDED.window_alpha,
                    window_beta = EXCLUDED.window_beta, window_n = EXCLUDED.window_n,
                    context_weights = EXCLUDED.context_weights, updated_at = NOW()",
            )
            .bind(&arm.form)
            .bind(arm.alpha)
            .bind(arm.beta)
            .bind(arm.mean_reward)
            .bind(arm.reward_var)
            .bind(arm.n_obs as i64)
            .bind(arm.window_alpha)
            .bind(arm.window_beta)
            .bind(arm.window_n as i64)
            .bind(&context_weights)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Load bandit arm states from `ml_bandit_arms`.
    pub async fn load_bandit_arms(
        &self,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::ml::bandits::FormArm>> {
        let rows = sqlx::query_as::<_, MlBanditArmRow>(
            "SELECT form, alpha, beta, mean_reward, reward_var,
                    n_obs, window_alpha, window_beta, window_n
             FROM ml_bandit_arms",
        )
        .fetch_all(&self.read_pool)
        .await?;

        let mut arms = std::collections::HashMap::new();
        for row in rows {
            arms.insert(
                row.form.clone(),
                crate::ml::bandits::FormArm {
                    form: row.form,
                    alpha: row.alpha,
                    beta: row.beta,
                    mean_reward: row.mean_reward,
                    reward_var: row.reward_var,
                    n_obs: row.n_obs as u64,
                    window_alpha: row.window_alpha,
                    window_beta: row.window_beta,
                    window_n: row.window_n as u64,
                },
            );
        }
        Ok(arms)
    }

    /// Save GP model state to `ml_gp_state`.
    pub async fn save_gp_state(&self, gp: &crate::ml::gp_cost::CostGpModel) -> anyhow::Result<()> {
        for (form, model) in &gp.models {
            let hyperparams = serde_json::json!({
                "lengthscales": model.lengthscales,
                "noise_var": model.noise_var,
            });
            sqlx::query(
                "INSERT INTO ml_gp_state (form, n_points, last_mape, hyperparams, updated_at)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (form) DO UPDATE SET
                    n_points = EXCLUDED.n_points, last_mape = EXCLUDED.last_mape,
                    hyperparams = EXCLUDED.hyperparams, updated_at = NOW()",
            )
            .bind(form)
            .bind(model.n_points as i32)
            .bind(model.last_mape)
            .bind(&hyperparams)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Load GP model state from `ml_gp_state`.
    pub async fn load_gp_state(
        &self,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::ml::gp_cost::FormCostGp>> {
        let rows = sqlx::query_as::<_, MlGpStateRow>(
            "SELECT form, n_points, last_mape, hyperparams FROM ml_gp_state",
        )
        .fetch_all(&self.read_pool)
        .await?;

        let mut models = std::collections::HashMap::new();
        for row in rows {
            let lengthscales = row
                .hyperparams
                .as_ref()
                .and_then(|h| h.get("lengthscales"))
                .and_then(|v| serde_json::from_value::<[f64; 3]>(v.clone()).ok())
                .unwrap_or([1.0, 1.0, 1.0]);
            let noise_var = row
                .hyperparams
                .as_ref()
                .and_then(|h| h.get("noise_var"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.1);

            models.insert(
                row.form,
                crate::ml::gp_cost::FormCostGp {
                    training_x: Vec::new(),
                    training_y: Vec::new(),
                    n_points: row.n_points as usize,
                    max_points: 500,
                    last_mape: row.last_mape.unwrap_or(1.0),
                    last_fit: chrono::Utc::now(),
                    lengthscales,
                    noise_var,
                },
            );
        }
        Ok(models)
    }

    /// Save BayesOpt optimizer state to `ml_bayesopt_state`.
    pub async fn save_bayesopt_state(
        &self,
        opt: &crate::ml::bayesopt::SieveOptimizer,
    ) -> anyhow::Result<()> {
        for (form, state) in &opt.optimizers {
            sqlx::query(
                "INSERT INTO ml_bayesopt_state
                    (form, best_sieve_depth, best_block_size, best_throughput, n_evals, updated_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())
                 ON CONFLICT (form) DO UPDATE SET
                    best_sieve_depth = EXCLUDED.best_sieve_depth,
                    best_block_size = EXCLUDED.best_block_size,
                    best_throughput = EXCLUDED.best_throughput,
                    n_evals = EXCLUDED.n_evals, updated_at = NOW()",
            )
            .bind(form)
            .bind(state.best_sieve_depth as i64)
            .bind(state.best_block_size)
            .bind(state.best_throughput)
            .bind(state.n_evals as i64)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Load BayesOpt state from `ml_bayesopt_state`.
    pub async fn load_bayesopt_state(
        &self,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::ml::bayesopt::FormOptState>> {
        let rows = sqlx::query_as::<_, MlBayesOptStateRow>(
            "SELECT form, best_sieve_depth, best_block_size, best_throughput, n_evals
             FROM ml_bayesopt_state",
        )
        .fetch_all(&self.read_pool)
        .await?;

        let mut optimizers = std::collections::HashMap::new();
        for row in rows {
            optimizers.insert(
                row.form,
                crate::ml::bayesopt::FormOptState {
                    observations: Vec::new(),
                    best_sieve_depth: row.best_sieve_depth as u64,
                    best_block_size: row.best_block_size,
                    best_throughput: row.best_throughput,
                    n_evals: row.n_evals as u64,
                },
            );
        }
        Ok(optimizers)
    }

    /// Save node intelligence profiles to `ml_node_profiles` and `ml_node_form_affinity`.
    pub async fn save_node_profiles(
        &self,
        intel: &crate::ml::node_intel::NodeIntelligence,
    ) -> anyhow::Result<()> {
        for (node_id, profile) in &intel.profiles {
            let avg_throughput = serde_json::to_value(&profile.avg_throughput)?;
            let failure_rate = serde_json::to_value(&profile.failure_rate)?;
            let anomaly_score = intel.anomaly_score(node_id);

            sqlx::query(
                "INSERT INTO ml_node_profiles
                    (node_id, node_class, avg_throughput, failure_rate,
                     blocks_completed, anomaly_score, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, NOW())
                 ON CONFLICT (node_id) DO UPDATE SET
                    node_class = EXCLUDED.node_class,
                    avg_throughput = EXCLUDED.avg_throughput,
                    failure_rate = EXCLUDED.failure_rate,
                    blocks_completed = EXCLUDED.blocks_completed,
                    anomaly_score = EXCLUDED.anomaly_score, updated_at = NOW()",
            )
            .bind(node_id)
            .bind(profile.node_class.to_string())
            .bind(&avg_throughput)
            .bind(&failure_rate)
            .bind(profile.blocks_completed as i64)
            .bind(anomaly_score)
            .execute(&self.pool)
            .await?;
        }

        for ((node_id, form), aff) in &intel.affinity {
            sqlx::query(
                "INSERT INTO ml_node_form_affinity
                    (node_id, form, alpha, beta, mean_throughput, n_obs, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, NOW())
                 ON CONFLICT (node_id, form) DO UPDATE SET
                    alpha = EXCLUDED.alpha, beta = EXCLUDED.beta,
                    mean_throughput = EXCLUDED.mean_throughput,
                    n_obs = EXCLUDED.n_obs, updated_at = NOW()",
            )
            .bind(node_id)
            .bind(form)
            .bind(aff.alpha)
            .bind(aff.beta)
            .bind(aff.mean_throughput)
            .bind(aff.n_obs as i64)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Load node intelligence profiles from `ml_node_profiles`.
    pub async fn load_node_profiles(
        &self,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::ml::node_intel::NodeProfile>> {
        let rows = sqlx::query_as::<_, MlNodeProfileRow>(
            "SELECT node_id, node_class, avg_throughput, failure_rate, blocks_completed
             FROM ml_node_profiles",
        )
        .fetch_all(&self.read_pool)
        .await?;

        let mut profiles = std::collections::HashMap::new();
        for row in rows {
            let avg_throughput: std::collections::HashMap<String, f64> = row
                .avg_throughput
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let failure_rate: std::collections::HashMap<String, f64> = row
                .failure_rate
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let node_class = match row.node_class.as_deref() {
                Some("cpu_small") => crate::ml::features::NodeClass::CpuSmall,
                Some("cpu_large") => crate::ml::features::NodeClass::CpuLarge,
                Some("gpu_cuda") => crate::ml::features::NodeClass::GpuCuda,
                Some("gpu_metal") => crate::ml::features::NodeClass::GpuMetal,
                _ => crate::ml::features::NodeClass::CpuMedium,
            };

            profiles.insert(
                row.node_id.clone(),
                crate::ml::node_intel::NodeProfile {
                    node_id: row.node_id,
                    node_class,
                    avg_throughput,
                    throughput_var: std::collections::HashMap::new(),
                    failure_rate,
                    blocks_completed: row.blocks_completed as u64,
                },
            );
        }
        Ok(profiles)
    }

    /// Get recently completed work blocks enriched with form and node metadata.
    pub async fn get_ml_recent_blocks(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Vec<crate::ml::MlBlockRow>> {
        let rows = sqlx::query_as::<_, crate::ml::MlBlockRow>(
            "SELECT sj.search_type AS form,
                    CASE
                        WHEN sj.search_type IN ('palindromic', 'near_repdigit', 'repunit') THEN
                            ((wb.block_start + wb.block_end) / 2.0)
                        WHEN sj.search_type = 'factorial' THEN
                            ((wb.block_start + wb.block_end) / 2.0)
                            * LN((wb.block_start + wb.block_end) / 2.0 / EXP(1.0)) / LN(10.0)
                        ELSE
                            ((wb.block_start + wb.block_end) / 2.0) * 0.301
                    END::float8 AS digits,
                    EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at))::float8 AS secs,
                    CASE WHEN wb.tested > 0 AND EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at)) > 0
                         THEN wb.found::float8 / (EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at)) / 3600.0)
                         ELSE 0.0
                    END AS throughput,
                    COALESCE(wb.sieved_out, 0)::bigint AS sieve_depth,
                    COALESCE(wb.block_end - wb.block_start, 1000)::bigint AS block_size,
                    (wb.status = 'completed') AS success,
                    wb.claimed_by AS worker_id,
                    w.cores AS worker_cores,
                    NULL::text AS gpu_runtime
             FROM work_blocks wb
             JOIN search_jobs sj ON sj.id = wb.search_job_id
             LEFT JOIN workers w ON w.worker_id = wb.claimed_by
             WHERE wb.status IN ('completed', 'failed')
               AND wb.completed_at IS NOT NULL
               AND wb.completed_at > $1
               AND wb.claimed_at IS NOT NULL
               AND wb.completed_at > wb.claimed_at
               AND wb.tested > 0
             ORDER BY wb.completed_at DESC
             LIMIT 500",
        )
        .bind(since)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }

    /// Get recently completed projects for bandit reward updates.
    pub async fn get_ml_recently_completed_projects(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Vec<crate::ml::MlProjectRow>> {
        let rows = sqlx::query_as::<_, crate::ml::MlProjectRow>(
            "SELECT form, total_found, total_core_hours
             FROM projects
             WHERE status = 'completed'
               AND completed_at > $1
             ORDER BY completed_at DESC
             LIMIT 50",
        )
        .bind(since)
        .fetch_all(&self.read_pool)
        .await?;
        Ok(rows)
    }
}

// ── ML Row Types ───────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MlBanditArmRow {
    form: String,
    alpha: f64,
    beta: f64,
    mean_reward: f64,
    reward_var: f64,
    n_obs: i64,
    window_alpha: f64,
    window_beta: f64,
    window_n: i64,
}

#[derive(sqlx::FromRow)]
struct MlGpStateRow {
    form: String,
    n_points: i32,
    last_mape: Option<f64>,
    hyperparams: Option<serde_json::Value>,
}

#[derive(sqlx::FromRow)]
struct MlBayesOptStateRow {
    form: String,
    best_sieve_depth: i64,
    best_block_size: i64,
    best_throughput: f64,
    n_evals: i64,
}

#[derive(sqlx::FromRow)]
struct MlNodeProfileRow {
    node_id: String,
    node_class: Option<String>,
    avg_throughput: Option<serde_json::Value>,
    failure_rate: Option<serde_json::Value>,
    blocks_completed: i64,
}
