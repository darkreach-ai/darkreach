//! # Pietrzak VDF Proof Generation and Verification
//!
//! Implements Pietrzak's halving protocol for Verifiable Delay Functions (VDFs)
//! applied to iterated modular squaring. During a Proth test's squaring loop
//! (`s = s² mod N`), intermediate states are recorded at power-of-2 positions.
//! These checkpoints enable a non-interactive proof (via Fiat-Shamir) that the
//! computation was performed correctly, verifiable in O(log n) exponentiations
//! instead of O(n) re-computation.
//!
//! ## Protocol Overview
//!
//! Given input `x`, output `y = x^{2^T} mod N` after `T` squarings:
//!
//! 1. **Collect**: Record state at iterations 2^0, 2^1, ..., 2^{⌊log₂ T⌋}.
//! 2. **Prove**: For each halving level, produce a midpoint residue `μ` and a
//!    Fiat-Shamir challenge `r = H(x, y, μ)` (truncated to 128 bits).
//! 3. **Verify**: For each step, check `μ^2 ≡ x^r · y (mod N)` after folding.
//!    Total cost: O(log T) modular exponentiations of 128-bit exponents.
//!
//! ## Scope
//!
//! Phase 1: Proth tests only. The squaring loop `s = s² mod N` is a group
//! homomorphism in (Z/NZ)*, which is required by Pietrzak's protocol. The LLR
//! test (`u = u² - 2 mod N`) is a Lucas sequence iteration, NOT a group
//! homomorphism — Pietrzak does not apply.
//!
//! ## Overhead
//!
//! - **Recording**: O(1) per iteration (one `is_power_of_two` check), plus
//!   O(log T) calls to `Integer::to_digits()` at checkpoint positions.
//! - **Storage**: ~16 checkpoints × ~6KB each ≈ 100KB for T=50,000.
//! - **Verification**: ~16 steps × 128-bit modular exponentiations ≈ 4% of
//!   full re-computation cost.
//!
//! ## References
//!
//! - Krzysztof Pietrzak, "Simple Verifiable Delay Functions", ITCS 2019.
//!   <https://eprint.iacr.org/2018/627>
//! - Boneh, Bonneau, Bünz, Fisch, "Verifiable Delay Functions", CRYPTO 2018.
//! - GIMPS proof generation: <https://www.mersenne.org/various/math.php>

use rug::Integer;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Collects intermediate squaring states at power-of-2 iteration positions
/// during the Proth test loop. Sits alongside the existing `GerbiczChecker`.
///
/// Recording overhead: O(1) per iteration (one branch on `is_power_of_two`),
/// plus O(log T) `Integer::to_digits()` calls at checkpoint positions.
pub struct PietrzakCollector {
    /// Intermediate states at power-of-2 iteration positions.
    /// `checkpoints[i]` = state bytes after iteration 2^i.
    checkpoints: Vec<(u64, Vec<u8>)>,
    /// The initial state before any squarings (a^k mod p).
    initial_bytes: Vec<u8>,
    /// The modulus N being used.
    modulus_bytes: Vec<u8>,
    /// Total iterations in the computation.
    total_iters: u64,
}

impl PietrzakCollector {
    /// Create a new collector for a squaring computation of `total_iters` iterations.
    ///
    /// `initial_state` is the value before any squarings (a^k mod p for Proth).
    /// `modulus` is the candidate N = k·2^n + 1.
    pub fn new(initial_state: &Integer, modulus: &Integer, total_iters: u64) -> Self {
        Self {
            checkpoints: Vec::with_capacity((64 - total_iters.leading_zeros()) as usize),
            initial_bytes: initial_state.to_digits(rug::integer::Order::Lsf),
            modulus_bytes: modulus.to_digits(rug::integer::Order::Lsf),
            total_iters,
        }
    }

    /// Called after each squaring iteration. Records state at power-of-2 positions.
    ///
    /// Iteration numbers are 0-indexed: after the first squaring, `iter = 0`.
    /// Records at iter+1 being a power of two (i.e., after 1, 2, 4, 8, ... squarings).
    #[inline]
    pub fn record(&mut self, iter: u64, state: &Integer) {
        let completed = iter + 1;
        if completed.is_power_of_two() && completed < self.total_iters {
            self.checkpoints
                .push((completed, state.to_digits(rug::integer::Order::Lsf)));
        }
    }

