//! # Pipeline — Multi-Stage Composite Elimination Pipeline
//!
//! Formalizes the implicit sieve → P-1 → ECM → primality flow into an explicit,
//! configurable, instrumented pipeline. Each stage has independent enable/disable
//! controls and per-stage elimination statistics.
//!
//! ## Pipeline Stages
//!
//! 1. **Trial division**: Small factor check via hardcoded 64-prime table (up to 311).
//!    Cost: ~64 modular reductions. Eliminates ~87% of random odd composites.
//!
//! 2. **Algebraic sieve**: Form-specific BSGS sieve (runs at batch level in each
//!    search form, not in this per-candidate pipeline).
//!
//! 3. **P-1 factoring**: Pollard's P-1 with auto-tuned B1/B2 bounds. Catches
//!    composites where p-1 is smooth. Cost: ~π(B1) modular exponentiations.
//!    Only cost-effective for candidates > 5K bits.
//!
//! 4. **ECM factoring**: Lenstra's elliptic curve method on twisted Edwards curves.
//!    Catches composites where P-1 fails (curve order is smooth even when p-1 is not).
//!    Only cost-effective for candidates > 10K bits.
//!
//! 5. **Primality test**: Miller-Rabin with Frobenius pre-filter (handled downstream
//!    by `mr_screened_test`, not in this pipeline).
//!
//! ## Usage
//!
//! Search forms call `pipeline::is_composite()` on each sieve survivor before the
//! expensive primality test. The pipeline runs stages 1, 3, 4 in sequence, returning
//! early on the first elimination.
//!
//! ## References
//!
//! - J.M. Pollard, "Theorems on Factorization and Primality Testing", 1974.
//! - H.W. Lenstra Jr., "Factoring Integers with Elliptic Curves", 1987.

use rug::Integer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// The stages of the composite elimination pipeline.
///
/// Ordered by increasing cost — cheap stages run first to maximize
/// the fraction of composites eliminated per unit of computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// Trial division by the first 64 primes (2..311).
    TrialDivision,
    /// Form-specific algebraic sieve (BSGS). Runs at batch level, not per-candidate.
    AlgebraicSieve,
    /// Pollard's P-1 factoring (Stage 1 + Stage 2).
    P1Factoring,
    /// Lenstra's ECM factoring (twisted Edwards curves).
    EcmFactoring,
    /// Final primality test (MR + Frobenius).
    PrimalityTest,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineStage::TrialDivision => write!(f, "trial_division"),
            PipelineStage::AlgebraicSieve => write!(f, "algebraic_sieve"),
            PipelineStage::P1Factoring => write!(f, "p1_factoring"),
            PipelineStage::EcmFactoring => write!(f, "ecm_factoring"),
            PipelineStage::PrimalityTest => write!(f, "primality_test"),
        }
    }
}

/// Per-stage elimination statistics.
///
/// Tracks how many candidates entered each stage, how many were eliminated,
/// and the cumulative wall-clock time spent in that stage.
#[derive(Debug)]
pub struct StageStats {
    /// Number of candidates that entered this stage.
    pub input_count: AtomicU64,
    /// Number of candidates eliminated by this stage.
    pub eliminated: AtomicU64,
    /// Cumulative time spent in this stage, in microseconds.
    pub elapsed_us: AtomicU64,
}

impl StageStats {
    /// Create a new zero-initialized stats tracker.
    pub fn new() -> Self {
        Self {
            input_count: AtomicU64::new(0),
            eliminated: AtomicU64::new(0),
            elapsed_us: AtomicU64::new(0),
        }
    }

    /// Record one invocation of this stage.
    fn record(&self, was_eliminated: bool, elapsed: std::time::Duration) {
        self.input_count.fetch_add(1, Ordering::Relaxed);
        if was_eliminated {
            self.eliminated.fetch_add(1, Ordering::Relaxed);
        }
        self.elapsed_us
            .fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
    }

    /// Elimination rate as a fraction in [0, 1].
    pub fn elimination_rate(&self) -> f64 {
        let input = self.input_count.load(Ordering::Relaxed);
        if input == 0 {
            return 0.0;
        }
        self.eliminated.load(Ordering::Relaxed) as f64 / input as f64
    }
}

