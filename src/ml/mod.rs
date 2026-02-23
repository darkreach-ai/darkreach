//! # ML Engine — Unified Machine Learning for Prime Hunting
//!
//! Rust-native ML subsystems that augment the OODA decision loop in
//! [`ai_engine::AiEngine`] with data-driven models. Each subsystem has a
//! heuristic fallback, so the system degrades gracefully with insufficient data.
//!
//! ## Subsystems
//!
//! | Module | Purpose | Replaces |
//! |--------|---------|----------|
//! | [`bandits`] | Thompson Sampling for form selection | 10-weight heuristic scoring |
//! | [`gp_cost`] | Gaussian Process cost prediction | OLS power-law fitting |
//! | [`bayesopt`] | Bayesian optimization for sieve/block tuning | `auto_sieve_depth()` heuristic |
//! | [`node_intel`] | Anomaly detection + smart node routing | Static `score_job_for_worker()` |
//! | [`features`] | Feature engineering pipeline | Inline feature extraction |
//! | [`registry`] | Model versioning + shadow mode | — |
//!
//! ## Integration
//!
//! The [`MlEngine`] struct is embedded in `AiEngine` and participates in
//! each OODA phase:
//!
//! - **OBSERVE**: Feature store refresh, heartbeat anomaly updates
//! - **ORIENT**: Thompson sampling replaces/blends with heuristic scoring
//! - **DECIDE**: GP cost predictions with uncertainty, BayesOpt parameter suggestions
//! - **ACT**: Node intelligence for smart work block assignment
//! - **LEARN**: All subsystems update from completed blocks/projects
//!
//! ## Confidence Gates
//!
//! Each subsystem declares a minimum observation count. Below that threshold,
//! the ML prediction is blended with the heuristic fallback using [`ml_blend`].

pub mod bandits;
pub mod bayesopt;
pub mod features;
pub mod gp_cost;
pub mod node_intel;
pub mod registry;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::ai_engine::WorldSnapshot;
use crate::db::Database;

pub use bandits::FormBandits;
pub use bayesopt::SieveOptimizer;
pub use features::{FeatureStore, NodeClass};
pub use gp_cost::CostGpModel;
pub use node_intel::NodeIntelligence;
pub use registry::ModelRegistry;

// ── MlEngine ───────────────────────────────────────────────────

/// Central ML engine, embedded in [`AiEngine`].
///
/// Orchestrates all subsystems and provides the master [`learn`] step
/// that feeds observations to each model after completed work blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlEngine {
    /// Thompson Sampling multi-armed bandit for form selection.
    pub bandits: FormBandits,
    /// Gaussian Process cost predictor with uncertainty quantification.
    pub cost_gp: CostGpModel,
    /// Bayesian optimizer for sieve depth and block size per form.
    pub sieve_opt: SieveOptimizer,
    /// Anomaly detection and learned (node, form) affinity routing.
    pub node_intel: NodeIntelligence,
    /// Model version control and shadow mode evaluation.
    pub registry: ModelRegistry,
    /// Cached feature vectors refreshed each tick.
    #[serde(skip)]
    pub features: FeatureStore,
    /// Monotonically increasing learn cycle counter.
    pub learn_count: u64,
    /// Timestamp of last learn cycle.
    pub last_learn_ts: DateTime<Utc>,
}

impl Default for MlEngine {
    fn default() -> Self {
        Self {
            bandits: FormBandits::default(),
            cost_gp: CostGpModel::default(),
            sieve_opt: SieveOptimizer::default(),
            node_intel: NodeIntelligence::default(),
            registry: ModelRegistry::default(),
            features: FeatureStore::default(),
            learn_count: 0,
            last_learn_ts: Utc::now(),
        }
    }
}

/// Outcome of a single learn cycle, for logging and metrics.
#[derive(Debug, Clone, Serialize)]
pub struct LearnOutcome {
    pub blocks_processed: usize,
    pub projects_processed: usize,
    pub gp_refitted: bool,
    pub promotions: Vec<String>,
}

impl MlEngine {
    /// Refresh the feature store from the latest world snapshot.
    ///
    /// Called at the start of each OODA tick before any ML predictions.
    pub fn refresh_features(&mut self, snapshot: &WorldSnapshot) {
        self.features.refresh(snapshot);
    }

