//! # Feature Engineering Pipeline
//!
//! Provides cached, refreshable feature vectors for forms and nodes. The
//! [`FeatureStore`] assembles features from the [`WorldSnapshot`] and ML
//! subsystem state each tick, giving all downstream models a consistent
//! input surface.
//!
//! ## Design
//!
//! Features are recomputed once per OODA tick (~30s) rather than per-query.
//! This amortises the extraction cost and guarantees that all models see the
//! same feature values within a single decision cycle.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai_engine::WorldSnapshot;

// ── Node Classification ────────────────────────────────────────

/// Coarse hardware classification for GP input dimensions.
///
/// Keeps the GP feature space low-dimensional (avoids the curse of
/// dimensionality) while capturing the dominant performance factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeClass {
    /// < 4 cores, typical single-board or small VPS.
    CpuSmall,
    /// 4–16 cores, typical cloud instance.
    CpuMedium,
    /// > 16 cores, dedicated compute or bare metal.
    CpuLarge,
    /// NVIDIA GPU with CUDA runtime.
    GpuCuda,
    /// Apple GPU with Metal runtime.
    GpuMetal,
}

impl NodeClass {
    /// Encode as a numeric value for GP feature vectors.
    pub fn as_f64(self) -> f64 {
        match self {
            Self::CpuSmall => 0.0,
            Self::CpuMedium => 1.0,
            Self::CpuLarge => 2.0,
            Self::GpuCuda => 3.0,
            Self::GpuMetal => 4.0,
        }
    }

    /// Classify from core count and optional GPU runtime string.
    pub fn classify(cores: u32, gpu_runtime: Option<&str>) -> Self {
        match gpu_runtime {
            Some(rt) if rt.contains("cuda") => Self::GpuCuda,
            Some(rt) if rt.contains("metal") => Self::GpuMetal,
            _ if cores > 16 => Self::CpuLarge,
            _ if cores >= 4 => Self::CpuMedium,
            _ => Self::CpuSmall,
        }
    }
}

impl std::fmt::Display for NodeClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CpuSmall => write!(f, "cpu_small"),
            Self::CpuMedium => write!(f, "cpu_medium"),
            Self::CpuLarge => write!(f, "cpu_large"),
            Self::GpuCuda => write!(f, "gpu_cuda"),
            Self::GpuMetal => write!(f, "gpu_metal"),
        }
    }
}

/// GPU affinity score for a search form (0.0 = no benefit, 1.0 = fully GPU-accelerated).
///
/// Duplicated from `resource::gpu_affinity` to avoid dependency on unregistered module.
fn gpu_affinity_for(form: &str) -> f64 {
    match form {
        "kbn" | "gen_fermat" => 0.9,
        "twin" | "sophie_germain" => 0.8,
        "cullen_woodall" | "carol_kynea" => 0.7,
        "wagstaff" | "repunit" => 0.6,
        "factorial" | "primorial" => 0.5,
        "palindromic" | "near_repdigit" => 0.3,
        _ => 0.3,
    }
}

// ── Per-Form Features ──────────────────────────────────────────

/// Aggregated features for a single search form, combining static properties
/// with live snapshot data and ML model outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFeatures {
    /// Gap between our best and the world record (0.0 = at record, 1.0 = no results).
    pub record_gap: f64,
    /// 30-day rolling yield rate (primes per core-hour).
    pub yield_rate_30d: f64,
    /// Cost efficiency from the cost model.
    pub cost_efficiency: f64,
    /// Fraction of searchable range not yet covered.
    pub opportunity_density: f64,
    /// GPU affinity factor (0.0 = no benefit, 1.0 = fully GPU-accelerated).
    pub gpu_affinity: f64,
    /// External competition pressure (0.0 = no competition, 1.0 = heavily contested).
    pub competition_score: f64,
    /// Latest Thompson sample from the bandit (NaN if not yet available).
    pub bandit_sample: f64,
    /// Mean GP cost prediction uncertainty (NaN if GP not fitted).
    pub gp_uncertainty: f64,
    /// Sieve survival rate from recent blocks (fraction passing sieve).
    pub sieve_survival_rate: f64,
    /// Number of work blocks pending for this form.
    pub blocks_pending: u64,
}

impl Default for FormFeatures {
    fn default() -> Self {
        Self {
            record_gap: 1.0,
            yield_rate_30d: 0.0,
            cost_efficiency: 0.0,
            opportunity_density: 1.0,
            gpu_affinity: 0.0,
            competition_score: 0.5,
            bandit_sample: f64::NAN,
            gp_uncertainty: f64::NAN,
            sieve_survival_rate: 1.0,
            blocks_pending: 0,
        }
    }
}

// ── Per-Node Features ──────────────────────────────────────────

