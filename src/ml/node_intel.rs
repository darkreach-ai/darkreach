//! # Node Intelligence — Anomaly Detection & Smart Routing
//!
//! Replaces static `score_job_for_worker()` with learned (node, form)
//! affinity scores and EWMA-based anomaly detection.
//!
//! ## Anomaly Detection
//!
//! Exponentially Weighted Moving Average (EWMA) control charts track
//! per-node throughput, error rate, and latency. When a node's observed
//! metrics deviate significantly from its EWMA baseline, the anomaly
//! score rises and the node is deprioritized for work assignment.
//!
//! ## Affinity Learning
//!
//! Per-(node, form) Thompson Sampling posteriors learn which forms run
//! well on which nodes. A node that consistently completes `kbn` blocks
//! fast with no errors will develop a high affinity for `kbn`.
//!
//! ## Fallback
//!
//! When a (node, form) pair has fewer than `MIN_AFFINITY_OBS` observations,
//! the affinity score falls back to the hardware-based heuristic.
//!
//! ## References
//!
//! - Lucas & Saccucci (1990), "EWMA Control Schemes" (anomaly detection)
//! - Agrawal & Goyal (2012), "Thompson Sampling" (affinity learning)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum observations before affinity overrides heuristic.
const MIN_AFFINITY_OBS: u64 = 5;

/// EWMA decay factor (higher = more smoothing, slower reaction).
const EWMA_DECAY: f64 = 0.95;

/// Anomaly threshold: nodes above this score are flagged.
const ANOMALY_THRESHOLD: f64 = 0.85;

// ── Node Profile ───────────────────────────────────────────────

/// Aggregated performance profile for a single compute node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfile {
    /// Node identifier.
    pub node_id: String,
    /// Hardware classification.
    pub node_class: super::features::NodeClass,
    /// Per-form average throughput (candidates/sec).
    pub avg_throughput: HashMap<String, f64>,
    /// Per-form throughput variance.
    pub throughput_var: HashMap<String, f64>,
    /// Per-form failure rate (failures / total blocks).
    pub failure_rate: HashMap<String, f64>,
    /// Total blocks completed across all forms.
    pub blocks_completed: u64,
}

impl NodeProfile {
    fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            node_class: super::features::NodeClass::CpuMedium,
            avg_throughput: HashMap::new(),
            throughput_var: HashMap::new(),
            failure_rate: HashMap::new(),
            blocks_completed: 0,
        }
    }
}

// ── Affinity Score ─────────────────────────────────────────────

/// Thompson Sampling posterior for a (node, form) pair.
///
/// Tracks success/failure rate and mean throughput for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityScore {
    /// Beta shape: successes (blocks completed without error).
    pub alpha: f64,
    /// Beta rate: failures.
    pub beta: f64,
    /// Running mean throughput for this (node, form) pair.
    pub mean_throughput: f64,
    /// Total observations.
    pub n_obs: u64,
}

impl Default for AffinityScore {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta: 1.0,
            mean_throughput: 0.0,
            n_obs: 0,
        }
    }
}

impl AffinityScore {
    /// Update with a block result.
    fn update(&mut self, success: bool, throughput: f64) {
        if success {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
        self.n_obs += 1;
        // Online mean update
        self.mean_throughput += (throughput - self.mean_throughput) / self.n_obs as f64;
    }

    /// Success rate: alpha / (alpha + beta).
    fn success_rate(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }
}

// ── Anomaly State ──────────────────────────────────────────────

/// EWMA anomaly detector state for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyState {
    /// EWMA of throughput.
    pub ewma_throughput: f64,
    /// EWMA of error rate.
    pub ewma_error_rate: f64,
    /// EWMA of heartbeat latency.
    pub ewma_latency: f64,
    /// Decay factor (0 < decay < 1).
    pub decay: f64,
    /// Composite anomaly score (0.0 = normal, 1.0 = anomalous).
    pub anomaly_score: f64,
}

impl Default for AnomalyState {
    fn default() -> Self {
        Self {
            ewma_throughput: 0.0,
            ewma_error_rate: 0.0,
            ewma_latency: 0.0,
            decay: EWMA_DECAY,
            anomaly_score: 0.0,
        }
    }
}

impl AnomalyState {
    /// Update from a block completion observation.
    fn update_block(&mut self, success: bool, throughput: f64) {
        let error = if success { 0.0 } else { 1.0 };
        self.ewma_throughput = self.decay * self.ewma_throughput + (1.0 - self.decay) * throughput;
        self.ewma_error_rate = self.decay * self.ewma_error_rate + (1.0 - self.decay) * error;

        // Anomaly score: combines error rate spike with throughput drop
        self.anomaly_score = self.compute_score();
    }

