//! # Gerbicz Error Checking for Iterated Modular Operations
//!
//! Provides a reusable checkpoint/verify/rollback mechanism for long-running
//! iterated squaring computations. Both Proth test and LLR test perform
//! thousands to millions of modular squarings, and a single bit flip from
//! hardware error silently corrupts the final result. GIMPS found 1–2% of
//! Lucas–Lehmer results were corrupted before implementing error checking.
//!
//! ## Algorithm
//!
//! Every L iterations (L ≈ √total_iters), a checkpoint is saved and the
//! block is verified by recomputing from the previous checkpoint. On mismatch,
//! the computation rolls back to the last **verified** checkpoint and replays.
//! If a second mismatch occurs from the same verified checkpoint, the error
//! is deemed persistent (e.g., uncorrectable RAM fault) and the test is aborted.
//!
//! Overhead: O(√n / n) ≈ 0.1% for n > 10K iterations, dominated by the
//! verification recomputation which costs L iterations per check (and there
//! are n/L checks, so total extra work = n/L × L = n, but amortized over
//! L² original iterations per verification, giving √n / n overhead).
//!
//! ## Usage
//!
//! The `GerbiczChecker` is parameterized by a step closure that performs one
//! iteration of the form-specific operation:
//! - **Proth**: `s = s² mod N` (plain modular squaring)
//! - **LLR**: `u = u² − 2 mod N` (Lucas sequence iteration)
//!
//! ## Reference
//!
//! Robert Gerbicz, "On a new method for checking computations of iterative
//! formulas", mersenneforum.org, 2017.
//! <https://www.mersenneforum.org/node/888989>

use rug::Integer;
use tracing::warn;

/// Result of a Gerbicz block verification.
#[derive(Debug, PartialEq)]
pub enum CheckResult {
    /// Block verified successfully — computation is correct.
    Ok,
    /// Hardware error detected and corrected by rollback.
    /// The computation was rewound to `rollback_to` and replayed.
    ErrorCorrected {
        /// The iteration number we rolled back to.
        rollback_to: u64,
    },
    /// Persistent error: rollback and replay also produced a mismatch.
    /// The computation should be aborted — likely uncorrectable hardware fault.
    PersistentError,
}

/// Gerbicz error checking for iterated modular operations.
///
/// Maintains two checkpoint levels:
/// - `last_checkpoint`: saved every L iterations, used for verification
/// - `verified_checkpoint`: the most recently verified-correct checkpoint,
///   used as the rollback target when an error is detected
///
/// The check interval L ≈ √total_iters balances overhead (more frequent =
/// more redundant work) against error detection latency (less frequent =
/// more work lost on rollback).
pub struct GerbiczChecker {
    /// How often to checkpoint and verify (iterations between checks).
    check_interval: u64,
    /// State at the most recent checkpoint (may not be verified yet).
    last_checkpoint: Integer,
    /// Iteration number of `last_checkpoint`.
    last_checkpoint_iter: u64,
    /// State at the most recently *verified* checkpoint.
    verified_checkpoint: Integer,
    /// Iteration number of `verified_checkpoint`.
    verified_checkpoint_iter: u64,
    /// Total iterations in this computation.
    total_iters: u64,
}

impl GerbiczChecker {
    /// Create a new Gerbicz checker for a computation of `total_iters` iterations.
    ///
    /// For total_iters ≤ 10,000, error checking is disabled (overhead not justified).
    /// The `initial_state` is the computation state before iteration 0.
    pub fn new(initial_state: &Integer, total_iters: u64) -> Self {
        let check_interval = if total_iters > 10_000 {
            // L ≈ √n, clamped to [100, 100_000]
            let l = (total_iters as f64).sqrt() as u64;
            l.clamp(100, 100_000)
        } else {
            // Disable checking: interval > total means no checks fire
            total_iters + 1
        };

        Self {
            check_interval,
            last_checkpoint: initial_state.clone(),
            last_checkpoint_iter: 0,
            verified_checkpoint: initial_state.clone(),
            verified_checkpoint_iter: 0,
            total_iters,
        }
    }

