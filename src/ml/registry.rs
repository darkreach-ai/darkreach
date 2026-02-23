//! # Model Registry — Versioning, Shadow Mode, and A/B Testing
//!
//! Tracks model versions across all ML subsystems, supports running shadow
//! models alongside active ones, and provides promotion logic based on
//! predictive performance comparison.
//!
//! ## Shadow Mode
//!
//! When `shadow_mode` is enabled, both the active and shadow models produce
//! predictions. Only the active model's output drives decisions; the shadow's
//! predictions are logged for offline comparison. If the shadow outperforms
//! the active model over a configurable window, it can be promoted.
//!
//! ## Database
//!
//! Model metadata is persisted in `ml_model_registry` for crash recovery
//! and audit trail.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum predictions before a shadow model can be promoted.
const MIN_PREDICTIONS_FOR_PROMOTION: u64 = 100;

/// Shadow must outperform active by this factor to be promoted.
const PROMOTION_THRESHOLD: f64 = 1.05;

// ── Model Metrics ──────────────────────────────────────────────

/// Performance metrics tracked for each model version.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelMetrics {
    /// Total predictions made by this version.
    pub predictions: u64,
    /// Mean Absolute Percentage Error (cost model only).
    pub mape: f64,
    /// Cumulative regret (bandits only): sum of (best_possible - actual) reward.
    pub regret: f64,
    /// Throughput improvement over heuristic baseline (percentage).
    pub throughput_gain: f64,
}

// ── Model Version ──────────────────────────────────────────────

/// Metadata for a single model version within a subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Which subsystem this model belongs to.
    pub subsystem: String,
    /// Monotonically increasing version number.
    pub version: u64,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
    /// Accumulated performance metrics.
    pub metrics: ModelMetrics,
}

impl ModelVersion {
    /// Create a new version for a subsystem.
    pub fn new(subsystem: &str, version: u64) -> Self {
        Self {
            subsystem: subsystem.to_string(),
            version,
            created_at: Utc::now(),
            metrics: ModelMetrics::default(),
        }
    }

    /// Record a prediction, optionally with an error measurement.
    pub fn record_prediction(&mut self, error: Option<f64>) {
        self.metrics.predictions += 1;
        if let Some(e) = error {
            // Running MAPE update: MAPE_new = MAPE_old + (|error| - MAPE_old) / n
            let n = self.metrics.predictions as f64;
            self.metrics.mape += (e.abs() - self.metrics.mape) / n;
        }
    }

    /// Total predictions made.
    pub fn predictions(&self) -> u64 {
        self.metrics.predictions
    }

    /// Running MAPE.
    pub fn mape(&self) -> f64 {
        self.metrics.mape
    }
}

// ── Model Registry ─────────────────────────────────────────────

/// Central registry that tracks active and shadow model versions for
/// all ML subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    /// Currently active model per subsystem.
    pub active_models: HashMap<String, ModelVersion>,
    /// Shadow (candidate) models being evaluated per subsystem.
    pub shadow_models: HashMap<String, ModelVersion>,
    /// Whether shadow mode is enabled globally.
    pub shadow_mode: bool,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let subsystems = ["bandits", "cost_gp", "sieve_opt", "node_intel"];
        let mut active = HashMap::new();
        for sub in &subsystems {
            active.insert(sub.to_string(), ModelVersion::new(sub, 1));
        }
        Self {
            active_models: active,
            shadow_models: HashMap::new(),
            shadow_mode: false,
        }
    }
}

impl ModelRegistry {
    /// Get the active model version number for a subsystem.
    pub fn active_version(&self, subsystem: &str) -> u64 {
        self.active_models
            .get(subsystem)
            .map(|m| m.version)
            .unwrap_or(0)
    }

    /// Record a prediction for the active model in a subsystem.
    pub fn record_active_prediction(&mut self, subsystem: &str, error: Option<f64>) {
        if let Some(model) = self.active_models.get_mut(subsystem) {
            model.record_prediction(error);
        }
    }

    /// Record a prediction for the shadow model in a subsystem.
    pub fn record_shadow_prediction(&mut self, subsystem: &str, error: Option<f64>) {
        if let Some(model) = self.shadow_models.get_mut(subsystem) {
            model.record_prediction(error);
        }
    }

    /// Check if a shadow model is registered for a subsystem.
    pub fn has_shadow(&self, subsystem: &str) -> bool {
        self.shadow_mode && self.shadow_models.contains_key(subsystem)
    }

    /// Register a new shadow model for evaluation.
    pub fn register_shadow(&mut self, subsystem: &str) {
        let next_version = self.active_version(subsystem) + 1;
        self.shadow_models.insert(
            subsystem.to_string(),
            ModelVersion::new(subsystem, next_version),
        );
    }