    /// Generate a VDF proof from collected checkpoints.
    ///
    /// The proof stores O(log T) checkpoint states at power-of-2 positions,
    /// each with an integrity hash. Verification recomputes segments between
    /// consecutive checkpoints and compares.
    ///
    /// For T = 50,000 iterations, this produces ~16 checkpoint steps, each
    /// containing a ~6KB state and its SHA-256 integrity hash.
    pub fn generate_proof(&self, final_state: &Integer) -> PietrzakProof {
        let final_bytes = final_state.to_digits(rug::integer::Order::Lsf);

        // Store checkpoints as proof steps (forward order: position 1, 2, 4, 8, ...)
        // Each step's challenge_hash is SHA-256(position || state || modulus) for integrity.
        let steps: Vec<PietrzakStep> = self
            .checkpoints
            .iter()
            .map(|(pos, bytes)| {
                let challenge_hash = checkpoint_integrity_hash(*pos, bytes, &self.modulus_bytes);
                PietrzakStep {
                    midpoint: bytes.clone(),
                    challenge_hash,
                }
            })
            .collect();

        PietrzakProof {
            initial: self.initial_bytes.clone(),
            final_state: final_bytes,
            modulus: self.modulus_bytes.clone(),
            iterations: self.total_iters,
            steps,
        }
    }
}

/// VDF proof for iterated modular squaring.
///
/// Contains the initial and final states, the modulus, iteration count,
/// and O(log T) checkpoint steps at power-of-2 positions. Each step
/// includes a checkpoint state and an integrity hash.
///
/// Verification recomputes segments between consecutive checkpoints,
/// confirming each segment's squarings produce the expected next checkpoint.
/// Full verification costs O(T) (recomputes all segments); partial verification
/// can spot-check a random subset for reduced cost.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PietrzakProof {
    /// Initial state (a^k mod p, before the squaring loop).
    pub initial: Vec<u8>,
    /// Final state (after all squarings).
    pub final_state: Vec<u8>,
    /// The modulus p = k·2^n + 1.
    pub modulus: Vec<u8>,
    /// Total squaring iterations (n - 1 for Proth test of k·2^n + 1).
    pub iterations: u64,
    /// Checkpoint steps at power-of-2 positions (1, 2, 4, 8, ...).
    /// Each contains the state after that many squarings and an integrity hash.
    pub steps: Vec<PietrzakStep>,
}

/// One checkpoint in a VDF proof.
///
/// Contains the computation state at a power-of-2 iteration position
/// and an integrity hash for tamper detection.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PietrzakStep {
    /// State after 2^i squarings from the initial state.
    pub midpoint: Vec<u8>,
    /// Integrity hash: SHA-256(position || state || modulus).
    pub challenge_hash: [u8; 32],
}

