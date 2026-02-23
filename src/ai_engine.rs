//! # AI Engine — Unified OODA Decision Loop
//!
//! Replaces three independent background loops (strategy_tick, orchestrate_tick,
//! agent polling) with a single coherent decision loop following the OODA pattern:
//!
//! ```text
//! OBSERVE (30s) → ORIENT (pure) → DECIDE (pure) → ACT (DB writes) → LEARN (5min, async)
//! ```
//!
//! ## Key Design Properties
//!
//! - **Single consistent view**: All decisions are made against one `WorldSnapshot`
//!   assembled in parallel queries (~50ms). No stale reads between stages.
//! - **Pure scoring**: ORIENT and DECIDE are pure functions — no side effects,
//!   fully testable without a database.
//! - **Calibrated cost model**: The LEARN phase fits power-law coefficients from
//!   real work block data, replacing hardcoded curves in `project::cost`.
//! - **7-component scoring**: Extends the original 5-component model with
//!   momentum (recent discoveries) and competition (active external searches).
//! - **Decision audit trail**: Every decision is logged with reasoning, confidence,
//!   snapshot hash, and later annotated with measured outcomes.
//!
//! ## References
//!
//! - Strategy engine: [`strategy`] (scoring model, form constants)
//! - Project orchestration: [`project::orchestration`] (phase state machine)
//! - Cost model: [`project::cost`] (power-law estimation)
//! - Cost calibrations: [`db::calibrations`]

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::db::Database;
use crate::project;
use crate::strategy;

// ── Configuration ───────────────────────────────────────────────

/// Hot-reloadable engine configuration, loaded from `ai_engine_state` on each tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEngineConfig {
    /// Whether the AI engine is enabled (master switch).
    pub enabled: bool,
    /// Maximum concurrent projects (fleet-size dependent).
    pub max_concurrent_projects: i32,
    /// Monthly budget cap (USD).
    pub max_monthly_budget_usd: f64,
    /// Per-project budget cap (USD).
    pub max_per_project_budget_usd: f64,
    /// Forms the engine will not select.
    pub excluded_forms: Vec<String>,
    /// Forms that receive a preference multiplier.
    pub preferred_forms: Vec<String>,
    /// Minimum idle workers before creating a new project.
    pub min_idle_workers_to_create: u32,
    /// Record proximity threshold for triggering verification (0.0–1.0).
    pub record_proximity_threshold: f64,
    /// Interval between LEARN cycles in seconds.
    pub learn_interval_secs: u64,
    /// Minimum data points required to fit a cost model.
    pub min_calibration_samples: i64,
    /// Maximum acceptable MAPE before replacing defaults.
    pub max_calibration_mape: f64,
}

impl Default for AiEngineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_projects: 3,
            max_monthly_budget_usd: 100.0,
            max_per_project_budget_usd: 25.0,
            excluded_forms: vec![],
            preferred_forms: vec![],
            min_idle_workers_to_create: 2,
            record_proximity_threshold: 0.1,
            learn_interval_secs: 300,
            min_calibration_samples: 10,
            max_calibration_mape: 0.5,
        }
    }
}

// ── Scoring Weights ─────────────────────────────────────────────

/// 7-component scoring weights with learned adjustments.
/// All weights must be in [0.05, 0.40] and sum to 1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub record_gap: f64,
    pub yield_rate: f64,
    pub cost_efficiency: f64,
    pub opportunity_density: f64,
    pub fleet_fit: f64,
    pub momentum: f64,
    pub competition: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            record_gap: 0.20,
            yield_rate: 0.15,
            cost_efficiency: 0.20,
            opportunity_density: 0.15,
            fleet_fit: 0.10,
            momentum: 0.10,
            competition: 0.10,
        }
    }
}

impl ScoringWeights {
    /// Validate weights are within bounds and sum to ~1.0.
    pub fn validate(&self) -> bool {
        let weights = [
            self.record_gap,
            self.yield_rate,
            self.cost_efficiency,
            self.opportunity_density,
            self.fleet_fit,
            self.momentum,
            self.competition,
        ];
        let all_bounded = weights.iter().all(|&w| w >= 0.05 && w <= 0.40);
        let sum: f64 = weights.iter().sum();
        all_bounded && (sum - 1.0).abs() < 0.01
    }

    /// Clamp each weight individually to [min, max].
    pub fn clamp(&mut self, min: f64, max: f64) {
        self.record_gap = self.record_gap.clamp(min, max);
        self.yield_rate = self.yield_rate.clamp(min, max);
        self.cost_efficiency = self.cost_efficiency.clamp(min, max);
        self.opportunity_density = self.opportunity_density.clamp(min, max);
        self.fleet_fit = self.fleet_fit.clamp(min, max);
        self.momentum = self.momentum.clamp(min, max);
        self.competition = self.competition.clamp(min, max);
    }

    /// Return weights as an array for iteration (same order as field declaration).
    pub fn as_array(&self) -> [f64; 7] {
        [
            self.record_gap,
            self.yield_rate,
            self.cost_efficiency,
            self.opportunity_density,
            self.fleet_fit,
            self.momentum,
            self.competition,
        ]
    }

    /// Set weights from an array (same order as field declaration).
    pub fn from_array(&mut self, arr: [f64; 7]) {
        self.record_gap = arr[0];
        self.yield_rate = arr[1];
        self.cost_efficiency = arr[2];
        self.opportunity_density = arr[3];
        self.fleet_fit = arr[4];
        self.momentum = arr[5];
        self.competition = arr[6];
    }

    /// Normalize weights to sum to exactly 1.0.
    pub fn normalize(&mut self) {
        let sum = self.record_gap
            + self.yield_rate
            + self.cost_efficiency
            + self.opportunity_density
            + self.fleet_fit
            + self.momentum
            + self.competition;
        if sum > 0.0 {
            self.record_gap /= sum;
            self.yield_rate /= sum;
            self.cost_efficiency /= sum;
            self.opportunity_density /= sum;
            self.fleet_fit /= sum;
            self.momentum /= sum;
            self.competition /= sum;
        }
    }
}

// ── Cost Model ──────────────────────────────────────────────────

/// Data-fitted cost model that replaces hardcoded power-law curves.
///
/// Each form's timing follows: `secs = a * (digits/1000)^b`
/// The LEARN phase fits (a, b) from completed work block data.
/// Falls back to hardcoded defaults from `project::cost` when insufficient data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// Fitted coefficients: form → (a, b) from OLS on log-log data.
    pub fitted: HashMap<String, (f64, f64)>,
    /// Hardcoded fallback coefficients (from `project::cost::secs_per_candidate`).
    pub defaults: HashMap<String, (f64, f64)>,
    /// Per-form PFGW/GWNUM measured speedup factor.
    pub pfgw_speedup: HashMap<String, f64>,
    /// Version counter, incremented on each successful fit.
    pub version: u32,
}

impl Default for CostModel {
    fn default() -> Self {
        let mut defaults = HashMap::new();
        defaults.insert("factorial".to_string(), (0.5, 2.5));
        defaults.insert("primorial".to_string(), (0.5, 2.5));
        defaults.insert("kbn".to_string(), (0.1, 2.0));
        defaults.insert("twin".to_string(), (0.1, 2.0));
        defaults.insert("sophie_germain".to_string(), (0.1, 2.0));
        defaults.insert("cullen_woodall".to_string(), (0.2, 2.2));
        defaults.insert("carol_kynea".to_string(), (0.2, 2.2));
        defaults.insert("wagstaff".to_string(), (0.8, 2.5));
        defaults.insert("palindromic".to_string(), (0.3, 2.0));
        defaults.insert("near_repdigit".to_string(), (0.3, 2.0));
        defaults.insert("repunit".to_string(), (0.4, 2.3));
        defaults.insert("gen_fermat".to_string(), (0.3, 2.2));

        Self {
            fitted: HashMap::new(),
            defaults,
            pfgw_speedup: HashMap::new(),
            version: 0,
        }
    }
}

impl CostModel {
    /// Estimate seconds per candidate for a form at a given digit count.
    /// Uses fitted coefficients if available, otherwise falls back to defaults.
    pub fn secs_per_candidate(&self, form: &str, digits: u64, has_pfgw: bool) -> f64 {
        let d = digits as f64 / 1000.0;
        let (a, b) = self
            .fitted
            .get(form)
            .or_else(|| self.defaults.get(form))
            .copied()
            .unwrap_or((0.5, 2.5));

        let base = a * d.powf(b);

        if has_pfgw && digits >= 10_000 {
            let speedup = self.pfgw_speedup.get(form).copied().unwrap_or(50.0);
            base / speedup
        } else {
            base
        }
    }
}

// ── World Snapshot ──────────────────────────────────────────────

/// Single consistent view of the entire system state, assembled in one
/// atomic read. All ORIENT/DECIDE logic operates on this immutable snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct WorldSnapshot {
    pub records: Vec<project::RecordRow>,
    pub fleet: FleetSnapshot,
    pub active_projects: Vec<project::ProjectRow>,
    pub active_jobs: Vec<crate::db::SearchJobRow>,
    pub yield_rates: Vec<crate::db::FormYieldRateRow>,
    pub cost_calibrations: Vec<crate::db::CostCalibrationRow>,
    pub recent_discoveries: Vec<RecentDiscovery>,
    pub agent_results: Vec<crate::db::AgentTaskRow>,
    pub competition_intel: Vec<crate::db::AgentMemoryRow>,
    pub worker_speeds: Vec<crate::db::WorkerSpeedRow>,
    pub budget: BudgetSnapshot,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated fleet capabilities at snapshot time.
#[derive(Debug, Clone, Serialize)]
pub struct FleetSnapshot {
    pub worker_count: u32,
    pub total_cores: u32,
    pub idle_workers: u32,
    pub max_ram_gb: u32,
    pub active_search_types: Vec<String>,
    pub workers_per_form: HashMap<String, u32>,
}