    /// Master learn step — called every `learn_interval_secs`.
    ///
    /// Orchestrates all subsystem learning in priority order:
    /// 1. Collect new observations from completed blocks
    /// 2. Feed to cost GP, sieve optimizer, and node intelligence
    /// 3. Update bandit posteriors from completed projects
    /// 4. Decay sliding windows for non-stationarity
    /// 5. Periodically refit GP hyperparameters (every 10 cycles)
    /// 6. Persist all state to PostgreSQL
    /// 7. Evaluate shadow models for promotion
    pub async fn learn(&mut self, db: &Database, snapshot: &WorldSnapshot) -> Result<LearnOutcome> {
        let mut blocks_processed = 0;
        let mut projects_processed = 0;

        // 1. Collect new observations from recently completed blocks
        if let Ok(blocks) = db.get_ml_recent_blocks(self.last_learn_ts).await {
            for block in &blocks {
                let node_class = NodeClass::classify(
                    block.worker_cores.unwrap_or(4) as u32,
                    block.gpu_runtime.as_deref(),
                );

                // Feed to cost GP
                if block.digits > 0.0 && block.secs > 0.0 {
                    self.cost_gp
                        .observe(&block.form, block.digits, block.secs, node_class);
                }

                // Feed to sieve optimizer
                if block.throughput > 0.0 {
                    self.sieve_opt.observe(
                        &block.form,
                        block.sieve_depth as u64,
                        block.block_size,
                        block.digits,
                        block.throughput,
                    );
                }

                // Feed to node intelligence
                if let Some(ref node_id) = block.worker_id {
                    self.node_intel.observe_block(
                        node_id,
                        &block.form,
                        block.success,
                        block.throughput,
                        block.secs,
                    );
                }
            }
            blocks_processed = blocks.len();
        }

        // 2. Update bandit posteriors from completed projects
        if let Ok(projects) = db
            .get_ml_recently_completed_projects(self.last_learn_ts)
            .await
        {
            for project in &projects {
                let reward = project.total_found as f64 / project.total_core_hours.max(0.01);
                let context = bandits::FormContext::from_snapshot(snapshot);
                self.bandits.update(&project.form, reward, &context);
            }
            projects_processed = projects.len();
        }

        // 3. Decay sliding windows
        let cutoff = Utc::now() - chrono::Duration::days(self.bandits.window_days as i64);
        self.bandits.decay_window(cutoff);

        // 4. Refit GP models every 10 learn cycles (expensive)
        let gp_refitted = self.learn_count % 10 == 0;
        if gp_refitted {
            for &form in crate::strategy::ALL_FORMS {
                self.cost_gp.refit(form);
            }
        }

        // 5. Persist all state
        self.save_all(db).await;

        // 6. Evaluate shadow models
        let promotions = self.registry.evaluate_and_promote();
        if !promotions.is_empty() {
            info!(promoted = ?promotions, "ml: shadow models promoted");
        }

        self.learn_count += 1;
        self.last_learn_ts = Utc::now();

        info!(
            blocks_processed,
            projects_processed,
            gp_refitted,
            learn_count = self.learn_count,
            "ml: learn cycle complete"
        );

        Ok(LearnOutcome {
            blocks_processed,
            projects_processed,
            gp_refitted,
            promotions,
        })
    }

    /// Persist all ML subsystem state to PostgreSQL.
    async fn save_all(&self, db: &Database) {
        if let Err(e) = db.save_bandit_arms(&self.bandits.arms).await {
            warn!(error = %e, "ml: failed to save bandit arms");
        }
        if let Err(e) = db.save_gp_state(&self.cost_gp).await {
            warn!(error = %e, "ml: failed to save GP state");
        }
        if let Err(e) = db.save_bayesopt_state(&self.sieve_opt).await {
            warn!(error = %e, "ml: failed to save BayesOpt state");
        }
        if let Err(e) = db.save_node_profiles(&self.node_intel).await {
            warn!(error = %e, "ml: failed to save node profiles");
        }
    }