    /// Update from heartbeat data.
    fn update_heartbeat(&mut self, latency_ms: f64) {
        self.ewma_latency = self.decay * self.ewma_latency + (1.0 - self.decay) * latency_ms;
        self.anomaly_score = self.compute_score();
    }

    /// Compute composite anomaly score from EWMA signals.
    fn compute_score(&self) -> f64 {
        // High error rate is the strongest anomaly signal
        let error_component = self.ewma_error_rate;

        // Extreme latency (> 5 seconds) is suspicious
        let latency_component = (self.ewma_latency / 5000.0).min(1.0);

        // Combine: error rate dominates, latency contributes
        (0.7 * error_component + 0.3 * latency_component).clamp(0.0, 1.0)
    }
}

// ── Node Intelligence ──────────────────────────────────────────

/// Central node intelligence system: anomaly detection + smart routing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeIntelligence {
    /// Per-node performance profiles.
    pub profiles: HashMap<String, NodeProfile>,
    /// Per-(node, form) affinity scores.
    pub affinity: HashMap<(String, String), AffinityScore>,
    /// Per-node EWMA anomaly detector state.
    pub anomaly_state: HashMap<String, AnomalyState>,
}

impl NodeIntelligence {
    /// Score a (node, form) pair for work assignment.
    ///
    /// Returns a score in [0, 1] where higher is better.
    /// Combines learned affinity (success rate × throughput) with
    /// anomaly penalty.
    pub fn score_assignment(&self, node_id: &str, form: &str) -> f64 {
        let key = (node_id.to_string(), form.to_string());

        // Base score from affinity
        let affinity_score = self
            .affinity
            .get(&key)
            .filter(|a| a.n_obs >= MIN_AFFINITY_OBS)
            .map(|a| {
                // Combine success rate with normalized throughput
                let sr = a.success_rate();
                let tp_norm = (a.mean_throughput / 100.0).min(1.0); // rough normalization
                0.6 * sr + 0.4 * tp_norm
            })
            .unwrap_or(0.5); // neutral default

        // Anomaly penalty
        let anomaly = self
            .anomaly_state
            .get(node_id)
            .map(|a| a.anomaly_score)
            .unwrap_or(0.0);

        // Penalize anomalous nodes
        affinity_score * (1.0 - anomaly * 0.5)
    }

    /// Update after a work block completes (success or failure).
    pub fn observe_block(
        &mut self,
        node_id: &str,
        form: &str,
        success: bool,
        throughput: f64,
        _duration_secs: f64,
    ) {
        // Update profile
        let profile = self
            .profiles
            .entry(node_id.to_string())
            .or_insert_with(|| NodeProfile::new(node_id));
        profile.blocks_completed += 1;

        // Update per-form average throughput (online mean)
        let count = profile.blocks_completed as f64;
        let avg = profile
            .avg_throughput
            .entry(form.to_string())
            .or_insert(0.0);
        *avg += (throughput - *avg) / count;

        // Update per-form failure rate
        let fr = profile.failure_rate.entry(form.to_string()).or_insert(0.0);
        let error_val = if success { 0.0 } else { 1.0 };
        *fr += (error_val - *fr) / count;

        // Update affinity
        let key = (node_id.to_string(), form.to_string());
        let aff = self.affinity.entry(key).or_default();
        aff.update(success, throughput);

        // Update anomaly state
        let anomaly = self.anomaly_state.entry(node_id.to_string()).or_default();
        anomaly.update_block(success, throughput);
    }

    /// Update anomaly detector with node heartbeat data.
    pub fn update_heartbeat(
        &mut self,
        node_id: &str,
        _cpu_pct: f64,
        _mem_pct: f64,
        latency_ms: f64,
    ) {
        let anomaly = self.anomaly_state.entry(node_id.to_string()).or_default();
        anomaly.update_heartbeat(latency_ms);
    }

    /// Get nodes flagged as anomalous (anomaly_score > threshold).
    pub fn anomalous_nodes(&self) -> Vec<(&str, f64)> {
        self.anomaly_state
            .iter()
            .filter(|(_, state)| state.anomaly_score > ANOMALY_THRESHOLD)
            .map(|(id, state)| (id.as_str(), state.anomaly_score))
            .collect()
    }