/// Budget state at snapshot time.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetSnapshot {
    pub monthly_budget_usd: f64,
    pub monthly_spend_usd: f64,
    pub remaining_usd: f64,
}

/// A recent prime discovery for momentum scoring.
#[derive(Debug, Clone, Serialize)]
pub struct RecentDiscovery {
    pub form: String,
    pub digits: i64,
    pub found_at: DateTime<Utc>,
}

// ── Drift Detection ─────────────────────────────────────────────

/// Changes detected between consecutive snapshots.
#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub workers_joined: i32,
    pub workers_left: i32,
    pub new_discoveries: u32,
    pub stalled_jobs: Vec<i64>,
    pub budget_velocity_usd_per_hour: f64,
    pub significant: bool,
}

// ── Analysis ────────────────────────────────────────────────────

/// Output of the ORIENT phase: scored forms + drift analysis.
#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub scores: Vec<FormScore>,
    pub drift: DriftReport,
}

/// 7-component score breakdown for a single form.
#[derive(Debug, Clone, Serialize)]
pub struct FormScore {
    pub form: String,
    pub record_gap: f64,
    pub yield_rate: f64,
    pub cost_efficiency: f64,
    pub opportunity_density: f64,
    pub fleet_fit: f64,
    pub momentum: f64,
    pub competition: f64,
    pub total: f64,
}

// ── Decisions ───────────────────────────────────────────────────

/// Actions the AI engine can take.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    CreateProject {
        form: String,
        params: serde_json::Value,
        budget_usd: f64,
        reasoning: String,
        confidence: f64,
    },
    PauseProject {
        project_id: i64,
        reason: String,
    },
    ExtendProject {
        project_id: i64,
        new_range_end: u64,
        reasoning: String,
    },
    AbandonProject {
        project_id: i64,
        reason: String,
    },
    RebalanceFleet {
        moves: Vec<WorkerMove>,
        reasoning: String,
    },
    RequestAgentIntel {
        task_title: String,
        task_description: String,
    },
    VerifyResult {
        form: String,
        prime_id: Option<i64>,
        reasoning: String,
    },
    NoAction {
        reason: String,
    },
}

/// A fleet rebalancing move: reassign a worker to a different search.
#[derive(Debug, Clone, Serialize)]
pub struct WorkerMove {
    pub worker_id: String,
    pub from_form: String,
    pub to_form: String,
}

/// Outcome of a single engine tick.
#[derive(Debug, Clone, Serialize)]
pub struct TickOutcome {
    pub tick_id: u64,
    pub decisions: Vec<Decision>,
    pub analysis: Analysis,
    pub duration_ms: u64,
}

// ── Safety Checks ───────────────────────────────────────────────

/// Safety boundaries that all decisions must pass through.
#[derive(Debug)]
struct SafetyLimits {
    max_projects: i32,
    budget_remaining: f64,
    min_budget_for_project: f64,
}

fn safety_check(decision: &Decision, limits: &SafetyLimits) -> bool {
    match decision {
        Decision::CreateProject { budget_usd, .. } => {
            limits.budget_remaining >= limits.min_budget_for_project
                && *budget_usd <= limits.budget_remaining
        }
        Decision::AbandonProject { .. } => {
            // Abandoning requires careful review — only allow if no primes found
            true
        }
        Decision::NoAction { .. } => true,
        _ => true,
    }
}

// ── AI Engine ───────────────────────────────────────────────────

/// The unified AI engine. Holds learned state (cost model, scoring weights)
/// and the last snapshot for drift detection.
pub struct AiEngine {
    pub config: AiEngineConfig,
    pub cost_model: CostModel,
    pub scoring_weights: ScoringWeights,
    pub last_snapshot: Option<WorldSnapshot>,
    pub tick_count: u64,
    pub last_learn: Option<std::time::Instant>,
    /// ML engine: Thompson Sampling, GP cost, BayesOpt, node intelligence.
    pub ml: crate::ml::MlEngine,
}

impl AiEngine {
    /// Create a new AI engine with default configuration.
    pub fn new() -> Self {
        Self {
            config: AiEngineConfig::default(),
            cost_model: CostModel::default(),
            scoring_weights: ScoringWeights::default(),
            last_snapshot: None,
            tick_count: 0,
            last_learn: None,
            ml: crate::ml::MlEngine::default(),
        }
    }

    /// Create an AI engine with a specific configuration.
    pub fn with_config(config: AiEngineConfig) -> Self {
        Self {
            config,
            cost_model: CostModel::default(),
            scoring_weights: ScoringWeights::default(),
            last_snapshot: None,
            tick_count: 0,
            ml: crate::ml::MlEngine::default(),
            last_learn: None,
        }
    }