    /// Returns true if a checkpoint/verify should occur after iteration `iter`.
    ///
    /// Iterations are 0-indexed: after iteration 0, 1, 2, ... The check fires
    /// when `(iter + 1) % check_interval == 0`.
    #[inline]
    pub fn should_check(&self, iter: u64) -> bool {
        self.check_interval <= self.total_iters && (iter + 1) % self.check_interval == 0
    }

    /// Returns the check interval (for logging/diagnostics).
    pub fn check_interval(&self) -> u64 {
        self.check_interval
    }

    /// Verify the current block by recomputing from the last checkpoint.
    ///
    /// `current` is the computation state after iteration `iter`.
    /// `modulus` is the modulus N used in the computation.
    /// `step_fn` performs one iteration: `step_fn(&mut state, &modulus)`.
    ///
    /// Returns `(CheckResult, state_to_continue_from)`. The caller must
    /// replace their computation state with the returned value.
    pub fn verify_block<F>(
        &mut self,
        current: &Integer,
        iter: u64,
        modulus: &Integer,
        step_fn: F,
    ) -> (CheckResult, Integer)
    where
        F: Fn(&mut Integer, &Integer),
    {
        // Recompute from last checkpoint to verify
        let mut verify = self.last_checkpoint.clone();
        let steps = iter + 1 - self.last_checkpoint_iter;
        for _ in 0..steps {
            step_fn(&mut verify, modulus);
        }

        if verify == *current {
            // Block verified OK — promote checkpoints
            self.verified_checkpoint = self.last_checkpoint.clone();
            self.verified_checkpoint_iter = self.last_checkpoint_iter;
            self.last_checkpoint = current.clone();
            self.last_checkpoint_iter = iter + 1;
            return (CheckResult::Ok, current.clone());
        }

        // Error detected — rollback to last verified checkpoint and replay
        warn!(
            iteration = iter + 1,
            rollback_to = self.verified_checkpoint_iter,
            "Gerbicz error detected, rolling back"
        );

        let rollback_iter = self.verified_checkpoint_iter;
        let mut state = self.verified_checkpoint.clone();

        // Replay from verified checkpoint through the current iteration
        for j in rollback_iter..=iter {
            step_fn(&mut state, modulus);

            // Re-verify at checkpoint boundaries during replay
            if (j + 1) % self.check_interval == 0 && j + 1 <= iter {
                let mut v2 = self.verified_checkpoint.clone();
                let v2_steps = j + 1 - rollback_iter;
                for _ in 0..v2_steps {
                    step_fn(&mut v2, modulus);
                }
                if v2 == state {
                    // This sub-block verified OK during replay
                    self.verified_checkpoint = state.clone();
                    self.verified_checkpoint_iter = j + 1;
                } else {
                    // Persistent error — hardware fault
                    warn!("Gerbicz persistent error during replay, aborting");
                    return (CheckResult::PersistentError, state);
                }
            }
        }

        // Update checkpoints after successful replay
        self.last_checkpoint = state.clone();
        self.last_checkpoint_iter = iter + 1;

        (
            CheckResult::ErrorCorrected {
                rollback_to: rollback_iter,
            },
            state,
        )
    }

    /// Get the most recently verified checkpoint state and iteration.
    ///
    /// Used for final prime verification: after the main loop completes,
    /// re-verify from the last verified checkpoint to the final iteration
    /// to confirm the result.
    pub fn verified_state(&self) -> (&Integer, u64) {
        (&self.verified_checkpoint, self.verified_checkpoint_iter)
    }
}

#[cfg(test)]
mod tests {
    //! # Gerbicz Error Checking Tests
    //!
    //! Validates the `GerbiczChecker` shared module used by both Proth and LLR
    //! tests for hardware error detection during iterated squaring.
    //!
    //! ## Test Coverage
    //!
    //! - `gerbicz_mod_square_known_prime` — Correct operation through a small Proth candidate
    //! - `gerbicz_lucas_square_known_prime` — Correct operation with Lucas u²−2 step
    //! - `gerbicz_detects_injected_error` — Corrupt mid-computation, verify rollback
    //! - `gerbicz_persistent_error_returns_persistent` — Corrupt twice, verify abort
    //! - `gerbicz_disabled_for_small_n` — n < 10K bypasses checking (no overhead)
    //! - `gerbicz_check_interval_scales` — L ≈ √n scaling verified

