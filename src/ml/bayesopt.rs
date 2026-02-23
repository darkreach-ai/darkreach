//! # Bayesian Optimization — Adaptive Search Parameter Tuning
//!
//! Optimizes per-form search parameters (sieve depth, block size) using
//! Bayesian optimization with a GP surrogate and Expected Improvement
//! acquisition function.
//!
//! ## Replaces
//!
//! - `auto_sieve_depth()` in `sieve.rs` — heuristic sieve depth selection
//! - `optimal_block_size()` in `db/jobs.rs` — heuristic block sizing
//!
//! ## Design
//!
//! Each form maintains a set of (sieve_depth_log, block_size_log) → throughput
//! observations. The optimizer suggests the next parameter setting to try
//! using Expected Improvement (EI) over a GP surrogate.
//!
//! ## Fallback
//!
//! When `n_evals < MIN_BAYESOPT_EVALS`, returns the existing heuristic values.
//!
//! ## References
//!
//! - Snoek, Larochelle, Adams (2012), "Practical Bayesian Optimization of ML Hyperparameters"
//! - Jones, Schonlau, Welch (1998), "Efficient Global Optimization of Expensive Black-Box Functions"

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum evaluations before BayesOpt suggestions are used.
const MIN_BAYESOPT_EVALS: u64 = 15;

/// Default target block duration in seconds.
const DEFAULT_TARGET_BLOCK_SECS: f64 = 600.0;

// ── Observation Point ──────────────────────────────────────────

/// A single observation in the BayesOpt search space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesOptPoint {
    /// Log of sieve depth (log2 space).
    pub sieve_depth_log: f64,
    /// Log of block size.
    pub block_size_log: f64,
    /// Objective value: throughput (primes_found / core_hours).
    pub throughput: f64,
    /// Covariate: digit range of the work block.
    pub digits: f64,
    /// When this observation was recorded.
    pub observed_at: DateTime<Utc>,
}

// ── Per-Form Optimizer State ───────────────────────────────────

/// Optimization state for a single search form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormOptState {
    /// All observed (params → throughput) points.
    pub observations: Vec<BayesOptPoint>,
    /// Best observed sieve depth.
    pub best_sieve_depth: u64,
    /// Best observed block size.
    pub best_block_size: i64,
    /// Best observed throughput.
    pub best_throughput: f64,
    /// Total evaluations.
    pub n_evals: u64,
}

impl Default for FormOptState {
    fn default() -> Self {
        Self {
            observations: Vec::new(),
            best_sieve_depth: 100_000,
            best_block_size: 1000,
            best_throughput: 0.0,
            n_evals: 0,
        }
    }
}

// ── Sieve Optimizer ────────────────────────────────────────────

/// Bayesian optimizer for per-form sieve depth and block size.
///
/// Maintains per-form optimization state and suggests parameter settings
/// that balance exploration (trying new settings) with exploitation
/// (refining known-good settings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveOptimizer {
    /// Per-form optimization state.
    pub optimizers: HashMap<String, FormOptState>,
    /// Target block processing time in seconds.
    pub target_block_secs: f64,
}

impl Default for SieveOptimizer {
    fn default() -> Self {
        Self {
            optimizers: HashMap::new(),
            target_block_secs: DEFAULT_TARGET_BLOCK_SECS,
        }
    }
}

impl SieveOptimizer {
    /// Suggest next (sieve_depth, block_size) to try for a form.
    ///
    /// Uses Expected Improvement when enough data exists, otherwise returns
    /// a random perturbation of the current best for exploration.
    pub fn suggest(&self, form: &str, digits: u64) -> (u64, i64) {
        let state = match self.optimizers.get(form) {
            Some(s) if s.n_evals >= MIN_BAYESOPT_EVALS => s,
            _ => return self.heuristic_suggestion(form, digits),
        };

        // Use GP-based Expected Improvement
        self.ei_suggest(state, digits)
    }