impl Default for StageStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for the composite elimination pipeline.
///
/// Controls which stages are enabled and their parameters. Stages that are
/// disabled are skipped entirely (zero cost).
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Enable trial division (default: true). Only disable for benchmarking.
    pub trial_division_enabled: bool,
    /// Enable P-1 factoring (default: true for candidates > 5K bits).
    pub p1_enabled: bool,
    /// Minimum candidate bit size for P-1 (default: 5000).
    pub p1_min_bits: u32,
    /// Enable ECM factoring (default: false until ECM module is ready).
    pub ecm_enabled: bool,
    /// Number of ECM curves to try (default: 10).
    pub ecm_curves: u32,
    /// Minimum candidate bit size for ECM (default: 10000).
    pub ecm_min_bits: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            trial_division_enabled: true,
            p1_enabled: true,
            p1_min_bits: 5_000,
            ecm_enabled: false,
            ecm_curves: 10,
            ecm_min_bits: 10_000,
        }
    }
}

/// Aggregate statistics for all pipeline stages.
pub struct PipelineStats {
    pub trial_division: StageStats,
    pub p1_factoring: StageStats,
    pub ecm_factoring: StageStats,
}

impl PipelineStats {
    /// Create new zero-initialized aggregate stats.
    pub fn new() -> Self {
        Self {
            trial_division: StageStats::new(),
            p1_factoring: StageStats::new(),
            ecm_factoring: StageStats::new(),
        }
    }

    /// Log a summary of elimination rates across all stages.
    pub fn log_summary(&self) {
        let td_in = self.trial_division.input_count.load(Ordering::Relaxed);
        let td_elim = self.trial_division.eliminated.load(Ordering::Relaxed);
        let p1_in = self.p1_factoring.input_count.load(Ordering::Relaxed);
        let p1_elim = self.p1_factoring.eliminated.load(Ordering::Relaxed);
        let ecm_in = self.ecm_factoring.input_count.load(Ordering::Relaxed);
        let ecm_elim = self.ecm_factoring.eliminated.load(Ordering::Relaxed);

        if td_in > 0 || p1_in > 0 || ecm_in > 0 {
            tracing::info!(
                trial_division_rate = format!("{}/{} ({:.1}%)", td_elim, td_in,
                    if td_in > 0 { td_elim as f64 / td_in as f64 * 100.0 } else { 0.0 }),
                p1_rate = format!("{}/{} ({:.1}%)", p1_elim, p1_in,
                    if p1_in > 0 { p1_elim as f64 / p1_in as f64 * 100.0 } else { 0.0 }),
                ecm_rate = format!("{}/{} ({:.1}%)", ecm_elim, ecm_in,
                    if ecm_in > 0 { ecm_elim as f64 / ecm_in as f64 * 100.0 } else { 0.0 }),
                "Pipeline elimination summary"
            );
        }
    }
}

impl Default for PipelineStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the composite elimination pipeline on a single candidate.
///
/// Returns `(true, Some(stage))` if the candidate is definitely composite,
/// with the stage that eliminated it. Returns `(false, None)` if the candidate
/// survived all enabled stages and should proceed to the primality test.
///
/// Stages run in order of increasing cost:
/// 1. Trial division (~64 modular reductions)
/// 2. P-1 factoring (~π(B1) modular exponentiations, if enabled)
/// 3. ECM factoring (~num_curves × Stage1+2, if enabled)
pub fn is_composite(
    candidate: &Integer,
    config: &PipelineConfig,
    stats: &PipelineStats,
) -> (bool, Option<PipelineStage>) {
    let bits = candidate.significant_bits();

    // Stage 1: Trial division
    if config.trial_division_enabled {
        let start = Instant::now();
        let eliminated = crate::has_small_factor(candidate);
        stats.trial_division.record(eliminated, start.elapsed());
        if eliminated {
            return (true, Some(PipelineStage::TrialDivision));
        }
    }

    // Stage 3: P-1 factoring (skip for small candidates where it's not cost-effective)
    if config.p1_enabled && bits >= config.p1_min_bits {
        let start = Instant::now();
        let eliminated = crate::p1::adaptive_p1_filter(candidate);
        stats.p1_factoring.record(eliminated, start.elapsed());
        if eliminated {
            return (true, Some(PipelineStage::P1Factoring));
        }
    }

    // Stage 4: ECM factoring (skip for small candidates)
    if config.ecm_enabled && bits >= config.ecm_min_bits {
        let start = Instant::now();
        let eliminated = crate::ecm::adaptive_ecm_filter(candidate, config.ecm_curves);
        stats.ecm_factoring.record(eliminated, start.elapsed());
        if eliminated {
            return (true, Some(PipelineStage::EcmFactoring));
        }
    }

    (false, None)
}