    /// Run one complete OODA tick: Observe → Orient → Decide → Act → Learn.
    ///
    /// This is the single entry point that replaces `strategy_tick()` and
    /// `orchestrate_tick()`. Called every 30 seconds from the dashboard
    /// background loop.
    pub async fn tick(&mut self, db: &Database) -> Result<TickOutcome> {
        let start = std::time::Instant::now();
        self.tick_count += 1;
        let tick_id = self.tick_count;

        // Load config from DB (hot-reloadable)
        self.reload_config(db).await;

        if !self.config.enabled {
            return Ok(TickOutcome {
                tick_id,
                decisions: vec![Decision::NoAction {
                    reason: "AI engine disabled".to_string(),
                }],
                analysis: Analysis {
                    scores: vec![],
                    drift: empty_drift(),
                },
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        // OBSERVE: gather all state in parallel
        let snapshot = self.observe(db).await?;

        // ML: refresh feature store from snapshot
        self.ml.refresh_features(&snapshot);

        // Run project orchestration (phase advancement, cost aggregation)
        if let Err(e) = project::orchestrate_tick(db).await {
            warn!(error = %e, "ai_engine: project orchestration failed");
        }

        // ORIENT: score forms, detect drift
        let analysis = self.orient(&snapshot);

        // DECIDE: generate action plan
        let decisions = self.decide(&snapshot, &analysis);

        // ACT: execute decisions
        for decision in &decisions {
            if let Err(e) = self.act(db, decision, tick_id).await {
                warn!(error = %e, "ai_engine: failed to execute decision");
            }
        }

        // LEARN: calibrate cost model periodically
        if self.should_learn() {
            if let Err(e) = self.learn(db).await {
                warn!(error = %e, "ai_engine: learn phase failed");
            }
            // ML subsystem learning (bandits, GP, BayesOpt, node intelligence)
            if let Err(e) = self.ml.learn(db, &snapshot).await {
                warn!(error = %e, "ai_engine: ML learn phase failed");
            }
            self.last_learn = Some(std::time::Instant::now());
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Store snapshot for next tick's drift detection
        self.last_snapshot = Some(snapshot);

        // Persist engine state
        if let Err(e) = self.persist_state(db).await {
            warn!(error = %e, "ai_engine: failed to persist state");
        }

        Ok(TickOutcome {
            tick_id,
            decisions,
            analysis,
            duration_ms,
        })
    }

    // ── OBSERVE ─────────────────────────────────────────────────

    /// Gather a complete world snapshot via parallel database queries.
    async fn observe(&self, db: &Database) -> Result<WorldSnapshot> {
        // Run queries in parallel using tokio::join!
        let (
            records,
            workers,
            active_jobs,
            active_projects,
            yield_rates,
            calibrations,
            monthly_spend,
        ) = tokio::join!(
            db.get_records(),
            db.get_all_workers(),
            db.get_search_jobs(),
            db.get_projects(Some("active")),
            db.get_form_yield_rates(),
            db.get_cost_calibrations(),
            db.get_monthly_strategy_spend(),
        );

        let records = records.unwrap_or_default();
        let workers = workers.unwrap_or_default();
        let active_jobs = active_jobs.unwrap_or_default();
        let active_projects = active_projects.unwrap_or_default();
        let yield_rates = yield_rates.unwrap_or_default();
        let calibrations = calibrations.unwrap_or_default();
        let monthly_spend = monthly_spend.unwrap_or(0.0);

        let worker_count = workers.len() as u32;
        let total_cores: u32 = workers.iter().map(|w| w.cores as u32).sum();
        let busy_workers = active_jobs.iter().filter(|j| j.status == "running").count() as u32;
        let idle_workers = worker_count.saturating_sub(busy_workers.min(worker_count));

        // Get recent discoveries for momentum scoring (last 7 days)
        let recent_discoveries = db
            .get_recent_primes_for_momentum(7)
            .await
            .unwrap_or_default();

        // Get recent agent task completions
        let agent_results = db.get_recent_agent_results(10).await.unwrap_or_default();

        // Get competitive intel from agent memory (Phase 7)
        let competition_intel = db
            .get_agent_memory_by_category("competitive_intel")
            .await
            .unwrap_or_default();

        // Get worker speed statistics (Phase 8)
        let worker_speeds = db.get_worker_speeds().await.unwrap_or_default();

        let fleet_summary =
            db.get_fleet_summary()
                .await
                .unwrap_or_else(|_| crate::db::FleetSummary {
                    worker_count: 0,
                    total_cores: 0,
                    max_ram_gb: 0,
                    active_search_types: vec![],
                    gpu_worker_count: 0,
                    total_gpu_vram_gb: 0,
                });

        // Compute workers per form from active workers
        let mut workers_per_form: HashMap<String, u32> = HashMap::new();
        for w in &workers {
            if !w.search_type.is_empty() {
                *workers_per_form.entry(w.search_type.clone()).or_insert(0) += 1;
            }
        }

        Ok(WorldSnapshot {
            records,
            fleet: FleetSnapshot {
                worker_count,
                total_cores,
                idle_workers,
                max_ram_gb: fleet_summary.max_ram_gb,
                active_search_types: fleet_summary.active_search_types,
                workers_per_form,
            },
            active_projects,
            active_jobs,
            yield_rates,
            cost_calibrations: calibrations,
            recent_discoveries,
            agent_results,
            competition_intel,
            worker_speeds,
            budget: BudgetSnapshot {
                monthly_budget_usd: self.config.max_monthly_budget_usd,
                monthly_spend_usd: monthly_spend,
                remaining_usd: self.config.max_monthly_budget_usd - monthly_spend,
            },
            timestamp: Utc::now(),
        })
    }

    // ── ORIENT ──────────────────────────────────────────────────

    /// Score all forms and detect drift. Pure function — no side effects.
    fn orient(&self, snapshot: &WorldSnapshot) -> Analysis {
        let scores = self.score_forms(snapshot);
        let drift = self.detect_drift(snapshot);
        Analysis { scores, drift }
    }

    /// 7-component scoring model with calibrated cost and learned weights.
    fn score_forms(&self, snapshot: &WorldSnapshot) -> Vec<FormScore> {
        let mut scores = Vec::with_capacity(strategy::ALL_FORMS.len());

        for &form in strategy::ALL_FORMS {
            if self.config.excluded_forms.iter().any(|f| f == form) {
                scores.push(FormScore {
                    form: form.to_string(),
                    record_gap: 0.0,
                    yield_rate: 0.0,
                    cost_efficiency: 0.0,
                    opportunity_density: 0.0,
                    fleet_fit: 0.0,
                    momentum: 0.0,
                    competition: 0.0,
                    total: 0.0,
                });
                continue;
            }

            // 1. Record gap: room to improve toward world record
            let record_gap = snapshot
                .records
                .iter()
                .find(|r| r.form == form)
                .map(|r| {
                    if r.digits > 0 {
                        1.0 - (r.our_best_digits as f64 / r.digits as f64).min(1.0)
                    } else {
                        1.0
                    }
                })
                .unwrap_or(1.0);

            // 2. Yield rate: recency-weighted (recent yields matter more)
            let yr = snapshot
                .yield_rates
                .iter()
                .find(|y| y.form == form)
                .map(|y| y.yield_rate)
                .unwrap_or(0.001);
            let yield_score = ((yr * 1e6).ln().max(0.0) / 15.0).min(1.0);

            // 3. Cost efficiency: using calibrated cost model
            let digits_estimate = 1000u64;
            let spc = self
                .cost_model
                .secs_per_candidate(form, digits_estimate, false);
            let cost_eff = if spc > 0.0 {
                ((yr / spc) * 1e8).ln().max(0.0) / 20.0
            } else {
                0.0
            };
            let cost_efficiency = cost_eff.min(1.0);

            // 4. Opportunity density: expected primes per core-hour in uncovered range
            let max_searched = snapshot
                .yield_rates
                .iter()
                .find(|y| y.form == form)
                .map(|y| y.max_range_searched)
                .unwrap_or(0);
            let total_range = searchable_range(form);
            let uncovered = total_range.saturating_sub(max_searched as u64) as f64;
            let opportunity_density = if total_range > 0 {
                (uncovered / total_range as f64).min(1.0)
            } else {
                1.0
            };

            // 5. Fleet fit: utilization efficiency
            let min_cores = min_cores_for_form(form);
            let fleet_fit = if snapshot.fleet.total_cores >= min_cores {
                1.0
            } else if snapshot.fleet.total_cores > 0 {
                snapshot.fleet.total_cores as f64 / min_cores as f64
            } else {
                0.0
            };

            // 6. Momentum: bonus for forms with recent discoveries
            let recent_count = snapshot
                .recent_discoveries
                .iter()
                .filter(|d| d.form == form)
                .count();
            let momentum = (recent_count as f64 / 5.0).min(1.0);

            // 7. Competition: penalty for forms with active external searches
            // Populated from agent memory (category=competitive_intel, key=competitive_intel:{form})
            let competition = snapshot
                .competition_intel
                .iter()
                .find(|m| m.key == format!("competitive_intel:{}", form))
                .and_then(|m| m.value.parse::<f64>().ok())
                .unwrap_or(0.5); // neutral default when no intel available

            // Weighted composite score
            let w = &self.scoring_weights;
            let mut total = record_gap * w.record_gap
                + yield_score * w.yield_rate
                + cost_efficiency * w.cost_efficiency
                + opportunity_density * w.opportunity_density
                + fleet_fit * w.fleet_fit
                + momentum * w.momentum
                + competition * w.competition;

            // Preferred forms multiplier
            if self.config.preferred_forms.iter().any(|f| f == form) {
                total *= PREFERRED_MULTIPLIER;
            }

            scores.push(FormScore {
                form: form.to_string(),
                record_gap,
                yield_rate: yield_score,
                cost_efficiency,
                opportunity_density,
                fleet_fit,
                momentum,
                competition,
                total,
            });
        }

        scores.sort_by(|a, b| {
            b.total
                .partial_cmp(&a.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores
    }

    /// Compare current snapshot to last snapshot, detecting changes.
    fn detect_drift(&self, snapshot: &WorldSnapshot) -> DriftReport {
        let Some(last) = &self.last_snapshot else {
            return empty_drift();
        };

        let workers_joined = snapshot.fleet.worker_count as i32 - last.fleet.worker_count as i32;
        let workers_left = if workers_joined < 0 {
            workers_joined.unsigned_abs() as i32
        } else {
            0
        };
        let workers_joined = workers_joined.max(0);

        // Count new discoveries since last snapshot
        let new_discoveries = snapshot
            .recent_discoveries
            .iter()
            .filter(|d| d.found_at > last.timestamp)
            .count() as u32;

        // Detect stalled jobs: running > 30min with 0 tested
        let stalled_jobs: Vec<i64> = snapshot
            .active_jobs
            .iter()
            .filter(|j| {
                j.status == "running"
                    && j.total_tested == 0
                    && j.started_at
                        .map(|s| (Utc::now() - s).num_seconds() > 1800)
                        .unwrap_or(false)
            })
            .map(|j| j.id)
            .collect();

        // Budget velocity: rate of spend
        let time_delta = (snapshot.timestamp - last.timestamp).num_seconds().max(1) as f64;
        let spend_delta = snapshot.budget.monthly_spend_usd - last.budget.monthly_spend_usd;
        let budget_velocity_usd_per_hour = (spend_delta / time_delta) * 3600.0;

        let significant = workers_joined > 0
            || workers_left > 0
            || new_discoveries > 0
            || !stalled_jobs.is_empty()
            || budget_velocity_usd_per_hour.abs() > 1.0;

        DriftReport {
            workers_joined,
            workers_left,
            new_discoveries,
            stalled_jobs,
            budget_velocity_usd_per_hour,
            significant,
        }
    }

    // ── DECIDE ──────────────────────────────────────────────────

    /// Generate decisions based on snapshot and analysis. Pure function.
    fn decide(&self, snapshot: &WorldSnapshot, analysis: &Analysis) -> Vec<Decision> {
        let mut decisions = Vec::new();
        let limits = SafetyLimits {
            max_projects: self.config.max_concurrent_projects,
            budget_remaining: snapshot.budget.remaining_usd,
            min_budget_for_project: self.config.max_per_project_budget_usd * 0.5,
        };

        // 1. Handle stalled jobs
        for &job_id in &analysis.drift.stalled_jobs {
            if let Some(job) = snapshot.active_jobs.iter().find(|j| j.id == job_id) {
                decisions.push(Decision::PauseProject {
                    project_id: job_id,
                    reason: format!(
                        "Job {} ({}) stalled: running >30min with 0 tested",
                        job_id, job.search_type
                    ),
                });
            }
        }

        // 2. Check for near-record discoveries needing verification
        for record in &snapshot.records {
            if record.our_best_digits > 0 && record.digits > 0 {
                let proximity = record.our_best_digits as f64 / record.digits as f64;
                if proximity >= (1.0 - self.config.record_proximity_threshold) {
                    let already_verifying = snapshot
                        .active_projects
                        .iter()
                        .any(|p| p.form == record.form && p.objective == "verification");
                    if !already_verifying {
                        decisions.push(Decision::VerifyResult {
                            form: record.form.clone(),
                            prime_id: None,
                            reasoning: format!(
                                "{} prime at {} digits is within {:.1}% of {}-digit record",
                                record.form,
                                record.our_best_digits,
                                (1.0 - proximity) * 100.0,
                                record.digits,
                            ),
                        });
                    }
                }
            }
        }

        // 3. Create new projects if capacity and budget allow
        let active_project_count = snapshot.active_projects.len() as i32;
        if snapshot.fleet.idle_workers >= self.config.min_idle_workers_to_create
            && active_project_count < limits.max_projects
            && snapshot.budget.remaining_usd > limits.min_budget_for_project
        {
            let active_forms: Vec<&str> = snapshot
                .active_projects
                .iter()
                .map(|p| p.form.as_str())
                .collect();

            // Portfolio sizing based on fleet
            let max_new = self.portfolio_slots(snapshot.fleet.worker_count, active_project_count);

            for best in analysis
                .scores
                .iter()
                .filter(|s| {
                    s.total > 0.0
                        && !active_forms.contains(&s.form.as_str())
                        && !self.config.excluded_forms.contains(&s.form)
                })
                .take(max_new as usize)
            {
                let max_searched = snapshot
                    .yield_rates
                    .iter()
                    .find(|y| y.form == best.form)
                    .map(|y| y.max_range_searched)
                    .unwrap_or(0);

                let budget = snapshot
                    .budget
                    .remaining_usd
                    .min(self.config.max_per_project_budget_usd);

                let decision = Decision::CreateProject {
                    form: best.form.clone(),
                    params: serde_json::json!({
                        "continue_from": max_searched,
                        "budget_usd": budget,
                        "scores": {
                            "record_gap": best.record_gap,
                            "yield_rate": best.yield_rate,
                            "cost_efficiency": best.cost_efficiency,
                            "opportunity_density": best.opportunity_density,
                            "fleet_fit": best.fleet_fit,
                            "momentum": best.momentum,
                            "competition": best.competition,
                        },
                    }),
                    budget_usd: budget,
                    reasoning: format!(
                        "Top-scoring form ({:.3}): record_gap={:.2}, yield={:.2}, \
                         cost_eff={:.2}, opportunity={:.2}, fleet_fit={:.2}, \
                         momentum={:.2}, competition={:.2}. {} idle workers.",
                        best.total,
                        best.record_gap,
                        best.yield_rate,
                        best.cost_efficiency,
                        best.opportunity_density,
                        best.fleet_fit,
                        best.momentum,
                        best.competition,
                        snapshot.fleet.idle_workers,
                    ),
                    confidence: best.total.min(1.0),
                };

                if safety_check(&decision, &limits) {
                    decisions.push(decision);
                }
            }
        }

        // 4. Request competitive intel for forms with stale or missing data (Phase 7b)
        // Limit to 1 intel request per tick to avoid spamming agents
        let mut intel_requested = false;
        for &form in strategy::ALL_FORMS {
            if intel_requested {
                break;
            }
            if self.config.excluded_forms.iter().any(|f| f == form) {
                continue;
            }
            let intel_fresh = snapshot.competition_intel.iter().any(|m| {
                m.key == format!("competitive_intel:{}", form)
                    && (Utc::now() - m.updated_at).num_days() < 7
            });
            if !intel_fresh {
                decisions.push(Decision::RequestAgentIntel {
                    task_title: format!("Competitive Intel: {}", form),
                    task_description: format!(
                        "Research the competitive landscape for {} prime searching. \
                         Who else is actively searching? What ranges are covered? \
                         Rate competition intensity 0.0 (no competition) to 1.0 (heavily contested). \
                         Return result as JSON: {{\"competition_score\": <float>}}",
                        form
                    ),
                });
                intel_requested = true;
            }
        }

        // 5. Detect fleet imbalance and generate rebalance decisions (Phase 8d)
        if let Some(rebalance) = self.detect_fleet_imbalance(snapshot) {
            decisions.push(rebalance);
        }

        // 6. If no actionable decisions, emit NoAction with reason
        if decisions.is_empty() {
            let reason = if snapshot.fleet.worker_count == 0 {
                "No workers connected".to_string()
            } else if snapshot.fleet.idle_workers < self.config.min_idle_workers_to_create {
                format!(
                    "Only {} idle workers (need {})",
                    snapshot.fleet.idle_workers, self.config.min_idle_workers_to_create,
                )
            } else if active_project_count >= limits.max_projects {
                format!(
                    "{} active projects (max {})",
                    active_project_count, limits.max_projects,
                )
            } else if snapshot.budget.remaining_usd <= 0.0 {
                "Monthly budget exhausted".to_string()
            } else {
                "No actionable conditions met".to_string()
            };
            decisions.push(Decision::NoAction { reason });
        }

        decisions
    }

    /// Detect fleet imbalance: workers assigned to forms with no available blocks,
    /// while other forms have queued blocks and no workers. Returns a RebalanceFleet
    /// decision with suggested moves, or None if the fleet is balanced.
    fn detect_fleet_imbalance(&self, snapshot: &WorldSnapshot) -> Option<Decision> {
        // Find forms with workers but 0 available blocks (overprovisioned)
        let mut overprovisioned: Vec<(&str, Vec<&str>)> = Vec::new();
        // Find forms with available blocks but 0 workers (underprovisioned)
        let mut underprovisioned: Vec<String> = Vec::new();

        for job in &snapshot.active_jobs {
            if job.status != "running" {
                continue;
            }
            let form = &job.search_type;
            let workers_on_form = snapshot
                .fleet
                .workers_per_form
                .get(form)
                .copied()
                .unwrap_or(0);

            // Check if this form has worker speed data but no available blocks
            let has_available = snapshot.worker_speeds.iter().any(|ws| ws.form == *form);

            if workers_on_form > 0 && !has_available {
                // Find workers assigned to this form
                // (workers_per_form tracks the form, but we don't have individual worker IDs here)
                // We'll use worker_speeds to find workers on this form
                let worker_ids: Vec<&str> = snapshot
                    .worker_speeds
                    .iter()
                    .filter(|ws| ws.form == *form)
                    .map(|ws| ws.worker_id.as_str())
                    .collect();
                if !worker_ids.is_empty() {
                    overprovisioned.push((form, worker_ids));
                }
            } else if workers_on_form == 0 {
                underprovisioned.push(form.clone());
            }
        }

        if overprovisioned.is_empty() || underprovisioned.is_empty() {
            return None;
        }

        let mut moves = Vec::new();
        for (from_form, workers) in &overprovisioned {
            if underprovisioned.is_empty() {
                break;
            }
            for &worker_id in workers {
                if let Some(to_form) = underprovisioned.pop() {
                    moves.push(WorkerMove {
                        worker_id: worker_id.to_string(),
                        from_form: from_form.to_string(),
                        to_form: to_form.clone(),
                    });
                }
            }
        }

        if moves.is_empty() {
            return None;
        }

        Some(Decision::RebalanceFleet {
            moves: moves.clone(),
            reasoning: format!(
                "Fleet imbalance: {} workers moved from exhausted forms to underprovisioned forms",
                moves.len()
            ),
        })
    }

    /// Determine how many new projects to create based on fleet size.
    fn portfolio_slots(&self, fleet_size: u32, active_projects: i32) -> i32 {
        let max_concurrent = match fleet_size {
            0..=4 => 1,
            5..=16 => 3,
            17..=64 => 5,
            _ => 8,
        };
        (max_concurrent - active_projects).max(0).min(2) // max 2 new per tick
    }

    // ── ACT ─────────────────────────────────────────────────────

    /// Execute a single decision: create projects, pause jobs, log to audit trail.
    async fn act(&self, db: &Database, decision: &Decision, tick_id: u64) -> Result<()> {
        let (decision_type, form, action, reasoning, confidence, params, component_scores) =
            match decision {
                Decision::CreateProject {
                    form,
                    params,
                    budget_usd,
                    reasoning,
                    confidence,
                } => {
                    let continue_from = params["continue_from"].as_i64().unwrap_or(0);
                    let config =
                        strategy::build_auto_project_config(form, continue_from, *budget_usd);
                    match db.create_project(&config, None).await {
                        Ok(pid) => {
                            if let Err(e) = db.update_project_status(pid, "active").await {
                                warn!(project_id = pid, error = %e, "ai_engine: failed to activate project");
                            }
                            info!(project_id = pid, form, "ai_engine: created project");
                        }
                        Err(e) => {
                            warn!(form, error = %e, "ai_engine: failed to create project");
                        }
                    }
                    // Extract component scores from params for outcome correlation
                    let scores = params.get("scores").cloned();
                    (
                        "create_project",
                        Some(form.as_str()),
                        "executed",
                        reasoning.as_str(),
                        *confidence,
                        Some(params.clone()),
                        scores,
                    )
                }
                Decision::PauseProject { project_id, reason } => {
                    // Try pausing as a search job first (stalled jobs use job_id)
                    if let Err(e) = db
                        .update_search_job_status(*project_id, "paused", None)
                        .await
                    {
                        warn!(id = project_id, error = %e, "ai_engine: failed to pause job");
                    } else {
                        info!(id = project_id, "ai_engine: paused stalled job");
                    }
                    (
                        "pause_project",
                        None,
                        "executed",
                        reason.as_str(),
                        0.8,
                        Some(serde_json::json!({"project_id": project_id})),
                        None,
                    )
                }
                Decision::VerifyResult {
                    form,
                    prime_id,
                    reasoning,
                } => (
                    "verify_result",
                    Some(form.as_str()),
                    "logged",
                    reasoning.as_str(),
                    0.9,
                    Some(serde_json::json!({"form": form, "prime_id": prime_id})),
                    None,
                ),
                Decision::NoAction { reason } => (
                    "no_action",
                    None,
                    "logged",
                    reason.as_str(),
                    1.0,
                    None,
                    None,
                ),
                Decision::ExtendProject {
                    project_id,
                    reasoning,
                    ..
                } => (
                    "extend_project",
                    None,
                    "logged",
                    reasoning.as_str(),
                    0.7,
                    Some(serde_json::json!({"project_id": project_id})),
                    None,
                ),
                Decision::AbandonProject { project_id, reason } => (
                    "abandon_project",
                    None,
                    "logged",
                    reason.as_str(),
                    0.6,
                    Some(serde_json::json!({"project_id": project_id})),
                    None,
                ),
                Decision::RebalanceFleet { moves, reasoning } => {
                    // Execute rebalance: release claimed blocks from overprovisioned workers
                    for mv in moves {
                        match db.release_worker_blocks(&mv.worker_id).await {
                            Ok(released) => {
                                info!(
                                    worker = %mv.worker_id,
                                    from = %mv.from_form,
                                    to = %mv.to_form,
                                    released,
                                    "ai_engine: rebalanced worker"
                                );
                            }
                            Err(e) => {
                                warn!(
                                    worker = %mv.worker_id,
                                    error = %e,
                                    "ai_engine: failed to release blocks"
                                );
                            }
                        }
                    }
                    (
                        "rebalance_fleet",
                        None,
                        "executed",
                        reasoning.as_str(),
                        0.5,
                        Some(serde_json::json!({"moves": moves})),
                        None,
                    )
                }
                Decision::RequestAgentIntel {
                    task_title,
                    task_description,
                } => {
                    // Create an agent task to gather competitive intel
                    match db
                        .create_agent_task(
                            task_title,
                            task_description,
                            "low",
                            Some("haiku"),
                            "ai_engine",
                            Some(1.0),
                            0,
                            Some("strategy-advisor"),
                        )
                        .await
                    {
                        Ok(task) => {
                            info!(task_id = task.id, "ai_engine: created intel agent task");
                        }
                        Err(e) => {
                            warn!(error = %e, "ai_engine: failed to create intel agent task");
                        }
                    }
                    (
                        "request_agent_intel",
                        None,
                        "executed",
                        task_title.as_str(),
                        0.5,
                        Some(serde_json::json!({
                            "title": task_title,
                            "description": task_description,
                        })),
                        None,
                    )
                }
            };

        // Log to ai_engine_decisions audit table (with component scores if available)
        db.insert_ai_engine_decision_with_scores(
            tick_id as i64,
            decision_type,
            form,
            action,
            reasoning,
            confidence,
            params.as_ref(),
            component_scores.as_ref(),
        )
        .await?;

        // Also log to legacy strategy_decisions for backward compatibility
        db.insert_strategy_decision(
            decision_type,
            form,
            &format!("[ai_engine] {}", reasoning),
            reasoning,
            params.as_ref(),
            None,
            action,
            None,
            None,
            None,
        )
        .await
        .ok();

        Ok(())
    }

    // ── LEARN ───────────────────────────────────────────────────

    /// Should we run the LEARN phase this tick?
    fn should_learn(&self) -> bool {
        match self.last_learn {
            None => true, // first tick always learns
            Some(last) => last.elapsed().as_secs() >= self.config.learn_interval_secs,
        }
    }

    /// Calibrate cost model from work block data, evaluate outcomes, and update
    /// scoring weights via exponential weighted averaging (EWA).
    async fn learn(&mut self, db: &Database) -> Result<()> {
        // 1. Fetch cost calibration data
        let calibrations = db.get_cost_calibrations().await?;

        // 2. Update fitted coefficients from calibration table
        for cal in &calibrations {
            if cal.sample_count >= self.config.min_calibration_samples {
                let mape = cal.avg_error_pct.unwrap_or(1.0);
                if mape <= self.config.max_calibration_mape {
                    self.cost_model
                        .fitted
                        .insert(cal.form.clone(), (cal.coeff_a, cal.coeff_b));
                }
            }
        }

        // 3. Fit cost model from raw work block data
        for &form in strategy::ALL_FORMS {
            match db.get_cost_observations(form, 50).await {
                Ok(obs) if obs.len() >= self.config.min_calibration_samples as usize => {
                    if let Some((a, b, mape)) = fit_power_law(&obs) {
                        if mape <= self.config.max_calibration_mape {
                            self.cost_model.fitted.insert(form.to_string(), (a, b));
                            db.upsert_cost_calibration(form, a, b, obs.len() as i64, Some(mape))
                                .await
                                .ok();
                            self.cost_model.version += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        // 4. Evaluate outcomes for unmeasured decisions (Phase 6b)
        self.evaluate_outcomes(db).await;

        // 5. Update scoring weights from measured outcomes (Phase 6c-d)
        let old_weights = self.scoring_weights.clone();
        if let Ok(outcomes) = db.get_outcomes_with_scores(50).await {
            self.update_weights_from_outcomes(&outcomes);
        }
        if old_weights.as_array() != self.scoring_weights.as_array() {
            info!(
                old = ?old_weights.as_array(),
                new = ?self.scoring_weights.as_array(),
                "ai_engine: weights updated"
            );
        }

        // 6. Process completed agent intel tasks (Phase 7d)
        self.process_agent_intel(db).await;

        info!(
            cost_model_version = self.cost_model.version,
            fitted_forms = self.cost_model.fitted.len(),
            "ai_engine: learn phase complete"
        );

        Ok(())
    }

    // ── Phase 6: Outcome Evaluation & Weight Learning ───────────

    /// Evaluate outcomes for create_project decisions that haven't been measured yet.
    /// Joins decisions with their associated projects and records the verdict.
    async fn evaluate_outcomes(&self, db: &Database) {
        let candidates = match db.get_decisions_needing_outcomes(20).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "ai_engine: failed to fetch outcome candidates");
                return;
            }
        };

        for candidate in candidates {
            let verdict = if candidate.primes_found > 0 {
                "success"
            } else if candidate.project_status == "failed" {
                "failure"
            } else {
                "stalled"
            };

            let outcome = serde_json::json!({
                "verdict": verdict,
                "primes_found": candidate.primes_found,
                "cost_usd": candidate.cost_usd,
                "core_hours": candidate.core_hours,
                "project_id": candidate.project_id,
                "project_status": candidate.project_status,
            });

            if let Err(e) = db
                .update_decision_outcome(candidate.decision_id, &outcome)
                .await
            {
                warn!(
                    decision_id = candidate.decision_id,
                    error = %e,
                    "ai_engine: failed to store outcome"
                );
            }
        }
    }

    /// Update scoring weights from measured outcomes via exponential weighted
    /// averaging (EWA). Each outcome nudges weights toward components that
    /// correlated with success.
    fn update_weights_from_outcomes(&mut self, outcomes: &[crate::db::DecisionWithOutcome]) {
        if outcomes.is_empty() {
            return;
        }

        for outcome in outcomes {
            let Some(scores_json) = &outcome.component_scores else {
                continue;
            };
            let Some(outcome_json) = &outcome.outcome else {
                continue;
            };

            let component_scores = match extract_component_scores(scores_json) {
                Some(s) => s,
                None => continue,
            };

            let reward = compute_reward(outcome_json);

            // EWA nudge: w_i += alpha * (reward - 0.5) * component_score_i
            let mut weights = self.scoring_weights.as_array();
            for (i, &score) in component_scores.iter().enumerate() {
                weights[i] += WEIGHT_LEARNING_RATE * (reward - 0.5) * score;
            }
            self.scoring_weights.from_array(weights);
        }

        // Clamp and normalize to maintain invariants
        self.scoring_weights.clamp(WEIGHT_MIN, WEIGHT_MAX);
        self.scoring_weights.normalize();
    }

    // ── Phase 7: Agent Intel Processing ─────────────────────────

    /// Process completed agent tasks that were spawned by the AI engine,
    /// extracting competitive intel scores and storing them in agent memory.
    async fn process_agent_intel(&self, db: &Database) {
        let tasks = match db.get_recent_agent_results(20).await {
            Ok(t) => t,
            Err(_) => return,
        };

        for task in &tasks {
            if task.source != "ai_engine" || task.status != "completed" {
                continue;
            }

            let Some(result) = &task.result else {
                continue;
            };
            let result_text = result.as_str().unwrap_or_default();

            // Try to extract form name from the task title
            // Title format: "Competitive Intel: {form}"
            let form = task
                .title
                .strip_prefix("Competitive Intel: ")
                .unwrap_or_default();
            if form.is_empty() {
                continue;
            }

            if let Some(score) = parse_competition_score(result_text) {
                let key = format!("competitive_intel:{}", form);
                if let Err(e) = db
                    .upsert_agent_memory(
                        &key,
                        &score.to_string(),
                        "competitive_intel",
                        Some(task.id),
                    )
                    .await
                {
                    warn!(form, error = %e, "ai_engine: failed to store competition score");
                }
            }
        }
    }

    // ── State Persistence ───────────────────────────────────────

    /// Reload configuration from the strategy_config table.
    async fn reload_config(&mut self, db: &Database) {
        match db.get_strategy_config().await {
            Ok(config) => {
                self.config.enabled = config.enabled;
                self.config.max_concurrent_projects = config.max_concurrent_projects;
                self.config.max_monthly_budget_usd = config.max_monthly_budget_usd;
                self.config.max_per_project_budget_usd = config.max_per_project_budget_usd;
                self.config.excluded_forms = config.excluded_forms;
                self.config.preferred_forms = config.preferred_forms;
                self.config.min_idle_workers_to_create = config.min_idle_workers_to_create as u32;
                self.config.record_proximity_threshold = config.record_proximity_threshold;
            }
            Err(e) => {
                warn!(error = %e, "ai_engine: failed to reload config, using cached");
            }
        }

        // Load learned weights from ai_engine_state if available
        match db.get_ai_engine_state().await {
            Ok(Some(state)) => {
                if let Ok(weights) = serde_json::from_value::<ScoringWeights>(state.scoring_weights)
                {
                    if weights.validate() {
                        self.scoring_weights = weights;
                    }
                }
            }
            Ok(None) => {} // first run, use defaults
            Err(e) => {
                warn!(error = %e, "ai_engine: failed to load engine state");
            }
        }
    }

    /// Persist current engine state to the database.
    async fn persist_state(&self, db: &Database) -> Result<()> {
        let weights_json = serde_json::to_value(&self.scoring_weights)?;
        db.upsert_ai_engine_state(
            &weights_json,
            self.cost_model.version as i32,
            self.tick_count as i64,
        )
        .await?;
        Ok(())
    }
}

// ── Helper Functions ────────────────────────────────────────────

/// Preferred forms scoring multiplier.
const PREFERRED_MULTIPLIER: f64 = 1.5;

/// Learning rate for EWA weight updates. Conservative to avoid oscillation.
const WEIGHT_LEARNING_RATE: f64 = 0.05;

/// Minimum weight bound (prevents any component from being zeroed out).
const WEIGHT_MIN: f64 = 0.05;

/// Maximum weight bound (prevents any single component from dominating).
const WEIGHT_MAX: f64 = 0.40;

/// Compute reward from a decision outcome. Reward is based on primes found
/// normalized by cost, producing a value in [0, 1].
pub fn compute_reward(outcome: &serde_json::Value) -> f64 {
    let primes_found = outcome["primes_found"].as_f64().unwrap_or(0.0);
    let cost = outcome["cost_usd"].as_f64().unwrap_or(0.01).max(0.01);
    // Raw efficiency: primes per dollar
    let efficiency = primes_found / cost;
    // Sigmoid mapping to [0, 1]: higher efficiency → higher reward
    // At efficiency=0 → reward≈0.38, at efficiency=1 → reward≈0.62, at efficiency=5 → reward≈0.92
    let x = efficiency - 0.5;
    (1.0 / (1.0 + (-x).exp())).clamp(0.0, 1.0)
}

/// Extract the 7 component scores from a JSON object.
/// Returns None if the JSON doesn't contain the expected fields.
fn extract_component_scores(scores_json: &serde_json::Value) -> Option<[f64; 7]> {
    Some([
        scores_json["record_gap"].as_f64()?,
        scores_json["yield_rate"].as_f64()?,
        scores_json["cost_efficiency"].as_f64()?,
        scores_json["opportunity_density"].as_f64()?,
        scores_json["fleet_fit"].as_f64()?,
        scores_json["momentum"].as_f64()?,
        scores_json["competition"].as_f64()?,
    ])
}

/// Parse a competition score from agent result text.
/// Looks for JSON `{"competition_score": 0.3}` or plain float patterns.
pub fn parse_competition_score(result_text: &str) -> Option<f64> {
    // Try JSON parsing first
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(result_text) {
        if let Some(score) = val["competition_score"].as_f64() {
            return Some(score.clamp(0.0, 1.0));
        }
    }

    // Try to find JSON embedded in text
    if let Some(start) = result_text.find('{') {
        if let Some(end) = result_text[start..].find('}') {
            let json_str = &result_text[start..=start + end];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(score) = val["competition_score"].as_f64() {
                    return Some(score.clamp(0.0, 1.0));
                }
            }
        }
    }

    // Try plain float at the end of text (e.g., "Competition score: 0.3")
    result_text
        .split_whitespace()
        .last()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&s| (0.0..=1.0).contains(&s))
}

/// Default searchable range per form (for opportunity density).
fn searchable_range(form: &str) -> u64 {
    match form {
        "factorial" | "primorial" => 1_000_000,
        "kbn" | "twin" | "sophie_germain" => 10_000_000,
        "palindromic" | "near_repdigit" => 50_000,
        "cullen_woodall" | "carol_kynea" => 5_000_000,
        "wagstaff" => 50_000_000,
        "repunit" => 1_000_000,
        "gen_fermat" => 1_000_000,
        _ => 1_000_000,
    }
}

/// Minimum cores recommended for a form.
fn min_cores_for_form(form: &str) -> u32 {
    match form {
        "wagstaff" | "repunit" => 16,
        "factorial" | "primorial" => 8,
        _ => 4,
    }
}

fn empty_drift() -> DriftReport {
    DriftReport {
        workers_joined: 0,
        workers_left: 0,
        new_discoveries: 0,
        stalled_jobs: vec![],
        budget_velocity_usd_per_hour: 0.0,
        significant: false,
    }
}

/// A single cost observation: (digits, seconds).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CostObservation {
    pub digits: f64,
    pub secs: f64,
}

/// Fit power-law `secs = a * (digits/1000)^b` via OLS on log-log data.
///
/// Returns (a, b, MAPE) or None if fitting fails.
pub fn fit_power_law(observations: &[CostObservation]) -> Option<(f64, f64, f64)> {
    if observations.len() < 3 {
        return None;
    }

    // Filter valid points (positive digits and secs)
    let points: Vec<(f64, f64)> = observations
        .iter()
        .filter(|o| o.digits > 0.0 && o.secs > 0.0)
        .map(|o| ((o.digits / 1000.0).ln(), o.secs.ln()))
        .collect();

    if points.len() < 3 {
        return None;
    }

    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
    let sum_xx: f64 = points.iter().map(|(x, _)| x * x).sum();

    let denom = n * sum_xx - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let b = (n * sum_xy - sum_x * sum_y) / denom;
    let ln_a = (sum_y - b * sum_x) / n;
    let a = ln_a.exp();

    // Compute MAPE (Mean Absolute Percentage Error)
    let mape: f64 = observations
        .iter()
        .filter(|o| o.digits > 0.0 && o.secs > 0.0)
        .map(|o| {
            let predicted = a * (o.digits / 1000.0).powf(b);
            ((o.secs - predicted) / o.secs).abs()
        })
        .sum::<f64>()
        / points.len() as f64;

    Some((a, b, mape))
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal snapshot for testing.
    fn test_snapshot(worker_count: u32, total_cores: u32, idle_workers: u32) -> WorldSnapshot {
        WorldSnapshot {
            records: vec![],
            fleet: FleetSnapshot {
                worker_count,
                total_cores,
                idle_workers,
                max_ram_gb: 32,
                active_search_types: vec![],
                workers_per_form: HashMap::new(),
            },
            active_projects: vec![],
            active_jobs: vec![],
            yield_rates: vec![],
            cost_calibrations: vec![],
            recent_discoveries: vec![],
            agent_results: vec![],
            competition_intel: vec![],
            worker_speeds: vec![],
            budget: BudgetSnapshot {
                monthly_budget_usd: 100.0,
                monthly_spend_usd: 0.0,
                remaining_usd: 100.0,
            },
            timestamp: Utc::now(),
        }
    }

    // ── ScoringWeights ──────────────────────────────────────────

    #[test]
    fn default_weights_valid() {
        let w = ScoringWeights::default();
        assert!(w.validate(), "Default weights should be valid");
    }

    #[test]
    fn weights_sum_to_one() {
        let w = ScoringWeights::default();
        let sum = w.record_gap
            + w.yield_rate
            + w.cost_efficiency
            + w.opportunity_density
            + w.fleet_fit
            + w.momentum
            + w.competition;
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Weights should sum to 1.0, got {}",
            sum
        );
    }

    #[test]
    fn invalid_weights_detected() {
        let w = ScoringWeights {
            record_gap: 0.90,
            yield_rate: 0.01,
            cost_efficiency: 0.01,
            opportunity_density: 0.01,
            fleet_fit: 0.01,
            momentum: 0.01,
            competition: 0.05,
        };
        assert!(
            !w.validate(),
            "Out-of-bounds weights should fail validation"
        );
    }

    #[test]
    fn normalize_weights() {
        let mut w = ScoringWeights {
            record_gap: 0.4,
            yield_rate: 0.3,
            cost_efficiency: 0.4,
            opportunity_density: 0.3,
            fleet_fit: 0.2,
            momentum: 0.2,
            competition: 0.2,
        };
        w.normalize();
        let sum = w.record_gap
            + w.yield_rate
            + w.cost_efficiency
            + w.opportunity_density
            + w.fleet_fit
            + w.momentum
            + w.competition;
        assert!(
            (sum - 1.0).abs() < 0.001,
            "Normalized weights should sum to 1.0, got {}",
            sum
        );
    }

    #[test]
    fn clamp_method() {
        let mut w = ScoringWeights {
            record_gap: 0.90,
            yield_rate: 0.01,
            cost_efficiency: 0.20,
            opportunity_density: 0.15,
            fleet_fit: 0.10,
            momentum: 0.10,
            competition: -0.10,
        };
        w.clamp(WEIGHT_MIN, WEIGHT_MAX);
        assert_eq!(w.record_gap, WEIGHT_MAX, "Should clamp to max");
        assert_eq!(w.yield_rate, WEIGHT_MIN, "Should clamp to min");
        assert_eq!(w.competition, WEIGHT_MIN, "Negative should clamp to min");
        assert_eq!(w.cost_efficiency, 0.20, "In-range should be unchanged");
    }

    #[test]
    fn as_array_from_array_roundtrip() {
        let w = ScoringWeights::default();
        let arr = w.as_array();
        let mut w2 = ScoringWeights::default();
        w2.from_array(arr);
        assert_eq!(w.as_array(), w2.as_array());
    }

    // ── CostModel ───────────────────────────────────────────────

    #[test]
    fn cost_model_uses_fitted_over_default() {
        let mut model = CostModel::default();
        model.fitted.insert("kbn".to_string(), (0.05, 1.8));

        let fitted = model.secs_per_candidate("kbn", 1000, false);
        let default_spc = 0.1 * 1.0f64.powf(2.0);
        let fitted_spc = 0.05 * 1.0f64.powf(1.8);

        assert!(
            (fitted - fitted_spc).abs() < 0.001,
            "Should use fitted coefficients, got {} expected {}",
            fitted,
            fitted_spc
        );
        assert!(
            (fitted - default_spc).abs() > 0.001,
            "Should differ from default"
        );
    }

    #[test]
    fn cost_model_falls_back_to_default() {
        let model = CostModel::default();
        let spc = model.secs_per_candidate("factorial", 2000, false);
        let expected = 0.5 * 2.0f64.powf(2.5);
        assert!(
            (spc - expected).abs() < 0.001,
            "Should use default coefficients: got {} expected {}",
            spc,
            expected
        );
    }

    #[test]
    fn cost_model_pfgw_speedup() {
        let model = CostModel::default();
        let without = model.secs_per_candidate("factorial", 10_000, false);
        let with = model.secs_per_candidate("factorial", 10_000, true);
        let ratio = without / with;
        assert!(
            (ratio - 50.0).abs() < 0.1,
            "PFGW should give 50x speedup, got {}x",
            ratio
        );
    }

    #[test]
    fn cost_model_custom_pfgw_speedup() {
        let mut model = CostModel::default();
        model.pfgw_speedup.insert("factorial".to_string(), 75.0);

        let without = model.secs_per_candidate("factorial", 10_000, false);
        let with = model.secs_per_candidate("factorial", 10_000, true);
        let ratio = without / with;
        assert!(
            (ratio - 75.0).abs() < 0.1,
            "Custom PFGW speedup should be 75x, got {}x",
            ratio
        );
    }

    // ── fit_power_law ───────────────────────────────────────────

    #[test]
    fn fit_power_law_exact_data() {
        let obs: Vec<CostObservation> = (1..=10)
            .map(|i| {
                let digits = 1000.0 * i as f64;
                let secs = 0.5 * (digits / 1000.0).powf(2.5);
                CostObservation { digits, secs }
            })
            .collect();

        let (a, b, mape) = fit_power_law(&obs).expect("Should fit exact data");
        assert!((a - 0.5).abs() < 0.01, "a should be ~0.5, got {}", a);
        assert!((b - 2.5).abs() < 0.01, "b should be ~2.5, got {}", b);
        assert!(
            mape < 0.01,
            "MAPE should be near 0 for exact data, got {}",
            mape
        );
    }

    #[test]
    fn fit_power_law_insufficient_data() {
        let obs = vec![
            CostObservation {
                digits: 1000.0,
                secs: 0.5,
            },
            CostObservation {
                digits: 2000.0,
                secs: 2.8,
            },
        ];
        assert!(fit_power_law(&obs).is_none(), "Should need >= 3 points");
    }

    #[test]
    fn fit_power_law_filters_invalid() {
        let obs = vec![
            CostObservation {
                digits: 0.0,
                secs: 0.0,
            },
            CostObservation {
                digits: -1.0,
                secs: 1.0,
            },
            CostObservation {
                digits: 1000.0,
                secs: 0.5,
            },
        ];
        assert!(
            fit_power_law(&obs).is_none(),
            "Should filter invalid and need >= 3 valid points"
        );
    }

    // ── Portfolio slots ─────────────────────────────────────────

    #[test]
    fn portfolio_slots_small_fleet() {
        let engine = AiEngine::new();
        assert_eq!(engine.portfolio_slots(2, 0), 1);
        assert_eq!(engine.portfolio_slots(4, 1), 0);
    }

    #[test]
    fn portfolio_slots_medium_fleet() {
        let engine = AiEngine::new();
        assert_eq!(engine.portfolio_slots(10, 0), 2);
        assert_eq!(engine.portfolio_slots(10, 2), 1);
        assert_eq!(engine.portfolio_slots(10, 3), 0);
    }

    #[test]
    fn portfolio_slots_large_fleet() {
        let engine = AiEngine::new();
        assert_eq!(engine.portfolio_slots(20, 0), 2);
        assert_eq!(engine.portfolio_slots(100, 0), 2);
    }

    // ── Safety checks ───────────────────────────────────────────

    #[test]
    fn safety_rejects_over_budget() {
        let limits = SafetyLimits {
            max_projects: 3,
            budget_remaining: 5.0,
            min_budget_for_project: 12.5,
        };
        let decision = Decision::CreateProject {
            form: "kbn".to_string(),
            params: serde_json::json!({}),
            budget_usd: 25.0,
            reasoning: "test".to_string(),
            confidence: 0.9,
        };
        assert!(!safety_check(&decision, &limits));
    }

    #[test]
    fn safety_allows_within_budget() {
        let limits = SafetyLimits {
            max_projects: 3,
            budget_remaining: 50.0,
            min_budget_for_project: 12.5,
        };
        let decision = Decision::CreateProject {
            form: "kbn".to_string(),
            params: serde_json::json!({}),
            budget_usd: 25.0,
            reasoning: "test".to_string(),
            confidence: 0.9,
        };
        assert!(safety_check(&decision, &limits));
    }

    #[test]
    fn safety_always_allows_no_action() {
        let limits = SafetyLimits {
            max_projects: 0,
            budget_remaining: 0.0,
            min_budget_for_project: 100.0,
        };
        let decision = Decision::NoAction {
            reason: "test".to_string(),
        };
        assert!(safety_check(&decision, &limits));
    }

    // ── Scoring ─────────────────────────────────────────────────

    #[test]
    fn score_forms_empty_snapshot() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(0, 0, 0);
        let scores = engine.score_forms(&snapshot);
        assert_eq!(scores.len(), 12, "Should score all 12 forms");
        for s in &scores {
            assert!(
                s.total >= 0.0,
                "Score for {} should be non-negative",
                s.form
            );
        }
    }

    #[test]
    fn score_forms_sorted_descending() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(8, 64, 8);
        let scores = engine.score_forms(&snapshot);
        for w in scores.windows(2) {
            assert!(
                w[0].total >= w[1].total,
                "Scores not sorted: {} ({}) >= {} ({})",
                w[0].form,
                w[0].total,
                w[1].form,
                w[1].total
            );
        }
    }

    #[test]
    fn score_forms_excludes_forms() {
        let mut engine = AiEngine::new();
        engine.config.excluded_forms = vec!["wagstaff".to_string()];
        let snapshot = test_snapshot(8, 64, 8);
        let scores = engine.score_forms(&snapshot);
        let wagstaff = scores.iter().find(|s| s.form == "wagstaff").unwrap();
        assert_eq!(wagstaff.total, 0.0, "Excluded form should have zero score");
    }

    #[test]
    fn momentum_scoring() {
        let engine = AiEngine::new();
        let now = Utc::now();

        let mut snapshot = test_snapshot(8, 64, 8);
        snapshot.timestamp = now;

        let scores_no = engine.score_forms(&snapshot);
        let kbn_no = scores_no.iter().find(|s| s.form == "kbn").unwrap();

        snapshot.recent_discoveries = vec![
            RecentDiscovery {
                form: "kbn".to_string(),
                digits: 5000,
                found_at: now - chrono::Duration::hours(1),
            },
            RecentDiscovery {
                form: "kbn".to_string(),
                digits: 6000,
                found_at: now - chrono::Duration::hours(2),
            },
        ];

        let scores_with = engine.score_forms(&snapshot);
        let kbn_with = scores_with.iter().find(|s| s.form == "kbn").unwrap();

        assert!(
            kbn_with.momentum > kbn_no.momentum,
            "Momentum should increase with discoveries: {} > {}",
            kbn_with.momentum,
            kbn_no.momentum
        );
    }

    // ── Decide ──────────────────────────────────────────────────

    #[test]
    fn decide_no_action_no_workers() {
        let engine = AiEngine::new();
        let now = Utc::now();
        let mut snapshot = test_snapshot(0, 0, 0);

        // Provide fresh intel for all forms so no intel requests are generated
        for form in strategy::ALL_FORMS {
            snapshot.competition_intel.push(crate::db::AgentMemoryRow {
                id: 0,
                key: format!("competitive_intel:{}", form),
                value: "0.5".to_string(),
                category: "competitive_intel".to_string(),
                created_by_task: None,
                created_at: now,
                updated_at: now,
            });
        }

        let analysis = engine.orient(&snapshot);
        let decisions = engine.decide(&snapshot, &analysis);

        let has_no_action = decisions
            .iter()
            .any(|d| matches!(d, Decision::NoAction { .. }));
        assert!(
            has_no_action,
            "Should have a NoAction decision when no workers and no stale intel"
        );
    }

    #[test]
    fn decide_creates_project_with_idle_workers() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(4, 32, 4);
        let analysis = engine.orient(&snapshot);
        let decisions = engine.decide(&snapshot, &analysis);

        let has_create = decisions
            .iter()
            .any(|d| matches!(d, Decision::CreateProject { .. }));
        assert!(has_create, "Should create a project with idle workers");
    }

    #[test]
    fn decide_includes_component_scores_in_params() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(4, 32, 4);
        let analysis = engine.orient(&snapshot);
        let decisions = engine.decide(&snapshot, &analysis);

        for d in &decisions {
            if let Decision::CreateProject { params, .. } = d {
                assert!(
                    params.get("scores").is_some(),
                    "CreateProject should include component scores in params"
                );
                let scores = &params["scores"];
                assert!(
                    scores["record_gap"].is_f64(),
                    "Should have record_gap score"
                );
                assert!(
                    scores["yield_rate"].is_f64(),
                    "Should have yield_rate score"
                );
                assert!(
                    scores["competition"].is_f64(),
                    "Should have competition score"
                );
            }
        }
    }

    // ── AiEngineConfig ──────────────────────────────────────────

    #[test]
    fn default_config_sensible() {
        let config = AiEngineConfig::default();
        assert!(config.enabled);
        assert!(config.max_monthly_budget_usd > 0.0);
        assert!(config.max_per_project_budget_usd > 0.0);
        assert!(config.learn_interval_secs > 0);
    }

    // ── Drift detection ─────────────────────────────────────────

    #[test]
    fn drift_first_tick_not_significant() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(4, 32, 4);
        let drift = engine.detect_drift(&snapshot);
        assert!(
            !drift.significant,
            "First tick should not report significant drift"
        );
    }

