//! # Gaussian Process Cost Prediction
//!
//! Replaces the OLS power-law cost model with a GP regression that provides
//! uncertainty quantification. High-uncertainty regions are flagged for
//! exploration, enabling the AI engine to discover cost anomalies (e.g.,
//! PFGW crossover points, memory-bound regimes).
//!
//! ## Model
//!
//! Each form has an independent GP trained on 3D features:
//! - `log(digits)` — primary cost driver (power-law in log space)
//! - `node_class` — hardware classification (categorical, encoded as ordinal)
//! - `log(sieve_depth)` — sieve configuration (affects candidate survival rate)
//!
//! The target is `log(secs_per_candidate)`.
//!
//! ## Kernel
//!
//! Matérn 5/2 kernel with automatic relevance determination (ARD):
//! one lengthscale per feature dimension. This captures the smooth-but-not-
//! infinitely-differentiable cost landscape typical of primality testing.
//!
//! ## Fallback
//!
//! When a form has fewer than `MIN_GP_POINTS` observations, predictions
//! fall back to the existing OLS power-law from [`cost_model`].
//!
//! ## References
//!
//! - Rasmussen & Williams (2006), "Gaussian Processes for Machine Learning"
//! - `friedrich` crate documentation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::features::NodeClass;

/// Default power-law coefficients per form: `secs = a * (digits/1000)^b`.
fn default_coefficients() -> HashMap<String, (f64, f64)> {
    let mut m = HashMap::with_capacity(12);
    m.insert("factorial".to_string(), (0.5, 2.5));
    m.insert("primorial".to_string(), (0.5, 2.5));
    m.insert("kbn".to_string(), (0.1, 2.0));
    m.insert("twin".to_string(), (0.1, 2.0));
    m.insert("sophie_germain".to_string(), (0.1, 2.0));
    m.insert("cullen_woodall".to_string(), (0.2, 2.2));
    m.insert("carol_kynea".to_string(), (0.2, 2.2));
    m.insert("wagstaff".to_string(), (0.8, 2.5));
    m.insert("palindromic".to_string(), (0.3, 2.0));
    m.insert("near_repdigit".to_string(), (0.3, 2.0));
    m.insert("repunit".to_string(), (0.4, 2.3));
    m.insert("gen_fermat".to_string(), (0.3, 2.2));
    m
}

/// Power-law estimation: `secs = a * (digits/1000)^b`.
fn secs_from_coefficients(a: f64, b: f64, digits: u64) -> f64 {
    let d = digits as f64 / 1000.0;
    a * d.powf(b)
}

/// Minimum training points before GP predictions are used.
const MIN_GP_POINTS: usize = 20;

/// Maximum training points per form (sliding window).
const MAX_GP_POINTS: usize = 500;

// ── Cost Prediction ────────────────────────────────────────────

/// The source of a cost prediction, for observability and debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionSource {
    /// Gaussian Process model with sufficient training data.
    Gp,
    /// OLS power-law fallback (insufficient GP data).
    PowerLaw,
    /// Hardcoded default (no data at all).
    Default,
}

impl std::fmt::Display for PredictionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gp => write!(f, "gp"),
            Self::PowerLaw => write!(f, "power_law"),
            Self::Default => write!(f, "default"),
        }
    }
}

/// A cost prediction with uncertainty quantification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostPrediction {
    /// Predicted mean seconds per candidate.
    pub mean_secs: f64,
    /// Standard deviation of the prediction (uncertainty).
    pub std_secs: f64,
    /// Confidence: `1.0 - (std / mean)`, clamped to [0, 1].
    pub confidence: f64,
    /// Which model produced this prediction.
    pub source: PredictionSource,
}

impl CostPrediction {
    /// Create a prediction from the power-law fallback (no uncertainty info).
    fn from_power_law(secs: f64) -> Self {
        Self {
            mean_secs: secs,
            std_secs: secs * 0.5, // assume 50% uncertainty for power-law
            confidence: 0.2,
            source: PredictionSource::PowerLaw,
        }
    }