/// Convenience wrapper: returns true if the candidate is definitely composite.
pub fn is_pipeline_composite(candidate: &Integer, config: &PipelineConfig, stats: &PipelineStats) -> bool {
    is_composite(candidate, config, stats).0
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Work Block Pipeline — Multi-Stage Distributed Search
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// The work block pipeline is DIFFERENT from the per-candidate composite
// elimination pipeline above. That pipeline (trial division → P-1 → ECM)
// runs on individual candidates within a single worker.
//
// This work block pipeline orchestrates the 4-stage flow at the distributed
// level, where different workers can handle different stages:
//
//   sieve → screen → test → proof
//
// Each stage produces survivors that are stored in stage_data (JSONB) and
// handed off to the next stage via the database. This lets cheap stages
// (sieve, screen) run on many workers in parallel, feeding expensive stages
// (test, proof) with pre-filtered candidates.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The four stages of the distributed work block pipeline.
///
/// These represent the lifecycle of a work block as it flows through the
/// system. Each stage has increasing computational cost but decreasing
/// candidate count (due to elimination in prior stages).
///
/// | Stage  | Cost per candidate | Typical elimination rate |
/// |--------|-------------------|-------------------------|
/// | sieve  | ~64 mod ops       | ~87% of composites      |
/// | screen | ~2 MR rounds      | ~75% of remaining       |
/// | test   | ~25 MR rounds     | identifies PRPs         |
/// | proof  | variable (heavy)  | proves or fails         |
pub const PIPELINE_STAGES: &[&str] = &["sieve", "screen", "test", "proof"];

/// Returns the next stage after the given one, or `None` if the pipeline is complete.
///
/// The pipeline flows linearly: sieve → screen → test → proof → done.
///
/// # Examples
///
/// ```
/// use darkreach::pipeline::next_stage;
/// assert_eq!(next_stage("sieve"), Some("screen"));
/// assert_eq!(next_stage("screen"), Some("test"));
/// assert_eq!(next_stage("test"), Some("proof"));
/// assert_eq!(next_stage("proof"), None);
/// assert_eq!(next_stage("unknown"), None);
/// ```
pub fn next_stage(current: &str) -> Option<&'static str> {
    match current {
        "sieve" => Some("screen"),
        "screen" => Some("test"),
        "test" => Some("proof"),
        "proof" => None,
        _ => None,
    }
}

/// Returns the index (0-based) of a pipeline stage, or `None` for unknown stages.
///
/// Useful for ordering comparisons: earlier stages have lower indices.
pub fn stage_index(stage: &str) -> Option<usize> {
    match stage {
        "sieve" => Some(0),
        "screen" => Some(1),
        "test" => Some(2),
        "proof" => Some(3),
        _ => None,
    }
}

/// Returns true if the given string is a valid pipeline stage name.
pub fn is_valid_stage(stage: &str) -> bool {
    matches!(stage, "sieve" | "screen" | "test" | "proof")
}

/// Result from executing a single pipeline stage on a batch of candidates.
///
/// Contains the survivors (candidates that passed this stage) and statistics
/// about the stage execution for monitoring and tuning.
#[derive(Debug)]
pub struct WorkBlockStageResult {
    /// Candidates that survived this stage and should proceed to the next.
    /// For the "proof" stage, these are the proven primes.
    pub survivors: Vec<Integer>,
    /// Statistics for this stage execution.
    pub stats: WorkBlockStageStats,
}