/// Aggregated features for a single compute node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFeatures {
    pub cores: u32,
    pub ram_gb: u32,
    pub node_class: NodeClass,
    pub benchmark_score: f64,
    /// Per-form block completion counts from history.
    pub forms_completed: HashMap<String, u64>,
    /// Anomaly score from EWMA detector (0.0 = normal, 1.0 = anomalous).
    pub anomaly_score: f64,
    /// Cumulative uptime in hours.
    pub uptime_hours: f64,
}

impl Default for NodeFeatures {
    fn default() -> Self {
        Self {
            cores: 0,
            ram_gb: 0,
            node_class: NodeClass::CpuSmall,
            benchmark_score: 0.0,
            forms_completed: HashMap::new(),
            anomaly_score: 0.0,
            uptime_hours: 0.0,
        }
    }
}

// ── Feature Store ──────────────────────────────────────────────

/// Cached feature vectors, refreshed once per OODA tick.
///
/// The store is the single source of truth for ML model inputs within a tick.
/// Call [`FeatureStore::refresh`] at the start of each tick to populate from
/// the latest [`WorldSnapshot`].
#[derive(Debug, Clone, Default)]
pub struct FeatureStore {
    /// Per-form feature vectors keyed by form name.
    pub form_features: HashMap<String, FormFeatures>,
    /// Per-node feature vectors keyed by node/worker ID.
    pub node_features: HashMap<String, NodeFeatures>,
}

impl FeatureStore {
    /// Refresh all features from a [`WorldSnapshot`].
    ///
    /// Called once per tick. Extracts form-level and node-level features
    /// from the snapshot. ML-derived features (bandit_sample, gp_uncertainty)
    /// are populated separately by the MlEngine after subsystem sampling.
    pub fn refresh(&mut self, snapshot: &WorldSnapshot) {
        self.refresh_form_features(snapshot);
        self.refresh_node_features(snapshot);
    }

    fn refresh_form_features(&mut self, snapshot: &WorldSnapshot) {
        for &form in crate::strategy::ALL_FORMS {
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

            let yield_rate_30d = snapshot
                .yield_rates
                .iter()
                .find(|y| y.form == form)
                .map(|y| y.yield_rate)
                .unwrap_or(0.0);

            let gpu_affinity = gpu_affinity_for(form);

            let competition_score = snapshot
                .competition_intel
                .iter()
                .find(|m| m.key == format!("competitive_intel:{}", form))
                .and_then(|m| m.value.parse::<f64>().ok())
                .unwrap_or(0.5);

            let mut ff = FormFeatures {
                record_gap,
                yield_rate_30d,
                gpu_affinity,
                competition_score,
                ..FormFeatures::default()
            };

            // Sieve survival from snapshot worker speeds (approximation)
            ff.sieve_survival_rate = 1.0; // populated from DB in learn phase

            self.form_features.insert(form.to_string(), ff);
        }
    }

    fn refresh_node_features(&mut self, snapshot: &WorldSnapshot) {
        // Populate from worker speed data in snapshot
        let mut node_forms: HashMap<String, HashMap<String, u64>> = HashMap::new();
        for ws in &snapshot.worker_speeds {
            node_forms
                .entry(ws.worker_id.clone())
                .or_default()
                .insert(ws.form.clone(), ws.blocks_completed as u64);
        }

        for (node_id, forms) in node_forms {
            let nf = NodeFeatures {
                forms_completed: forms,
                node_class: NodeClass::CpuMedium, // refined from heartbeat data
                ..NodeFeatures::default()
            };
            self.node_features.insert(node_id, nf);
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_class_classify_cpu() {
        assert_eq!(NodeClass::classify(2, None), NodeClass::CpuSmall);
        assert_eq!(NodeClass::classify(4, None), NodeClass::CpuMedium);
        assert_eq!(NodeClass::classify(8, None), NodeClass::CpuMedium);
        assert_eq!(NodeClass::classify(32, None), NodeClass::CpuLarge);
    }

    #[test]
    fn node_class_classify_gpu() {
        assert_eq!(
            NodeClass::classify(8, Some("cuda-12.1")),
            NodeClass::GpuCuda
        );
        assert_eq!(
            NodeClass::classify(8, Some("metal-3.0")),
            NodeClass::GpuMetal
        );
    }

    #[test]
    fn node_class_as_f64_is_ordered() {
        assert!(NodeClass::CpuSmall.as_f64() < NodeClass::CpuMedium.as_f64());
        assert!(NodeClass::CpuMedium.as_f64() < NodeClass::CpuLarge.as_f64());
        assert!(NodeClass::CpuLarge.as_f64() < NodeClass::GpuCuda.as_f64());
    }

    #[test]
    fn node_class_display() {
        assert_eq!(NodeClass::CpuSmall.to_string(), "cpu_small");
        assert_eq!(NodeClass::GpuCuda.to_string(), "gpu_cuda");
    }

    #[test]
    fn form_features_default() {
        let ff = FormFeatures::default();
        assert_eq!(ff.record_gap, 1.0);
        assert!(ff.bandit_sample.is_nan());
        assert!(ff.gp_uncertainty.is_nan());
    }
}