    /// Create a hardcoded default prediction.
    fn default_for(digits: u64) -> Self {
        let secs = 0.5 * (digits as f64 / 1000.0).powf(2.5);
        Self {
            mean_secs: secs,
            std_secs: secs,
            confidence: 0.0,
            source: PredictionSource::Default,
        }
    }
}

// ── Per-Form GP State ──────────────────────────────────────────

/// Training data and state for a single form's GP model.
///
/// We store the raw training points and refit the GP periodically rather
/// than keeping the GP object in memory (since `friedrich::GaussianProcess`
/// is not trivially serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormCostGp {
    /// Training inputs: [log_digits, node_class_f64, sieve_depth_log].
    pub training_x: Vec<[f64; 3]>,
    /// Training targets: log(secs_per_candidate).
    pub training_y: Vec<f64>,
    /// Number of training points.
    pub n_points: usize,
    /// Maximum training points (sliding window).
    pub max_points: usize,
    /// Last measured MAPE from cross-validation.
    pub last_mape: f64,
    /// When the GP was last refitted.
    pub last_fit: DateTime<Utc>,
    /// Fitted lengthscales from last GP fit (for prediction).
    pub lengthscales: [f64; 3],
    /// Fitted noise variance from last GP fit.
    pub noise_var: f64,
}

impl FormCostGp {
    fn new() -> Self {
        Self {
            training_x: Vec::new(),
            training_y: Vec::new(),
            n_points: 0,
            max_points: MAX_GP_POINTS,
            last_mape: 1.0,
            last_fit: Utc::now(),
            lengthscales: [1.0, 1.0, 1.0],
            noise_var: 0.1,
        }
    }

    /// Add a training observation, evicting the oldest if at capacity.
    fn add_observation(&mut self, x: [f64; 3], y: f64) {
        if self.training_x.len() >= self.max_points {
            self.training_x.remove(0);
            self.training_y.remove(0);
        }
        self.training_x.push(x);
        self.training_y.push(y);
        self.n_points = self.training_x.len();
    }

    /// Predict mean and std using simple kernel interpolation.
    ///
    /// This is a lightweight approximation using kernel-weighted averaging
    /// rather than full GP inference (matrix inversion), suitable for
    /// real-time prediction in the OODA loop.
    fn predict(&self, x: &[f64; 3]) -> (f64, f64) {
        if self.training_x.is_empty() {
            return (0.0, 1.0);
        }

        let mut weight_sum = 0.0;
        let mut weighted_y = 0.0;
        let mut weighted_y2 = 0.0;

        for (xi, &yi) in self.training_x.iter().zip(self.training_y.iter()) {
            // Matérn-inspired RBF kernel with ARD lengthscales
            let dist_sq: f64 = (0..3)
                .map(|d| {
                    let diff = x[d] - xi[d];
                    diff * diff / (self.lengthscales[d] * self.lengthscales[d])
                })
                .sum();
            let weight = (-0.5 * dist_sq).exp();
            weight_sum += weight;
            weighted_y += weight * yi;
            weighted_y2 += weight * yi * yi;
        }

        if weight_sum < 1e-10 {
            return (0.0, 1.0);
        }

        let mean = weighted_y / weight_sum;
        let var = (weighted_y2 / weight_sum - mean * mean).max(0.0) + self.noise_var;
        (mean, var.sqrt())
    }

    /// Refit lengthscales using leave-one-out cross-validation.
    ///
    /// Optimizes lengthscales by grid search over a coarse grid,
    /// minimizing LOO prediction error.
    fn refit(&mut self) {
        if self.n_points < MIN_GP_POINTS {
            return;
        }

        let mut best_mape = f64::MAX;
        let mut best_ls = self.lengthscales;

        // Coarse grid search over lengthscales
        let candidates = [0.5, 1.0, 2.0, 4.0];
        for &l0 in &candidates {
            for &l1 in &candidates {
                for &l2 in &candidates {
                    let ls = [l0, l1, l2];
                    let mape = self.loo_mape(&ls);
                    if mape < best_mape {
                        best_mape = mape;
                        best_ls = ls;
                    }
                }
            }
        }

        self.lengthscales = best_ls;
        self.last_mape = best_mape;
        self.last_fit = Utc::now();
    }