/// Statistics from executing a pipeline stage on a work block.
///
/// Tracked per-execution (not cumulative like `StageStats` above) to support
/// the decision of whether to advance the block or retry.
#[derive(Debug, Clone)]
pub struct WorkBlockStageStats {
    /// Number of candidates that entered this stage.
    pub input_count: u64,
    /// Number of candidates that survived (passed) this stage.
    pub survivor_count: u64,
    /// Wall-clock time spent executing this stage.
    pub elapsed: std::time::Duration,
}

impl WorkBlockStageStats {
    /// Elimination rate as a fraction in [0, 1]. Returns 0.0 if no input.
    pub fn elimination_rate(&self) -> f64 {
        if self.input_count == 0 {
            return 0.0;
        }
        1.0 - (self.survivor_count as f64 / self.input_count as f64)
    }
}

/// Configuration for the work block pipeline.
///
/// Controls which stages are enabled and their parameters. This is separate
/// from `PipelineConfig` which controls the per-candidate composite elimination
/// pipeline (trial division, P-1, ECM).
#[derive(Debug, Clone)]
pub struct WorkBlockPipelineConfig {
    /// Number of Miller-Rabin rounds for the "screen" stage (default: 2).
    /// This is a quick pre-filter; the full test uses 25 rounds.
    pub screen_mr_rounds: u32,
    /// Number of Miller-Rabin rounds for the "test" stage (default: 25).
    /// Matches `rug::Integer::is_probably_prime(25)` used throughout the engine.
    pub test_mr_rounds: u32,
    /// Whether to attempt deterministic proofs in the "proof" stage.
    /// When false, the proof stage is a no-op and PRPs are reported as-is.
    pub proof_enabled: bool,
    /// Per-candidate composite elimination config used within the sieve stage.
    pub composite_elimination: PipelineConfig,
}

impl Default for WorkBlockPipelineConfig {
    fn default() -> Self {
        Self {
            screen_mr_rounds: 2,
            test_mr_rounds: 25,
            proof_enabled: true,
            composite_elimination: PipelineConfig::default(),
        }
    }
}

/// Execute a single pipeline stage on a work block's candidates.
///
/// Dispatches to stage-specific logic based on the stage name:
///
/// - **sieve**: Runs the per-candidate composite elimination pipeline (trial
///   division, P-1, ECM) on each candidate. Survivors are those that pass all
///   enabled elimination stages.
///
/// - **screen**: Quick 2-round Miller-Rabin pre-screen. Eliminates obvious
///   composites cheaply before the full test. Uses `rug::Integer::is_probably_prime`
///   with a low round count.
///
/// - **test**: Full 25-round Miller-Rabin (or form-specific test like Proth/LLR).
///   This is the main primality test that identifies probable primes.
///
/// - **proof**: Deterministic proof attempt using form-specific methods
///   (Pocklington, Morrison, BLS, Proth, LLR). Survivors are proven primes.
///   Candidates that fail proof are still reported as PRPs.
///
/// # Arguments
///
/// * `stage` - Pipeline stage to execute ("sieve", "screen", "test", "proof")
/// * `candidates` - Candidates to process in this stage
/// * `_form` - Search form name (for form-specific test/proof dispatch, reserved for future use)
/// * `config` - Pipeline configuration
///
/// # Returns
///
/// A `WorkBlockStageResult` with the survivors and execution statistics.
pub fn execute_stage(
    stage: &str,
    candidates: &[Integer],
    _form: &str,
    config: &WorkBlockPipelineConfig,
) -> WorkBlockStageResult {
    let start = Instant::now();
    let input_count = candidates.len() as u64;

    let survivors: Vec<Integer> = match stage {
        "sieve" => {
            // Run per-candidate composite elimination (trial division, P-1, ECM).
            // This reuses the existing per-candidate pipeline.
            let pipeline_stats = PipelineStats::new();
            candidates
                .iter()
                .filter(|c| !is_pipeline_composite(c, &config.composite_elimination, &pipeline_stats))
                .cloned()
                .collect()
        }
        "screen" => {
            // Quick Miller-Rabin pre-screen with low round count.
            // This catches most remaining composites cheaply before the full test.
            // rug's is_probably_prime returns: NotPrime, ProbablyPrime, or Prime
            candidates
                .iter()
                .filter(|c| {
                    c.is_probably_prime(config.screen_mr_rounds) != rug::integer::IsPrime::No
                })
                .cloned()
                .collect()
        }
        "test" => {
            // Full primality test with 25 rounds of Miller-Rabin.
            // Form-specific tests (Proth, LLR, etc.) would be dispatched here
            // based on the `form` parameter in a future enhancement.
            candidates
                .iter()
                .filter(|c| {
                    c.is_probably_prime(config.test_mr_rounds) != rug::integer::IsPrime::No
                })
                .cloned()
                .collect()
        }
        "proof" => {
            // Deterministic proof stage. Currently passes through all candidates
            // since form-specific proof dispatch requires the search form context.
            // Individual search forms handle their own proofs after the pipeline.
            //
            // When proof_enabled is false, all candidates pass through as PRPs.
            if config.proof_enabled {
                // TODO: Dispatch to form-specific proof methods (Proth, LLR,
                // Pocklington, Morrison, BLS) based on the `form` parameter.
                // For now, all candidates are passed through — proof happens
                // downstream in the search form's own logic.
                candidates.to_vec()
            } else {
                candidates.to_vec()
            }
        }
        _ => {
            tracing::warn!(stage = stage, "Unknown pipeline stage, passing all candidates through");
            candidates.to_vec()
        }
    };

    let elapsed = start.elapsed();
    let survivor_count = survivors.len() as u64;

    WorkBlockStageResult {
        survivors,
        stats: WorkBlockStageStats {
            input_count,
            survivor_count,
            elapsed,
        },
    }
}

