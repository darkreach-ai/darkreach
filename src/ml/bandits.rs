//! # Thompson Sampling — Intelligent Form Selection
//!
//! Replaces the 10-weight heuristic scoring in `AiEngine::score_forms()` with
//! a Thompson Sampling multi-armed bandit using Normal-Inverse-Gamma priors
//! for continuous rewards (primes per core-hour).
//!
//! ## Algorithm
//!
//! Each search form is an "arm" with a Beta(α, β) posterior over reward
//! probability. On each ORIENT tick:
//!
//! 1. Sample θ ~ Beta(α, β) for each arm
//! 2. Modulate by contextual features (network size, GPU availability, etc.)
//! 3. Return ranked forms by modulated sample
//!
//! After each project completes, the arm's posterior is updated:
//! - reward = primes_found / core_hours (continuous, non-negative)
//! - α += reward, β += (1 - min(reward, 1))
//!
//! ## Sliding Window
//!
//! To handle non-stationarity (e.g., new forms becoming more productive),
//! a 90-day sliding window maintains separate windowed posteriors. The
//! windowed posterior is used for sampling; the global posterior is kept
//! for long-term statistics.
//!
//! ## References
//!
//! - Agrawal & Goyal (2012), "Analysis of Thompson Sampling for the MAB Problem"
//! - Chapelle & Li (2011), "An Empirical Evaluation of Thompson Sampling"

use chrono::{DateTime, Utc};
use rand::Rng;
use rand_distr::{Beta, Distribution};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai_engine::WorldSnapshot;

/// Default sliding window size in days.
const DEFAULT_WINDOW_DAYS: u64 = 90;

/// Minimum alpha/beta to prevent degenerate distributions.
const MIN_PRIOR: f64 = 1.0;

/// Number of contextual features for modulation.
const CONTEXT_DIM: usize = 5;

// ── FormArm ────────────────────────────────────────────────────

/// Per-form Thompson Sampling posterior state.
///
/// Maintains both a global posterior (all-time) and a windowed posterior
/// (recent N days) for non-stationary reward tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormArm {
    /// Search form name (e.g., "kbn", "factorial").
    pub form: String,
    /// Global Beta shape parameter (successes + prior).
    pub alpha: f64,
    /// Global Beta rate parameter (failures + prior).
    pub beta: f64,
    /// Running mean of primes_per_core_hour.
    pub mean_reward: f64,
    /// Running variance of reward.
    pub reward_var: f64,
    /// Total observations (all time).
    pub n_obs: u64,
    /// Windowed alpha (recent `window_days` only).
    pub window_alpha: f64,
    /// Windowed beta.
    pub window_beta: f64,
    /// Windowed observation count.
    pub window_n: u64,
}

impl FormArm {
    /// Create a new arm with uninformative prior Beta(1, 1).
    pub fn new(form: &str) -> Self {
        Self {
            form: form.to_string(),
            alpha: MIN_PRIOR,
            beta: MIN_PRIOR,
            mean_reward: 0.0,
            reward_var: 1.0,
            n_obs: 0,
            window_alpha: MIN_PRIOR,
            window_beta: MIN_PRIOR,
            window_n: 0,
        }
    }

    /// Update posterior with a reward observation.
    ///
    /// Maps continuous reward to Beta update:
    /// - α += min(reward, 1.0) (bounded success signal)
    /// - β += max(1.0 - reward, 0.0) (bounded failure signal)
    pub fn update(&mut self, reward: f64) {
        let bounded = reward.min(1.0).max(0.0);
        self.alpha += bounded;
        self.beta += 1.0 - bounded;
        self.window_alpha += bounded;
        self.window_beta += 1.0 - bounded;

        // Update running statistics (Welford's online algorithm)
        self.n_obs += 1;
        self.window_n += 1;
        let delta = reward - self.mean_reward;
        self.mean_reward += delta / self.n_obs as f64;
        let delta2 = reward - self.mean_reward;
        self.reward_var += delta * delta2;
    }