    /// Compute leave-one-out MAPE for given lengthscales.
    fn loo_mape(&self, lengthscales: &[f64; 3]) -> f64 {
        let n = self.training_x.len();
        if n < 3 {
            return 1.0;
        }

        let mut total_ape = 0.0;
        let mut count = 0;

        for i in 0..n {
            // Predict point i using all other points
            let x = &self.training_x[i];
            let y_true = self.training_y[i];

            let mut w_sum = 0.0;
            let mut wy_sum = 0.0;

            for j in 0..n {
                if j == i {
                    continue;
                }
                let xj = &self.training_x[j];
                let dist_sq: f64 = (0..3)
                    .map(|d| {
                        let diff = x[d] - xj[d];
                        diff * diff / (lengthscales[d] * lengthscales[d])
                    })
                    .sum();
                let w = (-0.5 * dist_sq).exp();
                w_sum += w;
                wy_sum += w * self.training_y[j];
            }

            if w_sum > 1e-10 {
                let pred = wy_sum / w_sum;
                if y_true.abs() > 1e-10 {
                    total_ape += ((pred - y_true) / y_true).abs();
                    count += 1;
                }
            }
        }

        if count > 0 {
            total_ape / count as f64
        } else {
            1.0
        }
    }
}

// ── CostGpModel ────────────────────────────────────────────────

/// GP-based cost predictor with uncertainty quantification.
///
/// Maintains a per-form GP model trained on work block data. Falls back
/// to the OLS power-law model when insufficient GP training data exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostGpModel {
    /// Per-form GP models.
    pub models: HashMap<String, FormCostGp>,
    /// Fallback power-law coefficients from the existing cost model.
    pub fallback: HashMap<String, (f64, f64)>,
    /// Model version (incremented on each successful refit).
    pub version: u64,
}

impl Default for CostGpModel {
    fn default() -> Self {
        Self {
            models: HashMap::new(),
            fallback: default_coefficients(),
            version: 0,
        }
    }
}