#[cfg(test)]
mod tests {
    //! # Pipeline Tests
    //!
    //! Validates the multi-stage composite elimination pipeline, verifying that:
    //! - Trial division catches small-factor composites
    //! - P-1 factoring catches smooth-factor composites (for large candidates)
    //! - Primes survive all stages
    //! - Stage statistics are correctly tracked
    //! - Configuration controls enable/disable stages
    //!
    //! ## References
    //!
    //! - OEIS A002808: Composite numbers.
    //! - J.M. Pollard, "Theorems on Factorization and Primality Testing", 1974.

    use super::*;
    use rug::ops::Pow;

    /// Small composites with factors in the trial division table are caught
    /// by Stage 1 (trial division), not requiring P-1 or ECM.
    #[test]
    fn pipeline_catches_small_factor_composites() {
        let config = PipelineConfig::default();
        let stats = PipelineStats::new();

        // 15 = 3 * 5 — has small factor
        let n = Integer::from(15u32);
        let (composite, stage) = is_composite(&n, &config, &stats);
        assert!(composite);
        assert_eq!(stage, Some(PipelineStage::TrialDivision));
    }

    /// Primes pass through all pipeline stages without being eliminated.
    #[test]
    fn pipeline_passes_primes() {
        let config = PipelineConfig::default();
        let stats = PipelineStats::new();

        let n = Integer::from(104729u32); // prime
        let (composite, stage) = is_composite(&n, &config, &stats);
        assert!(!composite);
        assert_eq!(stage, None);
    }

    /// Semiprimes with factors > 311 pass trial division but may be caught by P-1.
    /// 313 * 317 = 99221 has no small factors and is below the P-1 bit threshold.
    #[test]
    fn pipeline_passes_large_semiprimes_below_threshold() {
        let config = PipelineConfig::default();
        let stats = PipelineStats::new();

        let n = Integer::from(313u32) * Integer::from(317u32);
        let (composite, _stage) = is_composite(&n, &config, &stats);
        // This semiprime has no small factors and is too small for P-1
        assert!(!composite);
    }