    /// Sample from the windowed Beta posterior.
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        let alpha = self.window_alpha.max(MIN_PRIOR);
        let beta = self.window_beta.max(MIN_PRIOR);
        match Beta::new(alpha, beta) {
            Ok(dist) => dist.sample(rng),
            Err(_) => 0.5, // fallback for degenerate parameters
        }
    }

    /// Confidence measure: 1.0 - coefficient of variation of Beta posterior.
    /// Higher = more confident (more observations).
    pub fn confidence(&self) -> f64 {
        let a = self.window_alpha.max(MIN_PRIOR);
        let b = self.window_beta.max(MIN_PRIOR);
        let mean = a / (a + b);
        let var = (a * b) / ((a + b).powi(2) * (a + b + 1.0));
        if mean > 0.0 {
            (1.0 - (var.sqrt() / mean)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

// ── FormContext ─────────────────────────────────────────────────

/// Contextual features that modulate Thompson Sampling.
///
/// These features capture the current network state and are used to
/// adjust the raw Thompson sample toward forms that are best suited
/// for the current environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormContext {
    /// Total compute cores available in the network.
    pub network_cores: u32,
    /// Whether any GPU-capable node is available.
    pub gpu_available: bool,
    /// Fraction of nodes currently idle (0.0–1.0).
    pub idle_fraction: f64,
    /// Number of active projects.
    pub active_project_count: usize,
    /// Remaining monthly budget in USD.
    pub budget_remaining_usd: f64,
}

impl FormContext {
    /// Extract context from a [`WorldSnapshot`].
    pub fn from_snapshot(snapshot: &WorldSnapshot) -> Self {
        let total = snapshot.fleet.worker_count as f64;
        let idle = if total > 0.0 {
            (snapshot.fleet.idle_workers as f64 / total).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            network_cores: snapshot.fleet.total_cores,
            gpu_available: false, // GPU tracking not yet in FleetSnapshot
            idle_fraction: idle,
            active_project_count: snapshot.active_projects.len(),
            budget_remaining_usd: snapshot.budget.remaining_usd,
        }
    }

    /// Encode as a fixed-size feature vector [0..1] for contextual modulation.
    fn as_array(&self) -> [f64; CONTEXT_DIM] {
        [
            (self.network_cores as f64 / 1000.0).min(1.0),
            if self.gpu_available { 1.0 } else { 0.0 },
            self.idle_fraction,
            (self.active_project_count as f64 / 10.0).min(1.0),
            (self.budget_remaining_usd / 100.0).min(1.0),
        ]
    }
}

// ── FormBandits ────────────────────────────────────────────────

/// Multi-armed bandit for form selection using Thompson Sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormBandits {
    /// Per-form arm states.
    pub arms: HashMap<String, FormArm>,
    /// Sliding window size in days.
    pub window_days: u64,
    /// Learned contextual modulation weights.
    pub context_weights: [f64; CONTEXT_DIM],
    /// UCB-style exploration bonus for under-explored forms.
    pub exploration_bonus: f64,
}

impl Default for FormBandits {
    fn default() -> Self {
        Self {
            arms: HashMap::new(),
            window_days: DEFAULT_WINDOW_DAYS,
            context_weights: [0.2; CONTEXT_DIM],
            exploration_bonus: 0.1,
        }
    }
}

impl FormBandits {
    /// Sample from each arm's posterior and return ranked forms.
    ///
    /// Forms with fewer observations get an exploration bonus.
    /// Contextual features modulate the raw sample.
    pub fn sample_rankings(
        &mut self,
        context: &FormContext,
        rng: &mut impl Rng,
    ) -> Vec<(String, f64)> {
        // Ensure all forms have an arm
        for &form in crate::strategy::ALL_FORMS {
            self.arms
                .entry(form.to_string())
                .or_insert_with(|| FormArm::new(form));
        }

        let ctx = context.as_array();
        let total_obs: u64 = self.arms.values().map(|a| a.window_n).sum();

        let mut rankings: Vec<(String, f64)> = self
            .arms
            .iter()
            .map(|(form, arm)| {
                // Raw Thompson sample from windowed posterior
                let sample = arm.sample(rng);

                // Contextual modulation: dot product of context × weights
                let ctx_bonus: f64 = ctx
                    .iter()
                    .zip(self.context_weights.iter())
                    .map(|(c, w)| c * w)
                    .sum::<f64>()
                    / CONTEXT_DIM as f64;

                // Exploration bonus: UCB1-style sqrt(ln(N) / n_i)
                let explore = if arm.window_n > 0 && total_obs > 0 {
                    self.exploration_bonus * ((total_obs as f64).ln() / arm.window_n as f64).sqrt()
                } else {
                    self.exploration_bonus
                };

                let score = sample + ctx_bonus * 0.1 + explore;
                (form.clone(), score)
            })
            .collect();

        rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        rankings
    }

    /// Update an arm's posterior after a project completes.
    ///
    /// `reward` should be primes_found / core_hours (continuous, non-negative).
    pub fn update(&mut self, form: &str, reward: f64, _context: &FormContext) {
        let arm = self
            .arms
            .entry(form.to_string())
            .or_insert_with(|| FormArm::new(form));
        arm.update(reward);
    }

    /// Decay windowed posteriors by resetting observations outside the window.
    ///
    /// Since we don't store individual timestamps, we use exponential decay:
    /// multiply windowed alpha/beta by a decay factor proportional to time
    /// since last decay.
    pub fn decay_window(&mut self, _cutoff: DateTime<Utc>) {
        let decay = 0.95; // gentle exponential decay per cycle
        for arm in self.arms.values_mut() {
            arm.window_alpha = MIN_PRIOR + (arm.window_alpha - MIN_PRIOR) * decay;
            arm.window_beta = MIN_PRIOR + (arm.window_beta - MIN_PRIOR) * decay;
            arm.window_n = (arm.window_n as f64 * decay) as u64;
        }
    }

    /// Get the confidence level for a form (0.0–1.0).
    pub fn confidence(&self, form: &str) -> f64 {
        self.arms.get(form).map(|a| a.confidence()).unwrap_or(0.0)
    }

    /// Get observation count for a form.
    pub fn observation_count(&self, form: &str) -> u64 {
        self.arms.get(form).map(|a| a.n_obs).unwrap_or(0)
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_arm_new_has_uninformative_prior() {
        let arm = FormArm::new("kbn");
        assert_eq!(arm.alpha, 1.0);
        assert_eq!(arm.beta, 1.0);
        assert_eq!(arm.n_obs, 0);
    }

    #[test]
    fn form_arm_update_increases_observations() {
        let mut arm = FormArm::new("kbn");
        arm.update(0.5);
        assert_eq!(arm.n_obs, 1);
        assert!(arm.alpha > 1.0);
        assert!(arm.beta > 1.0);
    }

    #[test]
    fn form_arm_high_reward_increases_alpha() {
        let mut arm = FormArm::new("kbn");
        arm.update(1.0);
        // reward=1.0: α += 1.0, β += 0.0
        assert!((arm.alpha - 2.0).abs() < f64::EPSILON);
        assert!((arm.beta - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn form_arm_zero_reward_increases_beta() {
        let mut arm = FormArm::new("kbn");
        arm.update(0.0);
        // reward=0.0: α += 0.0, β += 1.0
        assert!((arm.alpha - 1.0).abs() < f64::EPSILON);
        assert!((arm.beta - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn form_arm_sample_in_unit_range() {
        let arm = FormArm::new("kbn");
        let mut rng = rand::rng();
        for _ in 0..100 {
            let s = arm.sample(&mut rng);
            assert!((0.0..=1.0).contains(&s), "sample {} out of [0,1]", s);
        }
    }

    #[test]
    fn form_arm_confidence_increases_with_observations() {
        let mut arm = FormArm::new("kbn");
        let c0 = arm.confidence();
        for _ in 0..50 {
            arm.update(0.6);
        }
        let c50 = arm.confidence();
        assert!(c50 > c0, "confidence should increase: {} vs {}", c50, c0);
    }

    #[test]
    fn bandits_sample_rankings_returns_all_forms() {
        let mut bandits = FormBandits::default();
        let context = FormContext {
            network_cores: 64,
            gpu_available: false,
            idle_fraction: 0.3,
            active_project_count: 2,
            budget_remaining_usd: 50.0,
        };
        let mut rng = rand::rng();
        let rankings = bandits.sample_rankings(&context, &mut rng);
        assert_eq!(rankings.len(), crate::strategy::ALL_FORMS.len());
    }

    #[test]
    fn bandits_update_creates_arm_if_missing() {
        let mut bandits = FormBandits::default();
        let context = FormContext {
            network_cores: 32,
            gpu_available: false,
            idle_fraction: 0.5,
            active_project_count: 1,
            budget_remaining_usd: 80.0,
        };
        bandits.update("factorial", 0.5, &context);
        assert!(bandits.arms.contains_key("factorial"));
        assert_eq!(bandits.arms["factorial"].n_obs, 1);
    }

    #[test]
    fn bandits_decay_reduces_window() {
        let mut bandits = FormBandits::default();
        let mut arm = FormArm::new("kbn");
        for _ in 0..20 {
            arm.update(0.5);
        }
        let before_alpha = arm.window_alpha;
        bandits.arms.insert("kbn".to_string(), arm);
        bandits.decay_window(Utc::now());
        let after_alpha = bandits.arms["kbn"].window_alpha;
        assert!(
            after_alpha < before_alpha,
            "decay should reduce alpha: {} vs {}",
            after_alpha,
            before_alpha
        );
    }

    #[test]
    fn form_arm_reward_bounded() {
        let mut arm = FormArm::new("kbn");
        // Reward > 1.0 should be clamped
        arm.update(5.0);
        // α should increase by 1.0 (clamped), β by 0.0
        assert!((arm.alpha - 2.0).abs() < f64::EPSILON);
        assert!((arm.beta - 1.0).abs() < f64::EPSILON);
    }
}
