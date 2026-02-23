//! # Bignum — Abstraction Layer for Native/WASM Compilation
//!
//! Provides a unified big integer interface that selects between GMP (`rug`)
//! on native targets and pure-Rust `num-bigint` on WASM targets.
//!
//! ## Native (default)
//!
//! Uses `rug::Integer` backed by GMP for maximum performance. GMP is
//! hand-tuned assembly for x86_64/ARM64 and is 10-100× faster than
//! pure-Rust alternatives for multi-thousand-digit arithmetic.
//!
//! ## WASM (`--features wasm`)
//!
//! Uses `num_bigint::BigInt` for a pure-Rust implementation that compiles
//! to WASM without native dependencies. Performance is adequate for
//! demonstration and small searches but not competitive with GMP for
//! large candidates.
//!
//! ## Usage
//!
//! ```ignore
//! use darkreach::bignum::BigInteger;
//! let n = BigInteger::from(42u32);
//! assert!(is_probably_prime_native(&n, 25));
//! ```
//!
//! ## Feature Gate
//!
//! ```toml
//! [features]
//! wasm = ["dep:num-bigint", "dep:num-traits", "dep:num-integer"]
//! ```

// ── Native Target (default) ───────────────────────────────────────────────

/// Re-export the appropriate big integer type based on target.
#[cfg(not(feature = "wasm"))]
pub use rug::Integer as BigInteger;

/// Miller-Rabin primality test on native `rug::Integer`.
#[cfg(not(feature = "wasm"))]
pub fn is_probably_prime_native(n: &rug::Integer, rounds: u32) -> bool {
    use rug::integer::IsPrime;
    n.is_probably_prime(rounds) != IsPrime::No
}

// ── WASM Target ───────────────────────────────────────────────────────────

#[cfg(feature = "wasm")]
pub use num_bigint::BigInt as BigInteger;

/// Miller-Rabin primality test on pure-Rust `BigInt`.
///
/// Implements the standard probabilistic Miller-Rabin test using
/// `num-bigint` arithmetic. Each round has a false positive probability
/// of at most 1/4 (Rabin, 1980), so `rounds` rounds give error < 4^{-rounds}.
#[cfg(feature = "wasm")]
pub fn is_probably_prime_native(n: &num_bigint::BigInt, rounds: u32) -> bool {
    use num_bigint::BigInt;
    use num_integer::Integer as NumInteger;
    use num_traits::{One, Zero};

    let zero = BigInt::zero();
    let one = BigInt::one();
    let two = &one + &one;

    if *n < two {
        return false;
    }
    if *n == two {
        return true;
    }
    if n.is_even() {
        return false;
    }

    // Write n-1 = 2^s * d with d odd
    let n_minus_1 = n - &one;
    let mut d = n_minus_1.clone();
    let mut s: u64 = 0;
    while d.is_even() {
        d /= &two;
        s += 1;
    }

    // Test with small bases
    let bases: Vec<u64> = (0..rounds as u64).map(|i| i + 2).collect();

    'outer: for &a_val in &bases {
        let a = BigInt::from(a_val);
        if &a >= n {
            continue;
        }

        let mut x = a.modpow(&d, n);
        if x == one || x == n_minus_1 {
            continue;
        }

        for _ in 0..s - 1 {
            x = x.modpow(&two, n);
            if x == n_minus_1 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

/// Sequential fallback shim for rayon on WASM.
///
/// On WASM, `rayon` is not available (no threads), so we provide a
/// simple sequential iterator adapter that mimics `par_iter()`.
#[cfg(feature = "wasm")]
pub mod seq_iter {
    /// Sequential replacement for `rayon::par_iter().for_each()`.
    pub fn for_each<T, F>(items: &[T], f: F)
    where
        F: Fn(&T),
    {
        items.iter().for_each(f);
    }
}

#[cfg(test)]
mod tests {
    //! # Bignum Abstraction Tests
    //!
    //! Validates that the bignum abstraction layer works correctly on the
    //! native target (rug/GMP). WASM tests require `--features wasm`.

    use super::*;

    /// Known primes pass the native MR test.
    #[test]
    #[cfg(not(feature = "wasm"))]
    fn native_primes_pass() {
        for &p in &[2u32, 3, 5, 7, 11, 13, 101, 1009, 10007, 104729] {
            let n = rug::Integer::from(p);
            assert!(
                is_probably_prime_native(&n, 25),
                "Native MR rejected prime {}",
                p
            );
        }
    }

    /// Known composites fail the native MR test.
    #[test]
    #[cfg(not(feature = "wasm"))]
    fn native_composites_fail() {
        for &c in &[4u32, 6, 8, 9, 15, 21, 25, 100, 1001] {
            let n = rug::Integer::from(c);
            assert!(
                !is_probably_prime_native(&n, 25),
                "Native MR accepted composite {}",
                c
            );
        }
    }

    /// BigInteger type is usable.
    #[test]
    fn big_integer_construction() {
        let n = BigInteger::from(42u32);
        assert!(n > BigInteger::from(0u32));
    }
}