    /// Record an observation from a completed work block.
    pub fn observe(
        &mut self,
        form: &str,
        sieve_depth: u64,
        block_size: i64,
        digits: f64,
        throughput: f64,
    ) {
        let state = self.optimizers.entry(form.to_string()).or_default();

        let point = BayesOptPoint {
            sieve_depth_log: (sieve_depth as f64).max(1.0).ln(),
            block_size_log: (block_size as f64).max(1.0).ln(),
            throughput,
            digits,
            observed_at: Utc::now(),
        };

        state.observations.push(point);
        state.n_evals += 1;

        // Update best if improved
        if throughput > state.best_throughput {
            state.best_throughput = throughput;
            state.best_sieve_depth = sieve_depth;
            state.best_block_size = block_size;
        }

        // Keep observation window bounded
        if state.observations.len() > 200 {
            state.observations.remove(0);
        }
    }

    /// Get current best parameters for a form (production use).
    pub fn best_params(&self, form: &str) -> (u64, i64) {
        self.optimizers
            .get(form)
            .map(|s| (s.best_sieve_depth, s.best_block_size))
            .unwrap_or((100_000, 1000))
    }

    /// Get total evaluations for a form.
    pub fn eval_count(&self, form: &str) -> u64 {
        self.optimizers.get(form).map(|s| s.n_evals).unwrap_or(0)
    }

    /// Get best observed throughput for a form.
    pub fn best_throughput(&self, form: &str) -> f64 {
        self.optimizers
            .get(form)
            .map(|s| s.best_throughput)
            .unwrap_or(0.0)
    }

    // ── Internal ───────────────────────────────────────────────

    /// Heuristic suggestion when insufficient BayesOpt data.
    fn heuristic_suggestion(&self, _form: &str, digits: u64) -> (u64, i64) {
        // Use the existing auto_sieve_depth heuristic logic
        let sieve_depth = if digits < 1000 {
            100_000
        } else if digits < 10_000 {
            1_000_000
        } else {
            10_000_000
        };

        // Block size scales with digits
        let block_size = if digits < 1000 {
            10_000
        } else if digits < 10_000 {
            1_000
        } else {
            100
        };

        (sieve_depth, block_size)
    }

    /// Suggest using Expected Improvement over a GP surrogate.
    ///
    /// Evaluates EI at a grid of candidate points and returns the maximizer.
    fn ei_suggest(&self, state: &FormOptState, digits: u64) -> (u64, i64) {
        let best_y = state.best_throughput;
        if state.observations.is_empty() {
            return (state.best_sieve_depth, state.best_block_size);
        }

        // Candidate grid: explore around the current best
        let best_sd_log = (state.best_sieve_depth as f64).max(1.0).ln();
        let best_bs_log = (state.best_block_size as f64).max(1.0).ln();

        let mut best_ei = f64::NEG_INFINITY;
        let mut best_sd = state.best_sieve_depth;
        let mut best_bs = state.best_block_size;

        let offsets = [-1.0, -0.5, 0.0, 0.5, 1.0];
        for &sd_off in &offsets {
            for &bs_off in &offsets {
                let sd_log = best_sd_log + sd_off;
                let bs_log = best_bs_log + bs_off;

                // GP prediction at this point using kernel-weighted average
                let (mean, std) = self.gp_predict(state, sd_log, bs_log, digits as f64);

                // Expected Improvement
                let ei = if std > 1e-10 {
                    let z = (mean - best_y) / std;
                    let pdf = normal_pdf(z);
                    let cdf = normal_cdf(z);
                    (mean - best_y) * cdf + std * pdf
                } else if mean > best_y {
                    mean - best_y
                } else {
                    0.0
                };

                if ei > best_ei {
                    best_ei = ei;
                    best_sd = sd_log.exp().round() as u64;
                    best_bs = bs_log.exp().round() as i64;
                }
            }
        }

        // Clamp to reasonable ranges
        best_sd = best_sd.clamp(10_000, 100_000_000);
        best_bs = best_bs.clamp(10, 1_000_000);

        (best_sd, best_bs)
    }