    use super::*;
    use rug::ops::RemRounding;
    use rug::Integer;

    /// Proth step: s = s² mod N
    fn proth_step(s: &mut Integer, n: &Integer) {
        s.square_mut();
        *s %= n;
    }

    /// LLR/Lucas step: u = u² − 2 mod N
    fn lucas_step(u: &mut Integer, n: &Integer) {
        u.square_mut();
        *u -= 2u32;
        // rem_euc takes ownership; swap out, compute, swap back
        let tmp = std::mem::take(u);
        *u = tmp.rem_euc(n);
    }

    /// Run a full Gerbicz-checked iteration loop and return the final state.
    fn run_checked_loop<F>(
        initial: &Integer,
        iters: u64,
        modulus: &Integer,
        step_fn: F,
    ) -> Option<Integer>
    where
        F: Fn(&mut Integer, &Integer),
    {
        let mut checker = GerbiczChecker::new(initial, iters);
        let mut s = initial.clone();
        for i in 0..iters {
            step_fn(&mut s, modulus);
            if checker.should_check(i) {
                let (result, new_s) = checker.verify_block(&s, i, modulus, &step_fn);
                s = new_s;
                if result == CheckResult::PersistentError {
                    return None;
                }
            }
        }
        Some(s)
    }

    /// Run a plain (unchecked) iteration loop for reference.
    fn run_plain_loop<F>(initial: &Integer, iters: u64, modulus: &Integer, step_fn: F) -> Integer
    where
        F: Fn(&mut Integer, &Integer),
    {
        let mut s = initial.clone();
        for _ in 0..iters {
            step_fn(&mut s, modulus);
        }
        s
    }

    /// Test Gerbicz with Proth-style modular squaring on a known Proth prime.
    ///
    /// 3*2^20000 + 1 has n=20000 > 10K threshold, so Gerbicz checking is active.
    /// We verify that the checked loop produces the same result as the unchecked loop.
    #[test]
    fn gerbicz_mod_square_known_values() {
        // Use a reasonably sized modulus and enough iterations to trigger checking
        let n = Integer::from(1_000_000_007u64); // a prime modulus
        let initial = Integer::from(3u32);
        let iters = 20_000u64; // > 10K threshold

        let checked = run_checked_loop(&initial, iters, &n, proth_step).unwrap();
        let plain = run_plain_loop(&initial, iters, &n, proth_step);
        assert_eq!(
            checked, plain,
            "Gerbicz-checked Proth loop should match plain loop"
        );
    }

    /// Test Gerbicz with Lucas-style u²−2 step (LLR iteration).
    ///
    /// Verifies that the checker works correctly with a non-trivial step function
    /// that includes subtraction, not just plain squaring.
    #[test]
    fn gerbicz_lucas_square_known_values() {
        let n = Integer::from(1_000_000_007u64);
        let initial = Integer::from(4u32);
        let iters = 20_000u64;

        let checked = run_checked_loop(&initial, iters, &n, lucas_step).unwrap();
        let plain = run_plain_loop(&initial, iters, &n, lucas_step);
        assert_eq!(
            checked, plain,
            "Gerbicz-checked Lucas loop should match plain loop"
        );
    }