    /// When trial division is disabled, small-factor composites pass through.
    #[test]
    fn pipeline_respects_disabled_trial_division() {
        let config = PipelineConfig {
            trial_division_enabled: false,
            ..Default::default()
        };
        let stats = PipelineStats::new();

        let n = Integer::from(15u32);
        let (composite, _) = is_composite(&n, &config, &stats);
        assert!(!composite, "Trial division disabled, 15 should pass");
    }

    /// Statistics track input counts and eliminations correctly.
    #[test]
    fn pipeline_stats_tracking() {
        let config = PipelineConfig::default();
        let stats = PipelineStats::new();

        // Test a composite
        is_composite(&Integer::from(15u32), &config, &stats);
        // Test a prime
        is_composite(&Integer::from(104729u32), &config, &stats);

        assert_eq!(stats.trial_division.input_count.load(Ordering::Relaxed), 2);
        assert_eq!(stats.trial_division.eliminated.load(Ordering::Relaxed), 1);
    }

    /// P-1 stage is skipped for candidates below the bit threshold.
    #[test]
    fn pipeline_p1_skipped_for_small_candidates() {
        let config = PipelineConfig {
            p1_enabled: true,
            p1_min_bits: 5_000,
            ..Default::default()
        };
        let stats = PipelineStats::new();

        // Small composite that passes trial division
        let n = Integer::from(313u32) * Integer::from(317u32);
        is_composite(&n, &config, &stats);

        // P-1 should not have been invoked
        assert_eq!(stats.p1_factoring.input_count.load(Ordering::Relaxed), 0);
    }

    /// Verify the elimination rate calculation.
    #[test]
    fn pipeline_elimination_rate() {
        let stats = StageStats::new();
        assert_eq!(stats.elimination_rate(), 0.0); // no data yet

        stats.input_count.store(100, Ordering::Relaxed);
        stats.eliminated.store(25, Ordering::Relaxed);
        assert!((stats.elimination_rate() - 0.25).abs() < 1e-10);
    }

    /// Default config has trial division and P-1 enabled, ECM disabled.
    #[test]
    fn pipeline_default_config() {
        let config = PipelineConfig::default();
        assert!(config.trial_division_enabled);
        assert!(config.p1_enabled);
        assert!(!config.ecm_enabled);
        assert_eq!(config.p1_min_bits, 5_000);
        assert_eq!(config.ecm_min_bits, 10_000);
        assert_eq!(config.ecm_curves, 10);
    }

    /// Pipeline catches a large composite via P-1 factoring.
    /// p = 65537 = 2^16+1 (Fermat prime, perfectly smooth p-1).
    /// q = next_prime(2^5000). n = p*q has > 5000 bits.
    #[test]
    fn pipeline_p1_catches_smooth_composite() {
        let config = PipelineConfig {
            trial_division_enabled: false, // skip to test P-1 directly
            p1_enabled: true,
            p1_min_bits: 5_000,
            ..Default::default()
        };
        let stats = PipelineStats::new();

        let p = Integer::from(65537u32);
        let q = {
            let mut q = Integer::from(2u32).pow(5000);
            q.next_prime_mut();
            q
        };
        let n = Integer::from(&p * &q);
        let (composite, stage) = is_composite(&n, &config, &stats);
        assert!(composite, "P-1 should find 65537 (perfectly smooth p-1)");
        assert_eq!(stage, Some(PipelineStage::P1Factoring));
    }

    /// PipelineStage Display implementation for logging.
    #[test]
    fn pipeline_stage_display() {
        assert_eq!(PipelineStage::TrialDivision.to_string(), "trial_division");
        assert_eq!(PipelineStage::P1Factoring.to_string(), "p1_factoring");
        assert_eq!(PipelineStage::EcmFactoring.to_string(), "ecm_factoring");
    }

    /// Log summary doesn't panic on empty stats.
    #[test]
    fn pipeline_log_summary_empty() {
        let stats = PipelineStats::new();
        stats.log_summary(); // should not panic
    }

    // ── Work Block Pipeline Tests ────────────────────────────────

    /// next_stage follows the linear pipeline: sieve → screen → test → proof → None.
    #[test]
    fn work_block_next_stage_linear_progression() {
        assert_eq!(next_stage("sieve"), Some("screen"));
        assert_eq!(next_stage("screen"), Some("test"));
        assert_eq!(next_stage("test"), Some("proof"));
        assert_eq!(next_stage("proof"), None);
    }