    /// Kernel-weighted GP prediction at a candidate point.
    fn gp_predict(
        &self,
        state: &FormOptState,
        sd_log: f64,
        bs_log: f64,
        digits: f64,
    ) -> (f64, f64) {
        let mut w_sum = 0.0;
        let mut wy_sum = 0.0;
        let mut wy2_sum = 0.0;

        let ls_sd = 1.0;
        let ls_bs = 1.0;
        let ls_digits = 2.0;

        for obs in &state.observations {
            let d2 = ((sd_log - obs.sieve_depth_log) / ls_sd).powi(2)
                + ((bs_log - obs.block_size_log) / ls_bs).powi(2)
                + ((digits - obs.digits) / (ls_digits * 1000.0)).powi(2);
            let w = (-0.5 * d2).exp();
            w_sum += w;
            wy_sum += w * obs.throughput;
            wy2_sum += w * obs.throughput * obs.throughput;
        }

        if w_sum < 1e-10 {
            return (0.0, 1.0);
        }

        let mean = wy_sum / w_sum;
        let var = (wy2_sum / w_sum - mean * mean).max(0.0) + 0.01;
        (mean, var.sqrt())
    }
}

// ── Normal Distribution Helpers ────────────────────────────────

/// Standard normal PDF.
fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Standard normal CDF (Abramowitz & Stegun approximation 7.1.26).
fn normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989422804014327;
    let poly = t * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));
    let cdf = 1.0 - d * (-0.5 * x * x).exp() * poly;
    if x >= 0.0 {
        cdf
    } else {
        1.0 - cdf
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_pdf_at_zero() {
        let p = normal_pdf(0.0);
        assert!((p - 0.3989).abs() < 0.001);
    }

    #[test]
    fn normal_cdf_at_zero() {
        let c = normal_cdf(0.0);
        assert!((c - 0.5).abs() < 0.01);
    }

    #[test]
    fn normal_cdf_far_positive() {
        let c = normal_cdf(5.0);
        assert!((c - 1.0).abs() < 0.001);
    }

    #[test]
    fn normal_cdf_far_negative() {
        let c = normal_cdf(-5.0);
        assert!(c < 0.001);
    }

    #[test]
    fn optimizer_heuristic_fallback() {
        let opt = SieveOptimizer::default();
        let (sd, bs) = opt.suggest("kbn", 5000);
        assert!(sd > 0);
        assert!(bs > 0);
    }

    #[test]
    fn optimizer_observe_updates_best() {
        let mut opt = SieveOptimizer::default();
        opt.observe("kbn", 100_000, 1000, 5000.0, 0.5);
        opt.observe("kbn", 1_000_000, 500, 5000.0, 0.8);
        assert_eq!(opt.best_params("kbn"), (1_000_000, 500));
        assert!((opt.best_throughput("kbn") - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn optimizer_suggest_uses_bayesopt_with_enough_data() {
        let mut opt = SieveOptimizer::default();
        for i in 0..20 {
            let sd = 100_000 + i * 50_000;
            let bs = 500 + i * 100;
            let tp = 0.1 + 0.05 * (i as f64);
            opt.observe("kbn", sd, bs as i64, 5000.0, tp);
        }
        assert!(opt.eval_count("kbn") >= MIN_BAYESOPT_EVALS);
        let (sd, bs) = opt.suggest("kbn", 5000);
        assert!(sd > 0);
        assert!(bs > 0);
    }

    #[test]
    fn optimizer_observation_window_bounded() {
        let mut opt = SieveOptimizer::default();
        for i in 0..250 {
            opt.observe("kbn", 100_000 + i, 1000, 5000.0, 0.1);
        }
        let state = &opt.optimizers["kbn"];
        assert!(state.observations.len() <= 200);
    }

    #[test]
    fn optimizer_eval_count_default() {
        let opt = SieveOptimizer::default();
        assert_eq!(opt.eval_count("unknown"), 0);
    }

    #[test]
    fn optimizer_best_throughput_default() {
        let opt = SieveOptimizer::default();
        assert_eq!(opt.best_throughput("unknown"), 0.0);
    }
}