    /// Evaluate all shadow models and promote any that outperform the active.
    ///
    /// Promotion criteria:
    /// 1. Shadow has >= `MIN_PREDICTIONS_FOR_PROMOTION` predictions
    /// 2. Shadow MAPE is at least `PROMOTION_THRESHOLD` times better than active
    ///
    /// Returns names of promoted subsystems.
    pub fn evaluate_and_promote(&mut self) -> Vec<String> {
        if !self.shadow_mode {
            return vec![];
        }

        let mut promoted = vec![];
        let subsystems: Vec<String> = self.shadow_models.keys().cloned().collect();

        for subsystem in subsystems {
            let should_promote = {
                let Some(shadow) = self.shadow_models.get(&subsystem) else {
                    continue;
                };
                let Some(active) = self.active_models.get(&subsystem) else {
                    continue;
                };

                shadow.predictions() >= MIN_PREDICTIONS_FOR_PROMOTION
                    && active.mape() > 0.0
                    && shadow.mape() > 0.0
                    && (active.mape() / shadow.mape()) >= PROMOTION_THRESHOLD
            };

            if should_promote {
                if let Some(shadow) = self.shadow_models.remove(&subsystem) {
                    self.active_models.insert(subsystem.clone(), shadow);
                    promoted.push(subsystem);
                }
            }
        }

        promoted
    }

    /// Get summary metrics for all subsystems (for API/dashboard).
    pub fn summary(&self) -> Vec<RegistrySummary> {
        let mut out = vec![];
        for (subsystem, model) in &self.active_models {
            out.push(RegistrySummary {
                subsystem: subsystem.clone(),
                active_version: model.version,
                active_predictions: model.predictions(),
                active_mape: model.mape(),
                shadow_version: self.shadow_models.get(subsystem).map(|m| m.version),
                shadow_predictions: self.shadow_models.get(subsystem).map(|m| m.predictions()),
                shadow_mape: self.shadow_models.get(subsystem).map(|m| m.mape()),
            });
        }
        out.sort_by(|a, b| a.subsystem.cmp(&b.subsystem));
        out
    }
}

/// Dashboard-friendly summary of a subsystem's model state.
#[derive(Debug, Clone, Serialize)]
pub struct RegistrySummary {
    pub subsystem: String,
    pub active_version: u64,
    pub active_predictions: u64,
    pub active_mape: f64,
    pub shadow_version: Option<u64>,
    pub shadow_predictions: Option<u64>,
    pub shadow_mape: Option<f64>,
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_all_subsystems() {
        let reg = ModelRegistry::default();
        assert!(reg.active_models.contains_key("bandits"));
        assert!(reg.active_models.contains_key("cost_gp"));
        assert!(reg.active_models.contains_key("sieve_opt"));
        assert!(reg.active_models.contains_key("node_intel"));
        assert_eq!(reg.active_models.len(), 4);
    }

    #[test]
    fn active_version_starts_at_1() {
        let reg = ModelRegistry::default();
        assert_eq!(reg.active_version("bandits"), 1);
        assert_eq!(reg.active_version("unknown"), 0);
    }

    #[test]
    fn record_prediction_updates_mape() {
        let mut reg = ModelRegistry::default();
        reg.record_active_prediction("cost_gp", Some(0.10));
        reg.record_active_prediction("cost_gp", Some(0.20));
        let model = &reg.active_models["cost_gp"];
        assert_eq!(model.predictions(), 2);
        assert!(model.mape() > 0.0);
    }

    #[test]
    fn shadow_registration_and_promotion() {
        let mut reg = ModelRegistry::default();
        reg.shadow_mode = true;
        reg.register_shadow("cost_gp");
        assert!(reg.has_shadow("cost_gp"));
        assert_eq!(reg.shadow_models["cost_gp"].version, 2);

        // Active has high MAPE, shadow has low MAPE
        for _ in 0..150 {
            reg.record_active_prediction("cost_gp", Some(0.50));
            reg.record_shadow_prediction("cost_gp", Some(0.10));
        }

        let promoted = reg.evaluate_and_promote();
        assert!(promoted.contains(&"cost_gp".to_string()));
        assert_eq!(reg.active_version("cost_gp"), 2);
        assert!(!reg.shadow_models.contains_key("cost_gp"));
    }

    #[test]
    fn shadow_not_promoted_without_enough_data() {
        let mut reg = ModelRegistry::default();
        reg.shadow_mode = true;
        reg.register_shadow("bandits");

        // Only 10 predictions — below threshold
        for _ in 0..10 {
            reg.record_active_prediction("bandits", Some(0.50));
            reg.record_shadow_prediction("bandits", Some(0.10));
        }

        let promoted = reg.evaluate_and_promote();
        assert!(promoted.is_empty());
    }

    #[test]
    fn summary_lists_all_subsystems() {
        let reg = ModelRegistry::default();
        let summary = reg.summary();
        assert_eq!(summary.len(), 4);
    }
}