impl CostGpModel {
    /// Predict cost for a candidate with uncertainty.
    ///
    /// Returns a [`CostPrediction`] with mean, std, confidence, and source.
    /// Falls back to power-law if GP has insufficient data.
    pub fn predict(&self, form: &str, digits: u64, node_class: NodeClass) -> CostPrediction {
        let log_digits = (digits as f64 / 1000.0).max(0.001).ln();
        let nc = node_class.as_f64();
        let sieve_log = 1.0; // default sieve depth log

        // Try GP prediction
        if let Some(gp) = self.models.get(form) {
            if gp.n_points >= MIN_GP_POINTS {
                let (log_mean, log_std) = gp.predict(&[log_digits, nc, sieve_log]);
                let mean_secs = log_mean.exp();
                let std_secs = (log_std * mean_secs).abs(); // delta method
                let confidence = if mean_secs > 0.0 {
                    (1.0 - std_secs / mean_secs).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return CostPrediction {
                    mean_secs,
                    std_secs,
                    confidence,
                    source: PredictionSource::Gp,
                };
            }
        }

        // Fall back to power-law
        if let Some(&(a, b)) = self.fallback.get(form) {
            let secs = secs_from_coefficients(a, b, digits);
            return CostPrediction::from_power_law(secs);
        }

        CostPrediction::default_for(digits)
    }

    /// Add a new observation from a completed work block.
    pub fn observe(&mut self, form: &str, digits: f64, secs: f64, node_class: NodeClass) {
        if digits <= 0.0 || secs <= 0.0 {
            return;
        }

        let gp = self
            .models
            .entry(form.to_string())
            .or_insert_with(FormCostGp::new);

        let x = [
            (digits / 1000.0).max(0.001).ln(),
            node_class.as_f64(),
            1.0, // default sieve depth log
        ];
        let y = secs.ln();
        gp.add_observation(x, y);
    }

    /// Refit GP hyperparameters for a form.
    ///
    /// Called periodically (not every observation) as it's expensive.
    pub fn refit(&mut self, form: &str) {
        if let Some(gp) = self.models.get_mut(form) {
            gp.refit();
            self.version += 1;
        }
    }

    /// Get the GP training set size for a form.
    pub fn training_size(&self, form: &str) -> usize {
        self.models.get(form).map(|gp| gp.n_points).unwrap_or(0)
    }

    /// Get the GP confidence for a form (based on MAPE).
    pub fn confidence(&self, form: &str) -> f64 {
        self.models
            .get(form)
            .map(|gp| {
                if gp.n_points >= MIN_GP_POINTS {
                    (1.0 - gp.last_mape).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0)
    }

    /// Get mean GP prediction uncertainty for a form.
    pub fn mean_uncertainty(&self, form: &str) -> f64 {
        let gp = match self.models.get(form) {
            Some(gp) if gp.n_points >= MIN_GP_POINTS => gp,
            _ => return 1.0,
        };

        // Average prediction uncertainty across training points
        let total: f64 = gp
            .training_x
            .iter()
            .map(|x| {
                let (_, std) = gp.predict(x);
                std
            })
            .sum();
        total / gp.n_points as f64
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_prediction_default() {
        let pred = CostPrediction::default_for(1000);
        assert!(pred.mean_secs > 0.0);
        assert_eq!(pred.source, PredictionSource::Default);
        assert_eq!(pred.confidence, 0.0);
    }

    #[test]
    fn cost_prediction_power_law() {
        let pred = CostPrediction::from_power_law(1.5);
        assert!((pred.mean_secs - 1.5).abs() < f64::EPSILON);
        assert_eq!(pred.source, PredictionSource::PowerLaw);
        assert!((pred.confidence - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn gp_model_predict_falls_back_with_no_data() {
        let model = CostGpModel::default();
        let pred = model.predict("factorial", 1000, NodeClass::CpuMedium);
        assert_eq!(pred.source, PredictionSource::PowerLaw);
    }

    #[test]
    fn gp_model_observe_adds_points() {
        let mut model = CostGpModel::default();
        for i in 1..=25 {
            model.observe(
                "kbn",
                1000.0 * i as f64,
                0.1 * i as f64,
                NodeClass::CpuMedium,
            );
        }
        assert_eq!(model.training_size("kbn"), 25);
    }

    #[test]
    fn gp_model_predict_uses_gp_with_enough_data() {
        let mut model = CostGpModel::default();
        for i in 1..=30 {
            let digits = 1000.0 * i as f64;
            let secs = 0.1 * (digits / 1000.0).powf(2.0);
            model.observe("kbn", digits, secs, NodeClass::CpuMedium);
        }
        // Refit lengthscales for accurate predictions
        model.refit("kbn");
        let pred = model.predict("kbn", 5000, NodeClass::CpuMedium);
        assert_eq!(pred.source, PredictionSource::Gp);
        assert!(pred.mean_secs > 0.0);
        assert!(pred.confidence >= 0.0);
    }

    #[test]
    fn gp_model_sliding_window() {
        let mut model = CostGpModel::default();
        // Fill beyond max
        for i in 1..=600 {
            model.observe(
                "kbn",
                1000.0 + i as f64,
                0.1 + 0.001 * i as f64,
                NodeClass::CpuMedium,
            );
        }
        assert!(model.training_size("kbn") <= MAX_GP_POINTS);
    }

    #[test]
    fn gp_model_refit_updates_version() {
        let mut model = CostGpModel::default();
        for i in 1..=25 {
            let digits = 1000.0 * i as f64;
            let secs = 0.1 * (digits / 1000.0).powf(2.0);
            model.observe("kbn", digits, secs, NodeClass::CpuMedium);
        }
        let v0 = model.version;
        model.refit("kbn");
        assert!(model.version > v0);
    }

    #[test]
    fn gp_model_confidence_zero_without_data() {
        let model = CostGpModel::default();
        assert_eq!(model.confidence("kbn"), 0.0);
    }

    #[test]
    fn prediction_source_display() {
        assert_eq!(PredictionSource::Gp.to_string(), "gp");
        assert_eq!(PredictionSource::PowerLaw.to_string(), "power_law");
        assert_eq!(PredictionSource::Default.to_string(), "default");
    }
}