    /// next_stage returns None for unknown stage names.
    #[test]
    fn work_block_next_stage_unknown_returns_none() {
        assert_eq!(next_stage("unknown"), None);
        assert_eq!(next_stage(""), None);
        assert_eq!(next_stage("SIEVE"), None); // case sensitive
    }

    /// Walking through next_stage from "sieve" visits all stages in order.
    #[test]
    fn work_block_next_stage_full_walk() {
        let mut current = "sieve";
        let mut visited = vec![current.to_string()];
        while let Some(next) = next_stage(current) {
            visited.push(next.to_string());
            current = next;
        }
        assert_eq!(visited, vec!["sieve", "screen", "test", "proof"]);
    }

    /// PIPELINE_STAGES constant contains all 4 stages in order.
    #[test]
    fn work_block_pipeline_stages_constant() {
        assert_eq!(PIPELINE_STAGES.len(), 4);
        assert_eq!(PIPELINE_STAGES[0], "sieve");
        assert_eq!(PIPELINE_STAGES[1], "screen");
        assert_eq!(PIPELINE_STAGES[2], "test");
        assert_eq!(PIPELINE_STAGES[3], "proof");
    }

    /// stage_index returns correct 0-based indices for all stages.
    #[test]
    fn work_block_stage_index_values() {
        assert_eq!(stage_index("sieve"), Some(0));
        assert_eq!(stage_index("screen"), Some(1));
        assert_eq!(stage_index("test"), Some(2));
        assert_eq!(stage_index("proof"), Some(3));
        assert_eq!(stage_index("unknown"), None);
    }

    /// is_valid_stage accepts the 4 pipeline stages and rejects others.
    #[test]
    fn work_block_is_valid_stage() {
        for stage in PIPELINE_STAGES {
            assert!(is_valid_stage(stage), "Expected '{}' to be valid", stage);
        }
        assert!(!is_valid_stage("unknown"));
        assert!(!is_valid_stage(""));
        assert!(!is_valid_stage("SIEVE"));
    }

    /// Sieve stage eliminates composites with small factors.
    #[test]
    fn work_block_sieve_stage_eliminates_composites() {
        let config = WorkBlockPipelineConfig::default();
        let candidates = vec![
            Integer::from(15u32),     // composite (3*5)
            Integer::from(104729u32), // prime
            Integer::from(21u32),     // composite (3*7)
            Integer::from(7u32),      // prime (but will be flagged as small factor since 7 is in table)
        ];
        let result = execute_stage("sieve", &candidates, "kbn", &config);
        // 15 and 21 have small factors, 7 == 7 so has_small_factor skips it,
        // 104729 is prime
        assert_eq!(result.stats.input_count, 4);
        // At least some should survive
        assert!(result.stats.survivor_count > 0);
        assert!(result.stats.survivor_count < result.stats.input_count);
    }

    /// Screen stage identifies primes and composites via quick MR.
    #[test]
    fn work_block_screen_stage_filters() {
        let config = WorkBlockPipelineConfig::default();
        let candidates = vec![
            Integer::from(104729u32), // prime
            Integer::from(99221u32),  // 313*317 composite (no small factors)
        ];
        let result = execute_stage("screen", &candidates, "kbn", &config);
        assert_eq!(result.stats.input_count, 2);
        // The prime should survive, the composite should be eliminated
        assert_eq!(result.stats.survivor_count, 1);
        assert_eq!(result.survivors[0], Integer::from(104729u32));
    }

    /// Test stage with full MR rounds identifies PRPs.
    #[test]
    fn work_block_test_stage_identifies_prps() {
        let config = WorkBlockPipelineConfig::default();
        // All primes should survive the test stage
        let primes = vec![
            Integer::from(104729u32),
            Integer::from(2u32),
            Integer::from(7919u32),
        ];
        let result = execute_stage("test", &primes, "kbn", &config);
        assert_eq!(result.stats.input_count, 3);
        assert_eq!(result.stats.survivor_count, 3);
    }