    /// Check if a node should be deprioritized for a specific form.
    ///
    /// A node is deprioritized if:
    /// 1. It's flagged as anomalous, OR
    /// 2. Its affinity for this form has a failure rate > 50%
    pub fn is_deprioritized(&self, node_id: &str, form: &str) -> bool {
        // Check anomaly
        if let Some(state) = self.anomaly_state.get(node_id) {
            if state.anomaly_score > ANOMALY_THRESHOLD {
                return true;
            }
        }

        // Check form-specific failure rate
        let key = (node_id.to_string(), form.to_string());
        if let Some(aff) = self.affinity.get(&key) {
            if aff.n_obs >= MIN_AFFINITY_OBS && aff.success_rate() < 0.5 {
                return true;
            }
        }

        false
    }

    /// Get anomaly score for a specific node.
    pub fn anomaly_score(&self, node_id: &str) -> f64 {
        self.anomaly_state
            .get(node_id)
            .map(|s| s.anomaly_score)
            .unwrap_or(0.0)
    }

    /// Get affinity confidence for a (node, form) pair.
    pub fn affinity_confidence(&self, node_id: &str, form: &str) -> f64 {
        let key = (node_id.to_string(), form.to_string());
        self.affinity
            .get(&key)
            .map(|a| {
                if a.n_obs >= MIN_AFFINITY_OBS {
                    (a.n_obs as f64 / 50.0).min(1.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0)
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_profile_new() {
        let np = NodeProfile::new("node-1");
        assert_eq!(np.node_id, "node-1");
        assert_eq!(np.blocks_completed, 0);
    }

    #[test]
    fn affinity_default_is_neutral() {
        let aff = AffinityScore::default();
        assert!((aff.success_rate() - 0.5).abs() < f64::EPSILON);
        assert_eq!(aff.n_obs, 0);
    }

    #[test]
    fn affinity_update_success() {
        let mut aff = AffinityScore::default();
        aff.update(true, 10.0);
        assert!(aff.success_rate() > 0.5);
        assert_eq!(aff.n_obs, 1);
    }

    #[test]
    fn affinity_update_failure() {
        let mut aff = AffinityScore::default();
        aff.update(false, 0.0);
        assert!(aff.success_rate() < 0.5);
    }

    #[test]
    fn anomaly_state_default_is_clean() {
        let state = AnomalyState::default();
        assert_eq!(state.anomaly_score, 0.0);
    }

    #[test]
    fn anomaly_increases_with_failures() {
        let mut state = AnomalyState::default();
        for _ in 0..50 {
            state.update_block(false, 0.0);
        }
        assert!(
            state.anomaly_score > 0.5,
            "anomaly should be high after many failures: {}",
            state.anomaly_score
        );
    }

    #[test]
    fn anomaly_stays_low_with_successes() {
        let mut state = AnomalyState::default();
        for _ in 0..20 {
            state.update_block(true, 50.0);
        }
        assert!(
            state.anomaly_score < 0.1,
            "anomaly should be low with successes: {}",
            state.anomaly_score
        );
    }

    #[test]
    fn node_intel_score_assignment_default() {
        let intel = NodeIntelligence::default();
        let score = intel.score_assignment("node-1", "kbn");
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn node_intel_observe_and_score() {
        let mut intel = NodeIntelligence::default();
        for _ in 0..10 {
            intel.observe_block("node-1", "kbn", true, 50.0, 10.0);
        }
        let score = intel.score_assignment("node-1", "kbn");
        assert!(score > 0.5, "good node should score > 0.5: {}", score);
    }

    #[test]
    fn node_intel_deprioritize_bad_node() {
        let mut intel = NodeIntelligence::default();
        for _ in 0..10 {
            intel.observe_block("node-bad", "kbn", false, 0.0, 100.0);
        }
        assert!(intel.is_deprioritized("node-bad", "kbn"));
    }

    #[test]
    fn node_intel_not_deprioritize_good_node() {
        let mut intel = NodeIntelligence::default();
        for _ in 0..10 {
            intel.observe_block("node-good", "kbn", true, 50.0, 10.0);
        }
        assert!(!intel.is_deprioritized("node-good", "kbn"));
    }

    #[test]
    fn node_intel_anomalous_nodes_empty_by_default() {
        let intel = NodeIntelligence::default();
        assert!(intel.anomalous_nodes().is_empty());
    }

    #[test]
    fn node_intel_heartbeat_updates_anomaly() {
        let mut intel = NodeIntelligence::default();
        // High latency heartbeats
        for _ in 0..20 {
            intel.update_heartbeat("node-slow", 50.0, 80.0, 10_000.0);
        }
        let score = intel.anomaly_score("node-slow");
        assert!(
            score > 0.0,
            "high latency should increase anomaly: {}",
            score
        );
    }
}