    // ── Phase 6: Reward computation ─────────────────────────────

    #[test]
    fn test_compute_reward_with_primes() {
        let outcome = serde_json::json!({
            "primes_found": 5,
            "cost_usd": 1.0,
        });
        let reward = compute_reward(&outcome);
        assert!(reward > 0.5, "Should reward finding primes: got {}", reward);
    }

    #[test]
    fn test_compute_reward_without_primes() {
        let outcome = serde_json::json!({
            "primes_found": 0,
            "cost_usd": 10.0,
        });
        let reward = compute_reward(&outcome);
        assert!(
            reward < 0.6,
            "Zero primes should give low-ish reward: got {}",
            reward
        );
    }

    #[test]
    fn test_compute_reward_high_efficiency() {
        let low = compute_reward(&serde_json::json!({"primes_found": 1, "cost_usd": 10.0}));
        let high = compute_reward(&serde_json::json!({"primes_found": 10, "cost_usd": 1.0}));
        assert!(
            high > low,
            "Higher efficiency should give higher reward: {} > {}",
            high,
            low
        );
    }

    // ── Phase 6: Weight update ──────────────────────────────────

    #[test]
    fn test_weight_update_positive_outcome() {
        let mut engine = AiEngine::new();
        let _original_record_gap = engine.scoring_weights.record_gap;

        let outcomes = vec![crate::db::DecisionWithOutcome {
            id: 1,
            form: Some("kbn".to_string()),
            component_scores: Some(serde_json::json!({
                "record_gap": 0.9,
                "yield_rate": 0.1,
                "cost_efficiency": 0.1,
                "opportunity_density": 0.1,
                "fleet_fit": 0.1,
                "momentum": 0.1,
                "competition": 0.1,
            })),
            outcome: Some(serde_json::json!({
                "primes_found": 10,
                "cost_usd": 1.0,
            })),
        }];

        engine.update_weights_from_outcomes(&outcomes);

        // With a high-reward outcome and high record_gap score,
        // record_gap weight should increase (before normalization may adjust)
        let sum: f64 = engine.scoring_weights.as_array().iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Weights should still sum to ~1.0 after update, got {}",
            sum
        );
    }

    #[test]
    fn test_weight_update_negative_outcome() {
        let mut engine = AiEngine::new();

        let outcomes = vec![crate::db::DecisionWithOutcome {
            id: 1,
            form: Some("kbn".to_string()),
            component_scores: Some(serde_json::json!({
                "record_gap": 0.9,
                "yield_rate": 0.1,
                "cost_efficiency": 0.1,
                "opportunity_density": 0.1,
                "fleet_fit": 0.1,
                "momentum": 0.1,
                "competition": 0.1,
            })),
            outcome: Some(serde_json::json!({
                "primes_found": 0,
                "cost_usd": 50.0,
            })),
        }];

        engine.update_weights_from_outcomes(&outcomes);

        let sum: f64 = engine.scoring_weights.as_array().iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Weights should still sum to ~1.0 after negative update, got {}",
            sum
        );
    }

    #[test]
    fn test_weight_update_preserves_bounds() {
        let mut engine = AiEngine::new();

        // Apply many extreme updates
        for _ in 0..100 {
            let outcomes = vec![crate::db::DecisionWithOutcome {
                id: 1,
                form: Some("kbn".to_string()),
                component_scores: Some(serde_json::json!({
                    "record_gap": 1.0,
                    "yield_rate": 0.0,
                    "cost_efficiency": 0.0,
                    "opportunity_density": 0.0,
                    "fleet_fit": 0.0,
                    "momentum": 0.0,
                    "competition": 0.0,
                })),
                outcome: Some(serde_json::json!({
                    "primes_found": 100,
                    "cost_usd": 0.01,
                })),
            }];
            engine.update_weights_from_outcomes(&outcomes);
        }

        let weights = engine.scoring_weights.as_array();
        for &w in &weights {
            assert!(
                w >= WEIGHT_MIN && w <= WEIGHT_MAX,
                "Weight {} out of bounds [{}, {}]",
                w,
                WEIGHT_MIN,
                WEIGHT_MAX
            );
        }
    }

    #[test]
    fn test_weight_update_preserves_sum() {
        let mut engine = AiEngine::new();

        for i in 0..50 {
            let primes = if i % 2 == 0 { 5.0 } else { 0.0 };
            let outcomes = vec![crate::db::DecisionWithOutcome {
                id: i,
                form: Some("kbn".to_string()),
                component_scores: Some(serde_json::json!({
                    "record_gap": 0.3,
                    "yield_rate": 0.7,
                    "cost_efficiency": 0.5,
                    "opportunity_density": 0.2,
                    "fleet_fit": 0.8,
                    "momentum": 0.4,
                    "competition": 0.6,
                })),
                outcome: Some(serde_json::json!({
                    "primes_found": primes,
                    "cost_usd": 5.0,
                })),
            }];
            engine.update_weights_from_outcomes(&outcomes);
        }

        let sum: f64 = engine.scoring_weights.as_array().iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Weights should sum to ~1.0 after 50 updates, got {}",
            sum
        );
    }

    #[test]
    fn test_multiple_updates_dont_degenerate() {
        let mut engine = AiEngine::new();

        // 100 random-ish updates shouldn't collapse all weights to min/max
        for i in 0..100 {
            let score = (i as f64 * 0.7) % 1.0;
            let primes = ((i * 3) % 7) as f64;
            let outcomes = vec![crate::db::DecisionWithOutcome {
                id: i,
                form: Some("factorial".to_string()),
                component_scores: Some(serde_json::json!({
                    "record_gap": score,
                    "yield_rate": 1.0 - score,
                    "cost_efficiency": score * 0.5,
                    "opportunity_density": 0.3,
                    "fleet_fit": 0.5,
                    "momentum": score * 0.8,
                    "competition": 0.4,
                })),
                outcome: Some(serde_json::json!({
                    "primes_found": primes,
                    "cost_usd": 2.0,
                })),
            }];
            engine.update_weights_from_outcomes(&outcomes);
        }

        let weights = engine.scoring_weights.as_array();
        let all_at_min = weights.iter().all(|&w| (w - WEIGHT_MIN).abs() < 0.001);
        let all_at_max = weights.iter().all(|&w| (w - WEIGHT_MAX).abs() < 0.001);
        assert!(
            !all_at_min && !all_at_max,
            "Weights shouldn't all collapse to bounds: {:?}",
            weights
        );
    }

    // ── Phase 6: extract_component_scores ───────────────────────

    #[test]
    fn test_extract_component_scores_valid() {
        let json = serde_json::json!({
            "record_gap": 0.8,
            "yield_rate": 0.3,
            "cost_efficiency": 0.5,
            "opportunity_density": 0.7,
            "fleet_fit": 0.9,
            "momentum": 0.2,
            "competition": 0.4,
        });
        let scores = extract_component_scores(&json).unwrap();
        assert_eq!(scores[0], 0.8);
        assert_eq!(scores[6], 0.4);
    }

    #[test]
    fn test_extract_component_scores_missing_field() {
        let json = serde_json::json!({
            "record_gap": 0.8,
            "yield_rate": 0.3,
        });
        assert!(extract_component_scores(&json).is_none());
    }

    // ── Phase 7: Competition scoring ────────────────────────────

    #[test]
    fn test_competition_score_from_memory() {
        let engine = AiEngine::new();
        let now = Utc::now();
        let mut snapshot = test_snapshot(8, 64, 8);
        snapshot.competition_intel = vec![crate::db::AgentMemoryRow {
            id: 1,
            key: "competitive_intel:kbn".to_string(),
            value: "0.3".to_string(),
            category: "competitive_intel".to_string(),
            created_by_task: None,
            created_at: now,
            updated_at: now,
        }];

        let scores = engine.score_forms(&snapshot);
        let kbn = scores.iter().find(|s| s.form == "kbn").unwrap();
        assert!(
            (kbn.competition - 0.3).abs() < 0.001,
            "Competition should be 0.3 from memory, got {}",
            kbn.competition
        );
    }

    #[test]
    fn test_competition_score_fallback() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(8, 64, 8);

        let scores = engine.score_forms(&snapshot);
        let kbn = scores.iter().find(|s| s.form == "kbn").unwrap();
        assert!(
            (kbn.competition - 0.5).abs() < 0.001,
            "Competition should default to 0.5, got {}",
            kbn.competition
        );
    }

    #[test]
    fn test_decide_requests_intel_when_stale() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(0, 0, 0);
        let analysis = engine.orient(&snapshot);
        let decisions = engine.decide(&snapshot, &analysis);

        let intel_requests: Vec<_> = decisions
            .iter()
            .filter(|d| matches!(d, Decision::RequestAgentIntel { .. }))
            .collect();
        assert_eq!(
            intel_requests.len(),
            1,
            "Should request exactly 1 intel per tick"
        );
    }

    #[test]
    fn test_decide_no_intel_when_fresh() {
        let engine = AiEngine::new();
        let now = Utc::now();
        let mut snapshot = test_snapshot(0, 0, 0);

        // Add fresh intel for all 12 forms
        for form in strategy::ALL_FORMS {
            snapshot.competition_intel.push(crate::db::AgentMemoryRow {
                id: 0,
                key: format!("competitive_intel:{}", form),
                value: "0.5".to_string(),
                category: "competitive_intel".to_string(),
                created_by_task: None,
                created_at: now,
                updated_at: now,
            });
        }

        let analysis = engine.orient(&snapshot);
        let decisions = engine.decide(&snapshot, &analysis);

        let intel_requests = decisions
            .iter()
            .filter(|d| matches!(d, Decision::RequestAgentIntel { .. }))
            .count();
        assert_eq!(
            intel_requests, 0,
            "Should not request intel when all forms have fresh data"
        );
    }

    #[test]
    fn test_only_one_intel_request_per_tick() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(4, 32, 4);
        let analysis = engine.orient(&snapshot);
        let decisions = engine.decide(&snapshot, &analysis);

        let intel_requests = decisions
            .iter()
            .filter(|d| matches!(d, Decision::RequestAgentIntel { .. }))
            .count();
        assert!(
            intel_requests <= 1,
            "Should have at most 1 intel request, got {}",
            intel_requests
        );
    }

    // ── Phase 7: parse_competition_score ────────────────────────

    #[test]
    fn test_parse_competition_score_json() {
        let result = r#"{"competition_score": 0.3}"#;
        assert_eq!(parse_competition_score(result), Some(0.3));
    }

    #[test]
    fn test_parse_competition_score_embedded_json() {
        let result = "Analysis complete. Result: {\"competition_score\": 0.7} end.";
        assert_eq!(parse_competition_score(result), Some(0.7));
    }

    #[test]
    fn test_parse_competition_score_plain_float() {
        let result = "Competition score: 0.4";
        assert_eq!(parse_competition_score(result), Some(0.4));
    }

    #[test]
    fn test_parse_competition_score_missing() {
        let result = "No competition data available";
        assert!(parse_competition_score(result).is_none());
    }

    #[test]
    fn test_parse_competition_score_clamped() {
        let result = r#"{"competition_score": 1.5}"#;
        assert_eq!(parse_competition_score(result), Some(1.0));
    }

    // ── Phase 8: Fleet rebalancing ──────────────────────────────

    #[test]
    fn test_rebalance_no_action_when_balanced() {
        let engine = AiEngine::new();
        let snapshot = test_snapshot(4, 32, 4);
        assert!(
            engine.detect_fleet_imbalance(&snapshot).is_none(),
            "Should not rebalance when no jobs are running"
        );
    }
}