    /// Load all ML subsystem state from PostgreSQL on startup.
    pub async fn load_all(&mut self, db: &Database) {
        match db.load_bandit_arms().await {
            Ok(arms) if !arms.is_empty() => {
                self.bandits.arms = arms;
                info!(arms = self.bandits.arms.len(), "ml: loaded bandit arms");
            }
            Ok(_) => info!("ml: no bandit arms in DB, using defaults"),
            Err(e) => warn!(error = %e, "ml: failed to load bandit arms"),
        }

        match db.load_gp_state().await {
            Ok(state) if !state.is_empty() => {
                for (form, gp) in state {
                    self.cost_gp.models.insert(form, gp);
                }
                info!(models = self.cost_gp.models.len(), "ml: loaded GP models");
            }
            Ok(_) => info!("ml: no GP state in DB, using defaults"),
            Err(e) => warn!(error = %e, "ml: failed to load GP state"),
        }

        match db.load_bayesopt_state().await {
            Ok(state) if !state.is_empty() => {
                for (form, opt) in state {
                    self.sieve_opt.optimizers.insert(form, opt);
                }
                info!(
                    optimizers = self.sieve_opt.optimizers.len(),
                    "ml: loaded BayesOpt state"
                );
            }
            Ok(_) => info!("ml: no BayesOpt state in DB, using defaults"),
            Err(e) => warn!(error = %e, "ml: failed to load BayesOpt state"),
        }

        match db.load_node_profiles().await {
            Ok(profiles) if !profiles.is_empty() => {
                let count = profiles.len();
                self.node_intel.profiles = profiles;
                info!(profiles = count, "ml: loaded node profiles");
            }
            Ok(_) => info!("ml: no node profiles in DB, using defaults"),
            Err(e) => warn!(error = %e, "ml: failed to load node profiles"),
        }
    }
}

// ── Blend Function ─────────────────────────────────────────────

/// Blend ML score with heuristic fallback based on ML confidence.
///
/// The blend curve is linear from 0% ML weight at confidence ≤ 0.3 to
/// 100% ML weight at confidence ≥ 1.0. This ensures the heuristic
/// dominates when the ML model has little data.
///
/// ```text
/// weight = clamp((confidence - 0.3) / 0.7, 0, 1)
/// result = weight * ml_score + (1 - weight) * heuristic_score
/// ```
pub fn ml_blend(ml_score: f64, heuristic_score: f64, ml_confidence: f64) -> f64 {
    let weight = ((ml_confidence - 0.3) / 0.7).clamp(0.0, 1.0);
    weight * ml_score + (1.0 - weight) * heuristic_score
}

// ── DB Row Types for ML ────────────────────────────────────────

/// A recently completed work block, enriched with form and node metadata.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MlBlockRow {
    pub form: String,
    pub digits: f64,
    pub secs: f64,
    pub throughput: f64,
    pub sieve_depth: i64,
    pub block_size: i64,
    pub success: bool,
    pub worker_id: Option<String>,
    pub worker_cores: Option<i32>,
    pub gpu_runtime: Option<String>,
}

/// A recently completed project for bandit reward updates.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MlProjectRow {
    pub form: String,
    pub total_found: i64,
    pub total_core_hours: f64,
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ml_blend_zero_confidence() {
        // At confidence 0.0, fully heuristic
        let result = ml_blend(1.0, 0.5, 0.0);
        assert!((result - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ml_blend_low_confidence() {
        // At confidence 0.3, still fully heuristic
        let result = ml_blend(1.0, 0.5, 0.3);
        assert!((result - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ml_blend_full_confidence() {
        // At confidence 1.0, fully ML
        let result = ml_blend(1.0, 0.5, 1.0);
        assert!((result - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ml_blend_mid_confidence() {
        // At confidence 0.65, weight = (0.65 - 0.3) / 0.7 = 0.5
        let result = ml_blend(1.0, 0.0, 0.65);
        assert!((result - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ml_blend_clamps_high() {
        // Confidence > 1.0 should clamp to 1.0
        let result = ml_blend(1.0, 0.0, 2.0);
        assert!((result - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_ml_engine() {
        let ml = MlEngine::default();
        assert_eq!(ml.learn_count, 0);
        assert!(ml.bandits.arms.is_empty());
        assert!(ml.cost_gp.models.is_empty());
    }
}