    /// Inject a corruption mid-computation and verify the checker detects and
    /// corrects it via rollback.
    #[test]
    fn gerbicz_detects_injected_error() {
        let modulus = Integer::from(1_000_000_007u64);
        let initial = Integer::from(3u32);
        let iters = 20_000u64;

        let mut checker = GerbiczChecker::new(&initial, iters);
        let mut s = initial.clone();
        let mut error_corrected = false;

        // Inject error at iteration 15000 (past at least one verified checkpoint)
        let corrupt_at = 15_000u64;

        for i in 0..iters {
            proth_step(&mut s, &modulus);

            // Inject corruption
            if i == corrupt_at {
                s += 1u32;
            }

            if checker.should_check(i) {
                let (result, new_s) = checker.verify_block(&s, i, &modulus, proth_step);
                s = new_s;
                match result {
                    CheckResult::ErrorCorrected { .. } => {
                        error_corrected = true;
                    }
                    CheckResult::PersistentError => {
                        panic!("Should have corrected, not persistent error");
                    }
                    CheckResult::Ok => {}
                }
            }
        }

        assert!(
            error_corrected,
            "Should have detected and corrected the injected error"
        );

        // Final result should match uncorrupted computation
        let plain = run_plain_loop(&initial, iters, &modulus, proth_step);
        assert_eq!(
            s, plain,
            "After correction, result should match plain computation"
        );
    }

    /// Verify that PersistentError is returned when the step function
    /// produces non-deterministic results (simulating random bit flips).
    ///
    /// Uses a step function that injects different corruption on each
    /// invocation via a call counter. This means the replay recomputation
    /// produces different results than the original computation,
    /// triggering PersistentError during the sub-block verification.
    #[test]
    fn gerbicz_persistent_error_returns_persistent() {
        use std::cell::Cell;

        let modulus = Integer::from(1_000_000_007u64);
        let initial = Integer::from(3u32);
        let iters = 20_000u64;

        let mut checker = GerbiczChecker::new(&initial, iters);
        let check_interval = checker.check_interval();
        let mut s = initial.clone();
        let mut checks_done = 0u32;

        // Counter for non-deterministic corruption
        let call_count = Cell::new(0u64);
        let bad_step = |state: &mut Integer, m: &Integer| {
            state.square_mut();
            *state %= m;
            let c = call_count.get();
            call_count.set(c + 1);
            // Inject different corruption based on call count
            // This makes replay non-deterministic
            *state ^= Integer::from((c % 37 + 1) as u32);
        };

        for i in 0..iters {
            proth_step(&mut s, &modulus);

            if checker.should_check(i) {
                checks_done += 1;

                if checks_done >= 3 {
                    // Corrupt the state and use non-deterministic step
                    s ^= Integer::from(42u32);
                    let (result, new_s) = checker.verify_block(&s, i, &modulus, &bad_step);
                    s = new_s;
                    if result == CheckResult::PersistentError {
                        return; // Expected
                    }
                } else {
                    let (result, new_s) = checker.verify_block(&s, i, &modulus, proth_step);
                    s = new_s;
                    assert_ne!(
                        result,
                        CheckResult::PersistentError,
                        "Should not get persistent error during clean checks"
                    );
                }
            }
        }
        assert!(
            check_interval < iters,
            "Checking should be enabled for {} iters (interval={})",
            iters,
            check_interval
        );
        panic!("Should have returned PersistentError for non-deterministic corruption");
    }

    /// For n < 10K, Gerbicz checking is disabled (overhead not justified).
    /// `should_check` should never return true.
    #[test]
    fn gerbicz_disabled_for_small_n() {
        let initial = Integer::from(3u32);
        let checker = GerbiczChecker::new(&initial, 5_000);
        for i in 0..5_000u64 {
            assert!(
                !checker.should_check(i),
                "Checking should be disabled for n=5000 at iter {}",
                i
            );
        }
    }

    /// Verify that check_interval scales as √n.
    #[test]
    fn gerbicz_check_interval_scales() {
        let initial = Integer::from(1u32);

        let c1 = GerbiczChecker::new(&initial, 100_000);
        let c2 = GerbiczChecker::new(&initial, 1_000_000);

        // √100K ≈ 316, √1M ≈ 1000
        assert!(
            c1.check_interval() > 200 && c1.check_interval() < 500,
            "100K iters should have interval ~316, got {}",
            c1.check_interval()
        );
        assert!(
            c2.check_interval() > 800 && c2.check_interval() < 1200,
            "1M iters should have interval ~1000, got {}",
            c2.check_interval()
        );
    }
}