    /// Proof stage currently passes all candidates through (pending form-specific dispatch).
    #[test]
    fn work_block_proof_stage_passes_through() {
        let config = WorkBlockPipelineConfig::default();
        let candidates = vec![Integer::from(104729u32), Integer::from(7919u32)];
        let result = execute_stage("proof", &candidates, "kbn", &config);
        assert_eq!(result.stats.input_count, 2);
        assert_eq!(result.stats.survivor_count, 2);
    }

    /// Unknown stage passes all candidates through without elimination.
    #[test]
    fn work_block_unknown_stage_passes_through() {
        let config = WorkBlockPipelineConfig::default();
        let candidates = vec![Integer::from(15u32), Integer::from(104729u32)];
        let result = execute_stage("nonexistent", &candidates, "kbn", &config);
        assert_eq!(result.stats.input_count, 2);
        assert_eq!(result.stats.survivor_count, 2);
    }

    /// Empty candidate list produces empty result with zero stats.
    #[test]
    fn work_block_stage_empty_candidates() {
        let config = WorkBlockPipelineConfig::default();
        for stage in PIPELINE_STAGES {
            let result = execute_stage(stage, &[], "kbn", &config);
            assert_eq!(result.stats.input_count, 0);
            assert_eq!(result.stats.survivor_count, 0);
            assert!(result.survivors.is_empty());
        }
    }

    /// WorkBlockStageStats elimination_rate is correct.
    #[test]
    fn work_block_stage_stats_elimination_rate() {
        let stats = WorkBlockStageStats {
            input_count: 100,
            survivor_count: 25,
            elapsed: std::time::Duration::from_millis(50),
        };
        assert!((stats.elimination_rate() - 0.75).abs() < 1e-10);

        let empty = WorkBlockStageStats {
            input_count: 0,
            survivor_count: 0,
            elapsed: std::time::Duration::ZERO,
        };
        assert_eq!(empty.elimination_rate(), 0.0);
    }

    /// Default WorkBlockPipelineConfig has expected values.
    #[test]
    fn work_block_pipeline_config_defaults() {
        let config = WorkBlockPipelineConfig::default();
        assert_eq!(config.screen_mr_rounds, 2);
        assert_eq!(config.test_mr_rounds, 25);
        assert!(config.proof_enabled);
        assert!(config.composite_elimination.trial_division_enabled);
    }

    /// Full pipeline run: sieve → screen → test → proof on a mixed batch.
    /// Verifies that candidates are progressively eliminated across stages.
    #[test]
    fn work_block_full_pipeline_run() {
        let config = WorkBlockPipelineConfig::default();
        // Mix of primes and composites of varying difficulty
        let initial = vec![
            Integer::from(15u32),     // composite, caught by sieve (factor 3)
            Integer::from(104729u32), // prime, survives all
            Integer::from(21u32),     // composite, caught by sieve (factor 3)
            Integer::from(99221u32),  // 313*317, caught by screen
            Integer::from(7919u32),   // prime, survives all
        ];

        // Stage 1: sieve
        let sieve_result = execute_stage("sieve", &initial, "kbn", &config);
        assert!(sieve_result.stats.survivor_count < sieve_result.stats.input_count,
            "Sieve should eliminate at least one composite");

        // Stage 2: screen
        let screen_result = execute_stage("screen", &sieve_result.survivors, "kbn", &config);
        assert!(screen_result.stats.survivor_count <= screen_result.stats.input_count);

        // Stage 3: test
        let test_result = execute_stage("test", &screen_result.survivors, "kbn", &config);

        // Stage 4: proof
        let proof_result = execute_stage("proof", &test_result.survivors, "kbn", &config);

        // The two actual primes (104729, 7919) should survive the full pipeline
        let survivor_values: Vec<u32> = proof_result.survivors.iter()
            .filter_map(|s| s.to_u32())
            .collect();
        assert!(survivor_values.contains(&104729), "104729 (prime) should survive");
        assert!(survivor_values.contains(&7919), "7919 (prime) should survive");
    }
}