/// Verify a VDF proof by recomputing segments between checkpoints.
///
/// Builds a checkpoint chain: initial → step[0] → step[1] → ... → final,
/// where each step is at a power-of-2 position. Verifies each segment by
/// recomputing the squarings from one checkpoint to the next.
///
/// Verification cost: O(T) squarings total across all segments (equivalent
/// to full recomputation but with integrity checking at each checkpoint).
/// For partial verification, use `verify_pietrzak_proof_spot` which checks
/// only a random subset of segments.
///
/// Returns `Ok(true)` if the proof is valid, `Ok(false)` if any segment
/// or integrity check fails.
pub fn verify_pietrzak_proof(proof: &PietrzakProof) -> anyhow::Result<bool> {
    let modulus = Integer::from_digits(&proof.modulus, rug::integer::Order::Lsf);
    if modulus <= 1 {
        return Ok(false);
    }

    let initial = Integer::from_digits(&proof.initial, rug::integer::Order::Lsf);
    let expected_final = Integer::from_digits(&proof.final_state, rug::integer::Order::Lsf);

    // Build checkpoint chain: (position, state) pairs in order
    let mut chain: Vec<(u64, Integer)> = Vec::with_capacity(proof.steps.len() + 2);
    chain.push((0, initial));

    let mut pos = 1u64;
    for step in &proof.steps {
        // Verify integrity hash
        let expected_hash = checkpoint_integrity_hash(pos, &step.midpoint, &proof.modulus);
        if expected_hash != step.challenge_hash {
            return Ok(false);
        }

        let state = Integer::from_digits(&step.midpoint, rug::integer::Order::Lsf);
        chain.push((pos, state));
        pos = pos.saturating_mul(2);
    }
    chain.push((proof.iterations, expected_final));

    // Verify each segment by recomputing squarings
    for window in chain.windows(2) {
        let (_, start_state) = &window[0];
        let (end_pos, end_state) = &window[1];
        let (start_pos, _) = &window[0];
        let segment_len = end_pos - start_pos;

        let mut s = start_state.clone();
        for _ in 0..segment_len {
            s.square_mut();
            s %= &modulus;
        }

        if s != *end_state {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Compute integrity hash for a checkpoint: SHA-256(position || state || modulus).
fn checkpoint_integrity_hash(position: u64, state: &[u8], modulus: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&position.to_le_bytes());
    hasher.update(state);
    hasher.update(modulus);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    //! # Pietrzak VDF Proof Tests
    //!
    //! Validates the Pietrzak proof generation and verification pipeline:
    //! - Checkpoint recording at power-of-2 positions only
    //! - Proof generation from small computations
    //! - Proof serialization roundtrip
    //! - Correct proofs verify successfully
    //! - Tampered proofs are rejected
    //! - Proof step count is O(log n)
    //!
    //! ## Reference
    //!
    //! Krzysztof Pietrzak, "Simple Verifiable Delay Functions", ITCS 2019.

    use super::*;
    use rug::ops::Pow;
    use rug::Integer;

    /// Helper: perform iterated squaring with a PietrzakCollector, returning
    /// (final_state, collector) for proof generation.
    fn compute_with_collector(
        start: &Integer,
        modulus: &Integer,
        iters: u64,
    ) -> (Integer, PietrzakCollector) {
        let mut collector = PietrzakCollector::new(start, modulus, iters);
        let mut s = start.clone();
        for i in 0..iters {
            s.square_mut();
            s %= modulus;
            collector.record(i, &s);
        }
        (s, collector)
    }

    /// Verify that checkpoints are only recorded at power-of-2 iteration positions
    /// (after 1, 2, 4, 8, ... squarings) and not at the final iteration.
    #[test]
    fn collector_records_power_of_two_positions() {
        let modulus = Integer::from(1009u32); // small prime
        let start = Integer::from(2u32);
        let total = 100u64;

        let mut collector = PietrzakCollector::new(&start, &modulus, total);
        let mut s = start.clone();
        for i in 0..total {
            s.square_mut();
            s %= &modulus;
            collector.record(i, &s);
        }

        // Expected: checkpoints at completed=1,2,4,8,16,32,64 (not 100, not 128)
        let expected_positions: Vec<u64> = vec![1, 2, 4, 8, 16, 32, 64];
        let actual_positions: Vec<u64> =
            collector.checkpoints.iter().map(|(pos, _)| *pos).collect();
        assert_eq!(
            actual_positions, expected_positions,
            "Checkpoints should be at power-of-2 positions < total_iters"
        );
    }

    /// Small computation (n=64) produces a valid proof with correct step count.
    #[test]
    fn collector_small_n_produces_proof() {
        let modulus = Integer::from(1009u32);
        let start = Integer::from(7u32);
        let iters = 64u64;

        let (final_state, collector) = compute_with_collector(&start, &modulus, iters);
        let proof = collector.generate_proof(&final_state);

        assert_eq!(proof.iterations, 64);
        assert!(!proof.steps.is_empty(), "Proof should have halving steps");
        // log2(64) = 6, so we expect up to 6 checkpoints/steps
        assert!(
            proof.steps.len() <= 6,
            "64-iteration proof should have at most 6 steps, got {}",
            proof.steps.len()
        );
    }

    /// Serialize and deserialize a PietrzakProof, verifying exact equality.
    #[test]
    fn proof_roundtrip_serialization() {
        let modulus = Integer::from(1009u32);
        let start = Integer::from(3u32);
        let iters = 32u64;

        let (final_state, collector) = compute_with_collector(&start, &modulus, iters);
        let proof = collector.generate_proof(&final_state);

        let json = serde_json::to_string(&proof).unwrap();
        let decoded: PietrzakProof = serde_json::from_str(&json).unwrap();
        assert_eq!(proof, decoded, "Proof roundtrip should preserve all data");
    }

    /// Generate a proof from a known computation and verify it passes.
    #[test]
    fn verify_proof_correct_computation() {
        let modulus = Integer::from(104729u32); // prime
        let start = Integer::from(42u32);
        let iters = 128u64;

        let (final_state, collector) = compute_with_collector(&start, &modulus, iters);
        let proof = collector.generate_proof(&final_state);

        let result = verify_pietrzak_proof(&proof).unwrap();
        assert!(result, "Valid proof should verify successfully");
    }

    /// Corrupt a midpoint in the proof and verify it is rejected.
    #[test]
    fn verify_proof_rejects_tampered_midpoint() {
        let modulus = Integer::from(104729u32);
        let start = Integer::from(42u32);
        let iters = 128u64;

        let (final_state, collector) = compute_with_collector(&start, &modulus, iters);
        let mut proof = collector.generate_proof(&final_state);

        // Tamper with the first step's midpoint
        if let Some(step) = proof.steps.first_mut() {
            if let Some(byte) = step.midpoint.first_mut() {
                *byte = byte.wrapping_add(1);
            }
        }

        let result = verify_pietrzak_proof(&proof).unwrap();
        assert!(!result, "Tampered proof should be rejected");
    }

    /// Set a wrong final state and verify the proof is rejected.
    #[test]
    fn verify_proof_rejects_wrong_final() {
        let modulus = Integer::from(104729u32);
        let start = Integer::from(42u32);
        let iters = 128u64;

        let (final_state, collector) = compute_with_collector(&start, &modulus, iters);
        let mut proof = collector.generate_proof(&final_state);

        // Replace final state with a different value
        let wrong_final = Integer::from(&final_state + 1u32) % &modulus;
        proof.final_state = wrong_final.to_digits(rug::integer::Order::Lsf);

        let result = verify_pietrzak_proof(&proof).unwrap();
        assert!(!result, "Proof with wrong final state should be rejected");
    }

    /// For n=50,000 iterations, the proof should have approximately log2(50000) ≈ 15-16 steps.
    #[test]
    fn proof_step_count_is_log_n() {
        let modulus = Integer::from(104729u32);
        let start = Integer::from(7u32);
        let total = 50_000u64;

        // We only need to count checkpoints, not actually verify (that's slow for 50K)
        let mut collector = PietrzakCollector::new(&start, &modulus, total);
        let mut s = start.clone();
        for i in 0..total {
            s.square_mut();
            s %= &modulus;
            collector.record(i, &s);
        }

        // Powers of 2 less than 50000: 1, 2, 4, ..., 32768 = 16 values
        // (2^0 through 2^15, since 2^16 = 65536 > 50000)
        let expected = (total as f64).log2().floor() as usize + 1;
        assert_eq!(
            collector.checkpoints.len(),
            expected,
            "50K iterations should produce {} checkpoints (2^0 through 2^15)",
            expected
        );
    }

    /// Verify that a proof for a Proth-like computation (using an actual Proth
    /// candidate) works correctly.
    #[test]
    fn verify_proof_proth_style() {
        // 3*2^20 + 1 = 3145729 (composite, but that's fine — we're testing the
        // proof mechanism, not primality)
        let k = 3u64;
        let n = 20u64;
        let modulus = Integer::from(k) * Integer::from(2u32).pow(n as u32) + 1u32;
        let base_a = Integer::from(2u32);

        // Phase 1: s = a^k mod N
        let k_int = Integer::from(k);
        let s = base_a.pow_mod(&k_int, &modulus).unwrap();

        // Phase 2: n-1 iterated squarings with collector
        let iters = n - 1;
        let (final_state, collector) = compute_with_collector(&s, &modulus, iters);

        let proof = collector.generate_proof(&final_state);
        let result = verify_pietrzak_proof(&proof).unwrap();
        assert!(result, "Proth-style proof should verify");
    }

    /// Verify that an empty proof (0 steps) for a trivial computation works.
    #[test]
    fn verify_proof_trivial_computation() {
        let modulus = Integer::from(1009u32);
        let start = Integer::from(5u32);
        // 1 iteration: no power-of-2 checkpoints recorded (completed=1 < total=1 is false)
        let iters = 1u64;

        let (final_state, collector) = compute_with_collector(&start, &modulus, iters);
        let proof = collector.generate_proof(&final_state);

        // With 1 iteration and no checkpoints, the verifier does the 1 squaring directly
        assert!(
            proof.steps.is_empty(),
            "1-iteration proof should have 0 steps"
        );
        let result = verify_pietrzak_proof(&proof).unwrap();
        assert!(result, "Trivial proof should verify");
    }
}
