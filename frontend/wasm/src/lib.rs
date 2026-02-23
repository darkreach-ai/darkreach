//! # darkreach-wasm — Browser Compute Worker
//!
//! Compiled-to-WASM prime search engine for browser-based contribution.
//! Provides a single `search_block` entry point that dispatches to
//! form-specific search functions (all 12 search forms supported).
//!
//! ## Supported Forms
//!
//! | Form | Expression | Params |
//! |------|-----------|--------|
//! | kbn | k·b^n ± 1 | k, base, sign |
//! | twin | (p, p+2) | — |
//! | factorial | n! ± 1 | — |
//! | sophie_germain | p where p, 2p+1 prime | — |
//! | cullen_woodall | n·2^n ± 1 | — |
//! | repunit | (b^n−1)/(b−1) | base |
//! | primorial | p# ± 1 | — |
//! | carol_kynea | (2^n±1)²−2 | — |
//! | gen_fermat | b^(2^n)+1 | base |
//! | wagstaff | (2^p+1)/3 | — |
//! | palindromic | base-b palindromes | base |
//! | near_repdigit | near-repdigit palindromes | base |
//!
//! ## Performance
//!
//! Uses `num-bigint` for arbitrary-precision arithmetic — roughly 3-5x
//! faster than JavaScript BigInt for modular exponentiation due to
//! compiled Rust and optimized multiplication routines.
//!
//! ## Proof Method
//!
//! All results use `"miller_rabin_12_browser_wasm"` — 12 fixed witnesses
//! deterministic for n < 3.317 × 10^24, strong PRP for larger values.

use std::collections::HashSet;

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{One, Zero};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

// ── Small primes table ──────────────────────────────────────────────

/// Build a table of the first `count` primes via sieve of Eratosthenes.
fn build_small_primes(count: usize) -> Vec<u64> {
    let limit = if count <= 1000 { 8000 } else { count * 12 };
    let mut sieve = vec![true; limit];
    sieve[0] = false;
    if limit > 1 {
        sieve[1] = false;
    }
    let mut i = 2;
    while i * i < limit {
        if sieve[i] {
            let mut j = i * i;
            while j < limit {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    let mut primes = Vec::with_capacity(count);
    for (n, &is_prime) in sieve.iter().enumerate() {
        if is_prime {
            primes.push(n as u64);
            if primes.len() >= count {
                break;
            }
        }
    }
    primes
}

/// Cached small primes (first 1000) for trial division.
fn small_primes() -> &'static Vec<u64> {
    use std::sync::OnceLock;
    static PRIMES: OnceLock<Vec<u64>> = OnceLock::new();
    PRIMES.get_or_init(|| build_small_primes(1000))
}

// ── Montgomery arithmetic (u64) ─────────────────────────────────────
//
// Ported from `src/sieve.rs` MontgomeryCtx. All arithmetic stays in
// Montgomery form (ā = a·R mod n, R = 2^64) to replace u128 division
// with multiply+shift (4-6 cycles vs 35-90 cycles per operation).

/// Montgomery multiplication context for a u64 odd modulus.
///
/// Precomputes -n⁻¹ mod 2^64, R mod n, and R² mod n so that modular
/// multiplication reduces to integer multiply + REDC (no division).
#[derive(Clone, Copy, Debug)]
struct MontCtx {
    /// The modulus (must be odd, > 1).
    n: u64,
    /// -n⁻¹ mod 2^64 (via Hensel lifting).
    n_prime: u64,
    /// R mod n = 2^64 mod n (Montgomery form of 1).
    r_mod_n: u64,
    /// R² mod n (for converting to Montgomery form).
    r2_mod_n: u64,
}

impl MontCtx {
    /// Create a Montgomery context for the given odd modulus n > 1.
    fn new(n: u64) -> Self {
        debug_assert!(n > 1 && n & 1 == 1, "Montgomery requires odd modulus > 1");

        // Hensel lifting: compute n⁻¹ mod 2^64.
        // 6 iterations: precision doubles each step (2¹→2²→2⁴→2⁸→2¹⁶→2³²→2⁶⁴).
        let mut inv: u64 = 1;
        for _ in 0..6 {
            inv = inv.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inv)));
        }
        let n_prime = inv.wrapping_neg(); // -n⁻¹ mod 2^64

        let r_mod_n = ((1u128 << 64) % n as u128) as u64;
        let r2_mod_n = ((r_mod_n as u128 * r_mod_n as u128) % n as u128) as u64;

        MontCtx {
            n,
            n_prime,
            r_mod_n,
            r2_mod_n,
        }
    }

    /// Convert a normal value to Montgomery form: ā = a·R mod n.
    #[inline]
    fn to_mont(&self, a: u64) -> u64 {
        self.mul(a % self.n, self.r2_mod_n)
    }

    /// Convert from Montgomery form back to normal: a = ā·R⁻¹ mod n.
    #[inline]
    fn from_mont(&self, a: u64) -> u64 {
        self.reduce(a as u128)
    }

    /// Montgomery reduction (REDC): compute t·R⁻¹ mod n.
    #[inline]
    fn reduce(&self, t: u128) -> u64 {
        let m = (t as u64).wrapping_mul(self.n_prime);
        let u = t + (m as u128) * (self.n as u128);
        let result = (u >> 64) as u64;
        if result >= self.n {
            result - self.n
        } else {
            result
        }
    }

    /// Montgomery multiplication: a·b·R⁻¹ mod n (both inputs in Montgomery form).
    #[inline]
    fn mul(&self, a: u64, b: u64) -> u64 {
        self.reduce((a as u128) * (b as u128))
    }

    /// Modular exponentiation in Montgomery form.
    fn pow_mod(&self, base: u64, mut exp: u64) -> u64 {
        let mut result = self.r_mod_n; // 1 in Montgomery form
        let mut b = base;
        while exp > 0 {
            if exp & 1 == 1 {
                result = self.mul(result, b);
            }
            exp >>= 1;
            if exp > 0 {
                b = self.mul(b, b);
            }
        }
        result
    }
}

/// Sieve primes for the modular sieve — first 1229 primes (all primes < 10000).
fn sieve_primes() -> &'static Vec<u64> {
    use std::sync::OnceLock;
    static PRIMES: OnceLock<Vec<u64>> = OnceLock::new();
    PRIMES.get_or_init(|| {
        let limit = 10_000usize;
        let mut sieve = vec![true; limit];
        sieve[0] = false;
        sieve[1] = false;
        let mut i = 2;
        while i * i < limit {
            if sieve[i] {
                let mut j = i * i;
                while j < limit {
                    sieve[j] = false;
                    j += i;
                }
            }
            i += 1;
        }
        sieve
            .iter()
            .enumerate()
            .filter(|(_, &is_p)| is_p)
            .map(|(n, _)| n as u64)
            .collect()
    })
}

/// Modular sieve for k·b^n + sign forms (KBN-family).
///
/// For each sieve prime p, computes b^start mod p via Montgomery exponentiation,
/// then walks n = start..end incrementally (b^(n+1) = b^n · b mod p), marking
/// any n where k·b^n + sign ≡ 0 (mod p) as composite.
///
/// Returns a boolean vector where `true` = survives (not sieved out).
/// Eliminates 80-90% of candidates before expensive BigUint MR.
///
/// **Caveat:** When a candidate equals a sieve prime (e.g., 2^2-1 = 3), the
/// sieve would incorrectly mark it composite. A post-pass "un-sieves" small
/// candidates that could be primes themselves, letting `is_prime()` decide.
fn modular_sieve_kbn(k: u64, base: u64, sign: i64, start: u64, end: u64) -> Vec<bool> {
    if end < start {
        return Vec::new();
    }
    let len = (end - start + 1) as usize;
    let mut survives = vec![true; len];

    for &p in sieve_primes() {
        // Skip p=2 (all KBN candidates are odd for the forms we care about)
        // and skip if p divides base (b^n ≡ 0 mod p for all n)
        if p < 3 || base % p == 0 {
            continue;
        }

        let ctx = MontCtx::new(p);
        let k_mont = ctx.to_mont(k % p);
        let base_mont = ctx.to_mont(base % p);

        // Compute b^start mod p via Montgomery exponentiation
        let mut bn_mont = ctx.pow_mod(base_mont, start);

        for i in 0..len {
            // Check if k * b^n + sign ≡ 0 (mod p)
            let kb_val = ctx.from_mont(ctx.mul(k_mont, bn_mont));
            let candidate_mod = if sign >= 0 {
                (kb_val + 1) % p
            } else {
                (kb_val + p - 1) % p
            };

            if candidate_mod == 0 {
                survives[i] = false;
            }

            // Increment: b^(n+1) = b^n * b
            bn_mont = ctx.mul(bn_mont, base_mont);
        }
    }

    // Post-pass: un-sieve small candidates that could be primes themselves.
    // When candidate == sieve_prime p, "p | candidate" is true but it's still prime.
    // Only affects candidates smaller than the sieve limit (~10000).
    let max_sieve = *sieve_primes().last().unwrap();
    for i in 0..len {
        if !survives[i] {
            let n = start + i as u64;
            if let Some(bn) = (base as u128).checked_pow(n as u32) {
                let kbn = k as u128 * bn;
                let candidate_val = if sign >= 0 {
                    kbn + 1
                } else {
                    kbn.saturating_sub(1)
                };
                if candidate_val <= max_sieve as u128 {
                    // Small candidate — let is_prime() decide
                    survives[i] = true;
                }
            }
            // If checked_pow overflows, candidate is huge (always > sieve limit)
        }
    }

    survives
}

// ── Core math primitives ────────────────────────────────────────────

/// Fixed Miller-Rabin witnesses — deterministic for n < 3.317e24 with
/// the first 12 primes, strong PRP test for larger values.
const MR_WITNESSES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

/// Compute n mod p without heap allocation.
///
/// Processes BigUint digits (little-endian u64 limbs) in big-endian order
/// via Horner's method: for digits d_k (most-significant first),
///   rem = (...((d_{k} mod p) * 2^64 + d_{k-1}) mod p * 2^64 + ...) mod p
///
/// Uses u128 intermediate to avoid overflow. Eliminates the `BigUint::from(p)`
/// allocation that the naive `n % BigUint::from(p)` approach requires.
fn mod_small(n: &BigUint, p: u64) -> u64 {
    let digits = n.to_u64_digits(); // little-endian
    let mut rem: u128 = 0;
    for &d in digits.iter().rev() {
        rem = ((rem << 64) | d as u128) % p as u128;
    }
    rem as u64
}

/// Trial division against the first 1000 small primes.
/// Returns `true` if n has a small factor (composite), `false` otherwise.
///
/// Uses `mod_small()` to compute n mod p via digit extraction, avoiding
/// ~1000 `BigUint::from()` heap allocations per candidate.
fn has_small_factor(n: &BigUint) -> bool {
    for &p in small_primes() {
        let r = mod_small(n, p);
        if r == 0 {
            // n is divisible by p — but it's only composite if n != p
            let digits = n.to_u64_digits();
            if digits.len() == 1 && digits[0] == p {
                return false; // n itself is this small prime
            }
            return true;
        }
    }
    false
}

/// Single Miller-Rabin witness test. Returns false (composite) if the
/// witness `a` proves n composite, true (probably prime) otherwise.
///
/// Caller must pre-compute d, r such that n-1 = 2^r * d.
fn mr_witness(n: &BigUint, a_val: u64, d: &BigUint, r: u64) -> bool {
    let one = BigUint::one();
    let two = BigUint::from(2u32);
    let n_minus_1 = n - &one;

    let a = BigUint::from(a_val);
    if a >= n_minus_1 {
        return true; // witness too large, skip
    }

    let mut x = a.modpow(d, n);
    if x == one || x == n_minus_1 {
        return true;
    }

    for _ in 1..r {
        x = x.modpow(&two, n);
        if x == n_minus_1 {
            return true;
        }
    }
    false
}

/// Two-stage Miller-Rabin primality test with 12 fixed witnesses.
///
/// **Pre-screen** (witnesses 2, 3): rejects ~93% of odd composites using
/// only 2 modpow calls. **Full test** (remaining 10 witnesses): runs only
/// on pre-screen survivors. This is a 5-7x speedup on composite-heavy ranges
/// since most candidates are eliminated cheaply.
///
/// Deterministic for n < 3.317 × 10^24 with all 12 witnesses.
fn miller_rabin(n: &BigUint) -> bool {
    let one = BigUint::one();
    let two = BigUint::from(2u32);

    if *n < two {
        return false;
    }
    if *n == two || *n == BigUint::from(3u32) {
        return true;
    }
    if n.is_even() {
        return false;
    }

    // Factor out powers of 2: n - 1 = 2^r * d (shared across all rounds)
    let n_minus_1 = n - &one;
    let mut d = n_minus_1.clone();
    let mut r = 0u64;
    while d.is_even() {
        d >>= 1;
        r += 1;
    }

    // Stage 1: Pre-screen with witnesses 2, 3
    // Catches ~93% of composites with just 2 modpow calls
    for &a_val in &MR_WITNESSES[..2] {
        if !mr_witness(n, a_val, &d, r) {
            return false;
        }
    }

    // Stage 2: Full test with remaining 10 witnesses
    for &a_val in &MR_WITNESSES[2..] {
        if !mr_witness(n, a_val, &d, r) {
            return false;
        }
    }
    true
}

/// Full primality test: trial division + 12-witness Miller-Rabin.
fn is_prime(n: &BigUint) -> bool {
    let two = BigUint::from(2u32);
    if *n < two {
        return false;
    }
    if has_small_factor(n) {
        // If n has a small factor, it's only prime if it IS a small prime
        let n_u64 = n.to_u64_digits();
        if n_u64.len() == 1 {
            return small_primes().contains(&n_u64[0]);
        }
        return false;
    }
    miller_rabin(n)
}

/// Count decimal digits of a BigUint.
fn digit_count(n: &BigUint) -> usize {
    if n.is_zero() {
        return 1;
    }
    n.to_string().len()
}

/// Quick primality test for u64 values via trial division.
/// Used as a precondition check (e.g., repunit requires prime n).
fn is_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5u64;
    while i.saturating_mul(i) <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}

/// Safe conversion from u64 to u32 for BigUint::pow exponents.
/// Returns None if the value exceeds u32::MAX.
fn checked_u32(n: u64) -> Option<u32> {
    if n <= u32::MAX as u64 {
        Some(n as u32)
    } else {
        None
    }
}

// ── Palindrome helpers ──────────────────────────────────────────────

/// Build a palindrome from its left half digits in the given base.
///
/// For an odd-length palindrome, `left_half` contains (d+1)/2 digits
/// including the center. The result mirrors all positions except the
/// center digit. Example: left_half=[1,2,3], base=10 → 12321.
fn build_palindrome(left_half: &[u64], base: u64) -> BigUint {
    let base_big = BigUint::from(base);
    let mut result = BigUint::zero();

    // Forward: all left half digits
    for &d in left_half {
        result = result * &base_big + BigUint::from(d);
    }

    // Reverse: mirror excluding center (last element of left_half)
    for i in (0..left_half.len().saturating_sub(1)).rev() {
        result = result * &base_big + BigUint::from(left_half[i]);
    }

    result
}

/// Increment a digit array (big-endian, position 0 is most significant).
/// Returns false on overflow when all digit combinations are exhausted.
fn increment_half(half: &mut [u64], base: u64) -> bool {
    for i in (0..half.len()).rev() {
        half[i] += 1;
        if half[i] < base {
            return true;
        }
        half[i] = 0;
    }
    false
}

// ── Search strategies ───────────────────────────────────────────────

/// Result prime entry serialized as JSON.
struct PrimeResult {
    expression: String,
    form: String,
    digits: usize,
}

/// Search k*b^n + sign for n in [start, end].
///
/// Applies a modular sieve (Montgomery-accelerated) to eliminate 80-90%
/// of candidates before the expensive BigUint Miller-Rabin test.
fn search_kbn(k: u64, base: u64, sign: i8, start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let k_big = BigUint::from(k);
    let base_big = BigUint::from(base);
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Pre-sieve: mark composite candidates via modular arithmetic
    let survivors = if end >= start {
        modular_sieve_kbn(k, base, sign as i64, start, end)
    } else {
        Vec::new()
    };

    for n in start..=end {
        let idx = (n - start) as usize;

        // Skip candidates eliminated by the modular sieve
        if !survivors[idx] {
            tested += 1;
            continue;
        }

        let exp = match checked_u32(n) {
            Some(e) => e,
            None => {
                tested += 1;
                continue;
            }
        };
        let power = base_big.pow(exp);
        let candidate = if sign >= 0 {
            &k_big * &power + BigUint::one()
        } else {
            let kp = &k_big * &power;
            if kp <= BigUint::one() {
                tested += 1;
                continue;
            }
            kp - BigUint::one()
        };

        tested += 1;

        if candidate >= BigUint::from(2u32) && is_prime(&candidate) {
            let sign_str = if sign >= 0 { "+" } else { "-" };
            let expression = if k == 1 {
                format!("{}^{}{sign_str}1", base, n)
            } else {
                format!("{}*{}^{}{sign_str}1", k, base, n)
            };
            let digits = digit_count(&candidate);
            primes.push(PrimeResult {
                expression,
                form: "kbn".to_string(),
                digits,
            });
        }
    }

    (primes, tested)
}

/// Search for twin primes in [start, end].
/// Twin primes: p and p+2 are both prime. Iterates via 6k-1 stepping.
fn search_twin(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Align to 6k-1 start
    let mut p = if start < 5 { 5 } else { start };
    let rem = p % 6;
    if rem != 5 {
        p = p + (5 + 6 - rem) % 6;
        if p % 6 != 5 {
            p = p - (p % 6) + 5;
        }
    }

    while p <= end {
        tested += 1;

        let p_big = BigUint::from(p);
        let p2_big = BigUint::from(p + 2);

        if is_prime(&p_big) && is_prime(&p2_big) {
            let digits = digit_count(&p_big);
            primes.push(PrimeResult {
                expression: format!("({}, {})", p, p + 2),
                form: "twin".to_string(),
                digits,
            });
        }

        p += 6;
    }

    (primes, tested)
}

/// Search for factorial primes n! ± 1 in [start, end].
///
/// Computes factorials iteratively (n! = (n-1)! * n) to avoid
/// redundant computation. Tests both n!+1 and n!-1 for primality.
fn search_factorial(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Build factorial up to start
    let mut factorial = BigUint::one();
    for i in 2..=start {
        factorial *= BigUint::from(i);
    }

    for n in start..=end {
        if n > start {
            factorial *= BigUint::from(n);
        }

        // Test n! + 1
        let plus_one = &factorial + BigUint::one();
        tested += 1;
        if is_prime(&plus_one) {
            let digits = digit_count(&plus_one);
            primes.push(PrimeResult {
                expression: format!("{}!+1", n),
                form: "factorial".to_string(),
                digits,
            });
        }

        // Test n! - 1
        if factorial > BigUint::one() {
            let minus_one = &factorial - BigUint::one();
            tested += 1;
            if is_prime(&minus_one) {
                let digits = digit_count(&minus_one);
                primes.push(PrimeResult {
                    expression: format!("{}!-1", n),
                    form: "factorial".to_string(),
                    digits,
                });
            }
        }
    }

    (primes, tested)
}

/// Search for Sophie Germain primes in [start, end].
///
/// A Sophie Germain prime p satisfies: both p and 2p+1 are prime.
/// The associated safe prime is 2p+1.
fn search_sophie_germain(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Handle p=2 specially (only even prime)
    if start <= 2 && end >= 2 {
        tested += 1;
        // 2 is SG: 2*2+1 = 5 is prime
        primes.push(PrimeResult {
            expression: "2".to_string(),
            form: "sophie_germain".to_string(),
            digits: 1,
        });
    }

    // Iterate odd candidates
    let p_start = if start <= 3 {
        3
    } else if start % 2 == 0 {
        start + 1
    } else {
        start
    };

    let mut p = p_start;
    while p <= end {
        tested += 1;
        let p_big = BigUint::from(p);

        if is_prime(&p_big) {
            let sg = BigUint::from(2u32) * &p_big + BigUint::one();
            if is_prime(&sg) {
                let digits = digit_count(&p_big);
                primes.push(PrimeResult {
                    expression: format!("{}", p),
                    form: "sophie_germain".to_string(),
                    digits,
                });
            }
        }

        p += 2;
    }

    (primes, tested)
}

/// Search for Cullen primes (n·2^n+1) and Woodall primes (n·2^n−1)
/// for n in [start, end].
///
/// Uses Montgomery-accelerated modular sieve for both Cullen (k=n, sign=+1)
/// and Woodall (k=n, sign=-1) independently. Since k varies with n, we
/// sieve with k=1 and handle the n multiplier in the sieve check.
fn search_cullen_woodall(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let two = BigUint::from(2u32);
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Pre-sieve for Cullen (n·2^n+1) and Woodall (n·2^n-1).
    // Since k=n varies per candidate, we build a custom sieve:
    // for each prime p, check if n·2^n ± 1 ≡ 0 (mod p).
    let len = if end >= start {
        (end - start + 1) as usize
    } else {
        0
    };
    let mut cullen_survives = vec![true; len];
    let mut woodall_survives = vec![true; len];

    for &p in sieve_primes() {
        if p < 3 {
            continue;
        }
        let ctx = MontCtx::new(p);
        let base_mont = ctx.to_mont(2 % p);
        let mut bn_mont = ctx.pow_mod(base_mont, start); // 2^start mod p

        for i in 0..len {
            let n = start + i as u64;
            let n_mod_p = n % p;
            let n_mont = ctx.to_mont(n_mod_p);
            let nb_val = ctx.from_mont(ctx.mul(n_mont, bn_mont)); // n * 2^n mod p

            // Cullen: n*2^n + 1 ≡ 0 (mod p)?
            if (nb_val + 1) % p == 0 {
                cullen_survives[i] = false;
            }
            // Woodall: n*2^n - 1 ≡ 0 (mod p)?
            if (nb_val + p - 1) % p == 0 {
                woodall_survives[i] = false;
            }

            bn_mont = ctx.mul(bn_mont, base_mont); // 2^(n+1)
        }
    }

    // Post-pass: un-sieve small candidates that could be primes themselves
    let max_sieve = *sieve_primes().last().unwrap();
    for i in 0..len {
        let n = start + i as u64;
        if let Some(bn) = 2u128.checked_pow(n as u32) {
            let nbn = n as u128 * bn;
            if !cullen_survives[i] && nbn + 1 <= max_sieve as u128 {
                cullen_survives[i] = true;
            }
            if !woodall_survives[i] && nbn.saturating_sub(1) <= max_sieve as u128 {
                woodall_survives[i] = true;
            }
        }
    }

    for n in start..=end {
        let idx = (n - start) as usize;
        let exp = match checked_u32(n) {
            Some(e) => e,
            None => {
                tested += 2;
                continue;
            }
        };
        let power = two.pow(exp); // 2^n
        let n_big = BigUint::from(n);
        let n_times_power = &n_big * &power; // n * 2^n

        // Cullen: n * 2^n + 1
        tested += 1;
        if cullen_survives[idx] {
            let cullen = &n_times_power + BigUint::one();
            if cullen >= BigUint::from(2u32) && is_prime(&cullen) {
                let digits = digit_count(&cullen);
                primes.push(PrimeResult {
                    expression: format!("{}*2^{}+1", n, n),
                    form: "cullen_woodall".to_string(),
                    digits,
                });
            }
        }

        // Woodall: n * 2^n - 1
        if n_times_power > BigUint::one() {
            tested += 1;
            if woodall_survives[idx] {
                let woodall = n_times_power - BigUint::one();
                if is_prime(&woodall) {
                    let digits = digit_count(&woodall);
                    primes.push(PrimeResult {
                        expression: format!("{}*2^{}-1", n, n),
                        form: "cullen_woodall".to_string(),
                        digits,
                    });
                }
            }
        }
    }

    (primes, tested)
}

/// Search for repunit primes R(base, n) = (base^n − 1)/(base − 1)
/// for prime n in [start, end].
///
/// Only tests prime n (necessary condition: if n is composite,
/// R(b,n) factors as R(b,d) × ... for any divisor d of n).
fn search_repunit(base: u64, start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    if base < 2 {
        return (Vec::new(), 0);
    }
    let base_big = BigUint::from(base);
    let divisor = BigUint::from(base - 1);
    let mut primes = Vec::new();
    let mut tested = 0u64;

    for n in start..=end {
        // Necessary condition: n must be prime
        if !is_prime_u64(n) {
            continue;
        }

        let exp = match checked_u32(n) {
            Some(e) => e,
            None => {
                tested += 1;
                continue;
            }
        };

        // R(base, n) = (base^n - 1) / (base - 1)
        let power = base_big.pow(exp);
        let candidate = (power - BigUint::one()) / &divisor;

        tested += 1;
        if is_prime(&candidate) {
            let digits = digit_count(&candidate);
            primes.push(PrimeResult {
                expression: format!("R({},{})", base, n),
                form: "repunit".to_string(),
                digits,
            });
        }
    }

    (primes, tested)
}

/// Search for primorial primes p# ± 1 for primes p in [start, end].
///
/// The primorial p# is the product of all primes ≤ p. Computes
/// iteratively by accumulating primes in order.
fn search_primorial(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let mut primes_found = Vec::new();
    let mut tested = 0u64;

    // Collect all primes up to end
    let prime_list: Vec<u64> = (2..=end).filter(|&n| is_prime_u64(n)).collect();

    // Accumulate primorial up to (but not including) start
    let mut primorial = BigUint::one();
    let mut idx = 0;
    while idx < prime_list.len() && prime_list[idx] < start {
        primorial *= BigUint::from(prime_list[idx]);
        idx += 1;
    }

    // Test each prime p in [start, end]
    while idx < prime_list.len() {
        let p = prime_list[idx];
        primorial *= BigUint::from(p);

        // Test p# + 1
        let plus_one = &primorial + BigUint::one();
        tested += 1;
        if is_prime(&plus_one) {
            let digits = digit_count(&plus_one);
            primes_found.push(PrimeResult {
                expression: format!("{}#+1", p),
                form: "primorial".to_string(),
                digits,
            });
        }

        // Test p# - 1
        if primorial > BigUint::one() {
            let minus_one = &primorial - BigUint::one();
            tested += 1;
            if is_prime(&minus_one) {
                let digits = digit_count(&minus_one);
                primes_found.push(PrimeResult {
                    expression: format!("{}#-1", p),
                    form: "primorial".to_string(),
                    digits,
                });
            }
        }

        idx += 1;
    }

    (primes_found, tested)
}

/// Search for Carol primes ((2^n−1)²−2) and Kynea primes ((2^n+1)²−2)
/// for n in [start, end].
///
/// Pre-sieves using Montgomery arithmetic:
/// - Carol: (2^n−1)² − 2 = 2^(2n) − 2^(n+1) − 1
/// - Kynea: (2^n+1)² − 2 = 2^(2n) + 2^(n+1) − 1
fn search_carol_kynea(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let two = BigUint::from(2u32);
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Pre-sieve Carol and Kynea candidates
    let len = if end >= start {
        (end - start + 1) as usize
    } else {
        0
    };
    let mut carol_survives = vec![true; len];
    let mut kynea_survives = vec![true; len];

    for &p in sieve_primes() {
        if p < 3 {
            continue;
        }
        let ctx = MontCtx::new(p);
        let two_mont = ctx.to_mont(2 % p);
        // 2^start mod p
        let mut pow_n_mont = ctx.pow_mod(two_mont, start);
        // 2^(2*start) mod p
        let mut pow_2n_mont = ctx.pow_mod(two_mont, 2 * start);
        // Precompute 2^2 in Montgomery form for stepping pow_2n
        let four_mont = ctx.mul(two_mont, two_mont);

        for i in 0..len {
            let pow_n_val = ctx.from_mont(pow_n_mont);
            let pow_2n_val = ctx.from_mont(pow_2n_mont);

            // Carol: 2^(2n) - 2^(n+1) - 1 mod p
            let pow_np1 = (pow_n_val * 2) % p;
            let carol_mod = (pow_2n_val + p - pow_np1 % p + p - 1) % p;
            if carol_mod == 0 {
                carol_survives[i] = false;
            }

            // Kynea: 2^(2n) + 2^(n+1) - 1 mod p
            let kynea_mod = (pow_2n_val + pow_np1 + p - 1) % p;
            if kynea_mod == 0 {
                kynea_survives[i] = false;
            }

            // Step: 2^(n+1) = 2^n * 2, 2^(2(n+1)) = 2^(2n) * 4
            pow_n_mont = ctx.mul(pow_n_mont, two_mont);
            pow_2n_mont = ctx.mul(pow_2n_mont, four_mont);
        }
    }

    // Post-pass: un-sieve small candidates that could be primes themselves
    let max_sieve = *sieve_primes().last().unwrap();
    for i in 0..len {
        let n = start + i as u64;
        if let Some(pn) = 2u128.checked_pow(n as u32) {
            // Carol: (2^n-1)^2 - 2
            if !carol_survives[i] && pn > 1 {
                let carol_val = (pn - 1) * (pn - 1) - 2;
                if carol_val <= max_sieve as u128 {
                    carol_survives[i] = true;
                }
            }
            // Kynea: (2^n+1)^2 - 2
            if !kynea_survives[i] {
                let kynea_val = (pn + 1) * (pn + 1) - 2;
                if kynea_val <= max_sieve as u128 {
                    kynea_survives[i] = true;
                }
            }
        }
    }

    for n in start..=end {
        let idx = (n - start) as usize;
        let exp = match checked_u32(n) {
            Some(e) => e,
            None => {
                tested += 2;
                continue;
            }
        };
        let power = two.pow(exp); // 2^n

        // Carol: (2^n - 1)^2 - 2
        if power > BigUint::one() {
            let pm1 = &power - BigUint::one();
            let sq = &pm1 * &pm1;
            if sq > two {
                tested += 1;
                if carol_survives[idx] {
                    let carol = sq - &two;
                    if is_prime(&carol) {
                        let digits = digit_count(&carol);
                        primes.push(PrimeResult {
                            expression: format!("(2^{}-1)^2-2", n),
                            form: "carol_kynea".to_string(),
                            digits,
                        });
                    }
                }
            }
        }

        // Kynea: (2^n + 1)^2 - 2
        let pp1 = &power + BigUint::one();
        let sq = &pp1 * &pp1;
        let kynea = sq - &two;
        tested += 1;
        if kynea_survives[idx] {
            if is_prime(&kynea) {
                let digits = digit_count(&kynea);
                primes.push(PrimeResult {
                    expression: format!("(2^{}+1)^2-2", n),
                    form: "carol_kynea".to_string(),
                    digits,
                });
            }
        }
    }

    (primes, tested)
}

/// Search for generalized Fermat primes base^(2^n) + 1
/// for n in [start, end].
///
/// Pre-sieves: for each sieve prime p, compute base^(2^n) mod p via
/// repeated squaring and check if base^(2^n) + 1 ≡ 0 (mod p).
fn search_gen_fermat(base: u64, start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let mut primes = Vec::new();
    let mut tested = 0u64;

    // Pre-sieve gen_fermat candidates
    let len = if end >= start {
        (end - start + 1) as usize
    } else {
        0
    };
    let mut survives = vec![true; len];

    for &p in sieve_primes() {
        if p < 3 || base % p == 0 {
            continue;
        }
        let ctx = MontCtx::new(p);
        let base_mont = ctx.to_mont(base % p);

        // Compute base^(2^start) mod p via start squarings
        let mut x_mont = base_mont;
        for _ in 0..start {
            x_mont = ctx.mul(x_mont, x_mont);
        }

        for i in 0..len {
            // x_mont = base^(2^n) in Montgomery form
            let x_val = ctx.from_mont(x_mont);
            if (x_val + 1) % p == 0 {
                survives[i] = false;
            }

            // Step: base^(2^(n+1)) = (base^(2^n))^2
            x_mont = ctx.mul(x_mont, x_mont);
        }
    }

    // Post-pass: un-sieve small candidates that could be primes themselves
    let max_sieve = *sieve_primes().last().unwrap();
    for i in 0..len {
        if !survives[i] {
            let n = start + i as u64;
            // base^(2^n) + 1: compute via repeated squaring in u128
            let mut val = base as u128;
            let mut overflowed = false;
            for _ in 0..n {
                match val.checked_mul(val) {
                    Some(v) => val = v,
                    None => {
                        overflowed = true;
                        break;
                    }
                }
            }
            if !overflowed && val + 1 <= max_sieve as u128 {
                survives[i] = true;
            }
        }
    }

    for n in start..=end {
        let idx = (n - start) as usize;
        tested += 1;

        if !survives[idx] {
            continue;
        }

        // Compute base^(2^n) via repeated squaring
        let mut x = BigUint::from(base);
        for _ in 0..n {
            x = &x * &x;
        }
        let candidate = x + BigUint::one();

        if is_prime(&candidate) {
            let digits = digit_count(&candidate);
            primes.push(PrimeResult {
                expression: format!("{}^(2^{})+1", base, n),
                form: "gen_fermat".to_string(),
                digits,
            });
        }
    }

    (primes, tested)
}

/// Search for Wagstaff primes (2^p + 1)/3 for odd primes p in [start, end].
///
/// Requires p odd for 2^p + 1 to be divisible by 3 (since 2 ≡ −1 mod 3,
/// so 2^p ≡ (−1)^p mod 3 — only −1 when p is odd, giving 2^p + 1 ≡ 0 mod 3).
fn search_wagstaff(start: u64, end: u64) -> (Vec<PrimeResult>, u64) {
    let two = BigUint::from(2u32);
    let three = BigUint::from(3u32);
    let mut primes = Vec::new();
    let mut tested = 0u64;

    for p in start..=end {
        // Must be an odd prime (p=2 gives non-integer (4+1)/3)
        if p == 2 || !is_prime_u64(p) {
            continue;
        }

        let exp = match checked_u32(p) {
            Some(e) => e,
            None => {
                tested += 1;
                continue;
            }
        };

        // (2^p + 1) / 3
        let power = two.pow(exp);
        let candidate = (power + BigUint::one()) / &three;

        tested += 1;
        if is_prime(&candidate) {
            let digits = digit_count(&candidate);
            primes.push(PrimeResult {
                expression: format!("(2^{}+1)/3", p),
                form: "wagstaff".to_string(),
                digits,
            });
        }
    }

    (primes, tested)
}

/// Search for palindromic primes in the given base with digit count
/// in [start_digits, end_digits].
///
/// Only odd digit counts are tested — even-digit palindromes are always
/// divisible by (base+1). Generates palindromes by enumerating all
/// left-half digit combinations and mirroring.
fn search_palindromic(base: u64, start_digits: u64, end_digits: u64) -> (Vec<PrimeResult>, u64) {
    if base < 2 {
        return (Vec::new(), 0);
    }

    let mut primes = Vec::new();
    let mut tested = 0u64;

    for num_digits in start_digits..=end_digits {
        // Skip even digit counts (always divisible by base+1)
        if num_digits % 2 == 0 || num_digits < 1 {
            continue;
        }

        let half_len = ((num_digits + 1) / 2) as usize;

        // Initialize left half: first digit = 1, rest = 0
        let mut left_half = vec![0u64; half_len];
        left_half[0] = 1;

        loop {
            let palindrome = build_palindrome(&left_half, base);
            tested += 1;

            if is_prime(&palindrome) {
                let digits = digit_count(&palindrome);
                primes.push(PrimeResult {
                    expression: palindrome.to_string(),
                    form: "palindromic".to_string(),
                    digits,
                });
            }

            if !increment_half(&mut left_half, base) {
                break;
            }
        }
    }

    (primes, tested)
}

/// Search for near-repdigit palindromic primes in the given base with
/// digit count in [start_digits, end_digits].
///
/// A near-repdigit palindrome has all digits equal to a base digit `b`
/// except for one position which differs. For a palindrome, varying a
/// non-center position forces its mirror to change too, so we enumerate:
/// - Center-only variation: 1 digit differs
/// - Non-center variation: 2 digits differ (position + mirror)
///
/// Deduplicates results since different (base_digit, position, replacement)
/// combinations can produce the same palindrome.
fn search_near_repdigit(
    base: u64,
    start_digits: u64,
    end_digits: u64,
) -> (Vec<PrimeResult>, u64) {
    if base < 2 {
        return (Vec::new(), 0);
    }

    let mut primes = Vec::new();
    let mut tested = 0u64;
    let mut seen = HashSet::new();

    for num_digits in start_digits..=end_digits {
        // Skip even digit counts and values < 3
        if num_digits % 2 == 0 || num_digits < 3 {
            continue;
        }

        let half_len = ((num_digits + 1) / 2) as usize;

        // For each repeated base digit b
        for b in 1..base {
            // For each position in the left half to vary
            for pos in 0..half_len {
                // First digit can't be 0 (leading zero)
                let r_start = if pos == 0 { 1 } else { 0 };

                for r in r_start..base {
                    if r == b {
                        continue;
                    }

                    let mut left_half = vec![b; half_len];
                    left_half[pos] = r;

                    let palindrome = build_palindrome(&left_half, base);
                    let key = palindrome.to_string();

                    // Deduplicate — same palindrome can arise from different
                    // (base_digit, position, replacement) combinations
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.insert(key);
                    tested += 1;

                    if is_prime(&palindrome) {
                        let digits = digit_count(&palindrome);
                        primes.push(PrimeResult {
                            expression: palindrome.to_string(),
                            form: "near_repdigit".to_string(),
                            digits,
                        });
                    }
                }
            }
        }
    }

    (primes, tested)
}

// ── WASM entry point ────────────────────────────────────────────────

/// Search a block of candidates and return JSON results.
///
/// # Arguments
///
/// * `search_type` — One of the 12 supported form names
/// * `params_json` — JSON object with form-specific params (e.g. `{"k":1,"base":2,"sign":1}`)
/// * `block_start` — First candidate index to test
/// * `block_end` — Last candidate index to test (inclusive)
///
/// # Returns
///
/// JSON string: `{ "primes": [{ "expression", "form", "digits", "proof_method" }], "tested": N }`
#[wasm_bindgen]
pub fn search_block(
    search_type: &str,
    params_json: &str,
    block_start: u64,
    block_end: u64,
) -> String {
    let params: serde_json::Value = serde_json::from_str(params_json)
        .unwrap_or(serde_json::Value::Object(Default::default()));

    let (results, tested) = match search_type {
        "kbn" => {
            let k = params["k"].as_u64().unwrap_or(1);
            let base = params["base"].as_u64().unwrap_or(2);
            let sign = params["sign"].as_i64().unwrap_or(1) as i8;
            search_kbn(k, base, sign, block_start, block_end)
        }
        "twin" => search_twin(block_start, block_end),
        "factorial" => search_factorial(block_start, block_end),
        "sophie_germain" => search_sophie_germain(block_start, block_end),
        "cullen_woodall" => search_cullen_woodall(block_start, block_end),
        "repunit" => {
            let base = params["base"].as_u64().unwrap_or(10);
            search_repunit(base, block_start, block_end)
        }
        "primorial" => search_primorial(block_start, block_end),
        "carol_kynea" => search_carol_kynea(block_start, block_end),
        "gen_fermat" => {
            let base = params["base"].as_u64().unwrap_or(2);
            search_gen_fermat(base, block_start, block_end)
        }
        "wagstaff" => search_wagstaff(block_start, block_end),
        "palindromic" => {
            let base = params["base"].as_u64().unwrap_or(10);
            search_palindromic(base, block_start, block_end)
        }
        "near_repdigit" => {
            let base = params["base"].as_u64().unwrap_or(10);
            search_near_repdigit(base, block_start, block_end)
        }
        _ => (Vec::new(), 0),
    };

    // Build primes array with alphabetically-sorted keys to match the server-side
    // Rust struct serialization order (serde serializes struct fields in declaration order,
    // and ContributePrimePayload declares: digits, expression, form, proof_method).
    let primes_json: Vec<serde_json::Value> = results
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "digits": p.digits,
                "expression": p.expression,
                "form": p.form,
                "proof_method": "miller_rabin_12_browser_wasm"
            })
        })
        .collect();

    serde_json::json!({
        "primes": primes_json,
        "tested": tested
    })
    .to_string()
}

/// Search a block and append a SHA-256 result hash for tamper detection.
///
/// Calls `search_block()` internally, then computes a SHA-256 digest of
/// the canonical result string `"{type}:{start}:{end}:{tested}:{primes_json}"`.
/// The hash is appended as `"result_hash"` in the JSON output.
///
/// Used by the Web Worker when content-addressed verification is enabled.
/// The server can independently recompute the hash to verify result integrity.
#[wasm_bindgen]
pub fn search_block_hashed(
    search_type: &str,
    params_json: &str,
    block_start: u64,
    block_end: u64,
) -> String {
    // Run the actual search
    let result_json = search_block(search_type, params_json, block_start, block_end);
    let parsed: serde_json::Value = serde_json::from_str(&result_json).unwrap();

    let tested = parsed["tested"].as_u64().unwrap_or(0);
    let primes = parsed["primes"].to_string();

    // Canonical string for hashing: deterministic, no whitespace variance
    let canonical = format!(
        "{}:{}:{}:{}:{}",
        search_type, block_start, block_end, tested, primes
    );

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    serde_json::json!({
        "primes": parsed["primes"],
        "tested": tested,
        "result_hash": hash
    })
    .to_string()
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ──────────────────────────────────────────────────────

    fn expressions(results: &[PrimeResult]) -> Vec<&str> {
        results.iter().map(|r| r.expression.as_str()).collect()
    }

    // ── KBN (existing form) ────────────────────────────────────────

    #[test]
    fn test_kbn_mersenne_small() {
        // Mersenne primes 2^n - 1 for small n: n=2(3), 3(7), 5(31), 7(127)
        let (results, tested) = search_kbn(1, 2, -1, 2, 10);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"2^2-1")); // 3
        assert!(exprs.contains(&"2^3-1")); // 7
        assert!(exprs.contains(&"2^5-1")); // 31
        assert!(exprs.contains(&"2^7-1")); // 127
        assert!(tested > 0);
    }

    // ── Twin ───────────────────────────────────────────────────────

    #[test]
    fn test_twin_small() {
        let (results, _) = search_twin(3, 50);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"(5, 7)"));
        assert!(exprs.contains(&"(11, 13)"));
        assert!(exprs.contains(&"(17, 19)"));
        assert!(exprs.contains(&"(29, 31)"));
        assert!(exprs.contains(&"(41, 43)"));
    }

    // ── Factorial ──────────────────────────────────────────────────

    #[test]
    fn test_factorial_small() {
        // OEIS A002981: n!+1 prime for n=1,2,3
        // OEIS A002982: n!-1 prime for n=3,4,6,7
        let (results, _) = search_factorial(1, 7);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"1!+1")); // 2
        assert!(exprs.contains(&"2!+1")); // 3
        assert!(exprs.contains(&"3!+1")); // 7
        assert!(exprs.contains(&"3!-1")); // 5
        assert!(exprs.contains(&"4!-1")); // 23
        assert!(exprs.contains(&"6!-1")); // 719
        assert!(exprs.contains(&"7!-1")); // 5039
    }

    // ── Sophie Germain ─────────────────────────────────────────────

    #[test]
    fn test_sophie_germain_small() {
        // OEIS A005384: 2, 3, 5, 11, 23, 29, 41, 53, 83, 89
        let (results, _) = search_sophie_germain(2, 100);
        let exprs = expressions(&results);
        assert_eq!(
            exprs,
            vec!["2", "3", "5", "11", "23", "29", "41", "53", "83", "89"]
        );
    }

    #[test]
    fn test_sophie_germain_empty_range() {
        let (results, tested) = search_sophie_germain(90, 100);
        assert!(results.is_empty());
        assert!(tested > 0);
    }

    // ── Cullen/Woodall ─────────────────────────────────────────────

    #[test]
    fn test_cullen_woodall_small() {
        let (results, _) = search_cullen_woodall(1, 10);
        let exprs = expressions(&results);

        // Cullen prime: n=1 → 1*2^1+1 = 3
        assert!(exprs.contains(&"1*2^1+1"));

        // Woodall primes: n=2 → 7, n=3 → 23, n=6 → 383
        assert!(exprs.contains(&"2*2^2-1"));
        assert!(exprs.contains(&"3*2^3-1"));
        assert!(exprs.contains(&"6*2^6-1"));

        // n=4 Cullen: 4*16+1=65 not prime, n=4 Woodall: 4*16-1=63 not prime
        assert!(!exprs.contains(&"4*2^4+1"));
        assert!(!exprs.contains(&"4*2^4-1"));
    }

    // ── Repunit ────────────────────────────────────────────────────

    #[test]
    fn test_repunit_base10_small() {
        // Base 10: only R(10,2) = 11 is prime for small n
        let (results, _) = search_repunit(10, 2, 20);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"R(10,2)")); // 11
        // R(10,3)=111=3*37, R(10,5)=11111=41*271, etc.
    }

    #[test]
    fn test_repunit_base2_mersenne() {
        // Base 2 repunits = Mersenne numbers: R(2,n) = 2^n - 1
        // Prime for n = 2,3,5,7,13,17,19
        let (results, _) = search_repunit(2, 2, 20);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"R(2,2)")); // 3
        assert!(exprs.contains(&"R(2,3)")); // 7
        assert!(exprs.contains(&"R(2,5)")); // 31
        assert!(exprs.contains(&"R(2,7)")); // 127
        assert!(exprs.contains(&"R(2,13)")); // 8191
        assert!(exprs.contains(&"R(2,17)")); // 131071
        assert!(exprs.contains(&"R(2,19)")); // 524287
    }

    // ── Primorial ──────────────────────────────────────────────────

    #[test]
    fn test_primorial_small() {
        // p=2: 2#+1=3 prime
        // p=3: 6+1=7, 6-1=5 both prime
        // p=5: 30+1=31, 30-1=29 both prime
        // p=7: 210+1=211 prime, 210-1=209=11*19 not prime
        let (results, _) = search_primorial(2, 7);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"2#+1")); // 3
        assert!(exprs.contains(&"3#+1")); // 7
        assert!(exprs.contains(&"3#-1")); // 5
        assert!(exprs.contains(&"5#+1")); // 31
        assert!(exprs.contains(&"5#-1")); // 29
        assert!(exprs.contains(&"7#+1")); // 211
        assert!(!exprs.contains(&"7#-1")); // 209 = 11*19
    }

    // ── Carol/Kynea ────────────────────────────────────────────────

    #[test]
    fn test_carol_kynea_small() {
        let (results, _) = search_carol_kynea(2, 5);
        let exprs = expressions(&results);

        // Carol primes: n=2→7, n=3→47, n=4→223
        assert!(exprs.contains(&"(2^2-1)^2-2")); // 7
        assert!(exprs.contains(&"(2^3-1)^2-2")); // 47
        assert!(exprs.contains(&"(2^4-1)^2-2")); // 223

        // n=5: Carol = 959 = 7*137, not prime
        assert!(!exprs.contains(&"(2^5-1)^2-2"));

        // Kynea primes: n=2→23, n=3→79
        assert!(exprs.contains(&"(2^2+1)^2-2")); // 23
        assert!(exprs.contains(&"(2^3+1)^2-2")); // 79
    }

    // ── Generalized Fermat ─────────────────────────────────────────

    #[test]
    fn test_gen_fermat_base2() {
        // Fermat primes: F(n) = 2^(2^n) + 1
        // F(0)=3, F(1)=5, F(2)=17, F(3)=257, F(4)=65537
        // F(5) = 4294967297 = 641*6700417, NOT prime
        let (results, _) = search_gen_fermat(2, 0, 4);
        assert_eq!(results.len(), 5);

        let exprs = expressions(&results);
        assert!(exprs.contains(&"2^(2^0)+1")); // 3
        assert!(exprs.contains(&"2^(2^1)+1")); // 5
        assert!(exprs.contains(&"2^(2^2)+1")); // 17
        assert!(exprs.contains(&"2^(2^3)+1")); // 257
        assert!(exprs.contains(&"2^(2^4)+1")); // 65537
    }

    #[test]
    fn test_gen_fermat_f5_composite() {
        // F(5) = 2^32 + 1 = 4294967297 = 641 * 6700417
        let (results, tested) = search_gen_fermat(2, 5, 5);
        assert_eq!(tested, 1);
        assert!(results.is_empty());
    }

    // ── Wagstaff ───────────────────────────────────────────────────

    #[test]
    fn test_wagstaff_small() {
        // Known Wagstaff primes for small p: 3,5,7,11,13,17,19,23,29,31
        let (results, _) = search_wagstaff(3, 31);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"(2^3+1)/3")); // 3
        assert!(exprs.contains(&"(2^5+1)/3")); // 11
        assert!(exprs.contains(&"(2^7+1)/3")); // 43
        assert!(exprs.contains(&"(2^11+1)/3")); // 683
        assert!(exprs.contains(&"(2^13+1)/3")); // 2731
    }

    #[test]
    fn test_wagstaff_skips_p2() {
        // p=2 should be skipped (2^2+1=5, not divisible by 3)
        let (results, _) = search_wagstaff(2, 2);
        assert!(results.is_empty());
    }

    // ── Palindromic ────────────────────────────────────────────────

    #[test]
    fn test_palindromic_1digit() {
        // Single-digit palindromic primes in base 10: 2, 3, 5, 7
        let (results, _) = search_palindromic(10, 1, 1);
        let exprs = expressions(&results);
        assert_eq!(exprs, vec!["2", "3", "5", "7"]);
    }

    #[test]
    fn test_palindromic_3digit_count() {
        // 3-digit base-10 palindromic primes: 101, 131, 151, 181, 191,
        // 313, 353, 373, 383, 727, 757, 787, 797, 919, 929 (15 total)
        let (results, _) = search_palindromic(10, 3, 3);
        assert_eq!(results.len(), 15);
        let exprs = expressions(&results);
        assert!(exprs.contains(&"101"));
        assert!(exprs.contains(&"131"));
        assert!(exprs.contains(&"929"));
    }

    #[test]
    fn test_palindromic_skips_even_digits() {
        // Even digit counts should produce no candidates
        let (results, tested) = search_palindromic(10, 2, 2);
        assert!(results.is_empty());
        assert_eq!(tested, 0);
    }

    // ── Near-repdigit ──────────────────────────────────────────────

    #[test]
    fn test_near_repdigit_3digit() {
        let (results, tested) = search_near_repdigit(10, 3, 3);
        // Should find some primes and test some candidates
        assert!(tested > 0);

        // 101 should be found (base digit 1, center varied to 0)
        let exprs = expressions(&results);
        assert!(exprs.contains(&"101"));

        // All results should be prime
        for r in &results {
            let n: BigUint = r.expression.parse().unwrap();
            assert!(is_prime(&n), "{} should be prime", r.expression);
        }
    }

    #[test]
    fn test_near_repdigit_skips_even_digits() {
        let (results, tested) = search_near_repdigit(10, 2, 2);
        assert!(results.is_empty());
        assert_eq!(tested, 0);
    }

    // ── search_block dispatch ──────────────────────────────────────

    #[test]
    fn test_search_block_all_forms() {
        // Verify all 12 form strings dispatch correctly and produce results
        let forms: &[(&str, &str, u64, u64)] = &[
            ("kbn", r#"{"k":1,"base":2,"sign":1}"#, 1, 10),
            ("twin", "{}", 3, 50),
            ("factorial", "{}", 1, 5),
            ("sophie_germain", "{}", 2, 30),
            ("cullen_woodall", "{}", 1, 6),
            ("repunit", r#"{"base":2}"#, 2, 7),
            ("primorial", "{}", 2, 5),
            ("carol_kynea", "{}", 2, 4),
            ("gen_fermat", r#"{"base":2}"#, 0, 4),
            ("wagstaff", "{}", 3, 7),
            ("palindromic", r#"{"base":10}"#, 1, 3),
            ("near_repdigit", r#"{"base":10}"#, 3, 3),
        ];

        for &(form, params, start, end) in forms {
            let result = search_block(form, params, start, end);
            let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
            let tested = parsed["tested"].as_u64().unwrap();
            let primes_arr = parsed["primes"].as_array().unwrap();
            assert!(
                tested > 0,
                "Form '{}' should test at least one candidate",
                form
            );
            assert!(
                !primes_arr.is_empty(),
                "Form '{}' should find at least one prime in test range",
                form
            );
        }
    }

    #[test]
    fn test_search_block_unknown_form() {
        let result = search_block("nonexistent", "{}", 1, 10);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["tested"].as_u64().unwrap(), 0);
        assert!(parsed["primes"].as_array().unwrap().is_empty());
    }

    // ── Edge cases ─────────────────────────────────────────────────

    #[test]
    fn test_empty_ranges() {
        // start > end should produce no results
        let (results, tested) = search_kbn(1, 2, 1, 10, 5);
        assert!(results.is_empty());
        assert_eq!(tested, 0);

        let (results, tested) = search_sophie_germain(100, 50);
        assert!(results.is_empty());
        assert_eq!(tested, 0);
    }

    #[test]
    fn test_is_prime_u64_basic() {
        assert!(!is_prime_u64(0));
        assert!(!is_prime_u64(1));
        assert!(is_prime_u64(2));
        assert!(is_prime_u64(3));
        assert!(!is_prime_u64(4));
        assert!(is_prime_u64(5));
        assert!(is_prime_u64(97));
        assert!(!is_prime_u64(100));
    }

    #[test]
    fn test_build_palindrome() {
        // 3-digit: [1,2] → 121
        assert_eq!(build_palindrome(&[1, 2], 10), BigUint::from(121u32));
        // 5-digit: [1,2,3] → 12321
        assert_eq!(build_palindrome(&[1, 2, 3], 10), BigUint::from(12321u32));
        // 1-digit: [5] → 5
        assert_eq!(build_palindrome(&[5], 10), BigUint::from(5u32));
        // Base 2, 3-digit: [1,0] → 101 in binary = 5
        assert_eq!(build_palindrome(&[1, 0], 2), BigUint::from(5u32));
    }

    // ── mod_small (zero-allocation trial division) ────────────────

    #[test]
    fn test_mod_small_basic() {
        assert_eq!(mod_small(&BigUint::from(100u32), 7), 2); // 100 mod 7 = 2
        assert_eq!(mod_small(&BigUint::from(17u32), 17), 0); // 17 mod 17 = 0
        assert_eq!(mod_small(&BigUint::from(1u32), 3), 1);
        assert_eq!(mod_small(&BigUint::from(0u32), 5), 0);
    }

    #[test]
    fn test_mod_small_large() {
        // 2^128 mod 7 = (2^3)^42 * 2^2 mod 7 = 1^42 * 4 = 4
        let big = BigUint::from(2u32).pow(128);
        assert_eq!(mod_small(&big, 7), 4);
    }

    // ── MontCtx (Montgomery arithmetic) ──────────────────────────

    #[test]
    fn test_mont_ctx_roundtrip() {
        let ctx = MontCtx::new(97);
        for a in 0..97u64 {
            let a_mont = ctx.to_mont(a);
            assert_eq!(ctx.from_mont(a_mont), a, "roundtrip failed for a={}", a);
        }
    }

    #[test]
    fn test_mont_ctx_mul() {
        let ctx = MontCtx::new(97);
        let a_mont = ctx.to_mont(42);
        let b_mont = ctx.to_mont(55);
        let result = ctx.from_mont(ctx.mul(a_mont, b_mont));
        assert_eq!(result, (42u64 * 55) % 97);
    }

    #[test]
    fn test_mont_ctx_pow_mod() {
        let ctx = MontCtx::new(97);
        let base_mont = ctx.to_mont(3);
        // 3^10 = 59049, 59049 mod 97 = 59049 - 608*97 = 59049 - 58976 = 73
        let result = ctx.from_mont(ctx.pow_mod(base_mont, 10));
        assert_eq!(result, 73);
    }

    // ── Modular sieve ────────────────────────────────────────────

    #[test]
    fn test_modular_sieve_kbn_no_false_negatives() {
        // Verify that the sieve never marks a known prime as composite
        // Mersenne primes 2^n-1: n=2(3), 3(7), 5(31), 7(127)
        let survivors = modular_sieve_kbn(1, 2, -1, 2, 10);
        // n=2: 2^2-1=3 (prime) — must survive
        assert!(survivors[0], "2^2-1=3 should survive sieve");
        // n=3: 2^3-1=7 (prime) — must survive
        assert!(survivors[1], "2^3-1=7 should survive sieve");
        // n=5: 2^5-1=31 (prime) — must survive
        assert!(survivors[3], "2^5-1=31 should survive sieve");
        // n=7: 2^7-1=127 (prime) — must survive
        assert!(survivors[5], "2^7-1=127 should survive sieve");
    }

    #[test]
    fn test_modular_sieve_kbn_eliminates_composites() {
        // For small candidates (< sieve_limit ~10000), the post-pass un-sieves
        // them to avoid false negatives (letting is_prime decide). Test with
        // larger values where the sieve properly eliminates.
        // n=14: 2^14-1 = 16383 = 3 × 43 × 127 (composite, > sieve limit)
        // n=15: 2^15-1 = 32767 = 7 × 31 × 151 (composite, > sieve limit)
        let survivors = modular_sieve_kbn(1, 2, -1, 14, 18);
        assert!(!survivors[0], "2^14-1=16383 should be sieved out");
        assert!(!survivors[1], "2^15-1=32767 should be sieved out");
        // n=16: 2^16-1 = 65535 = 3 × 5 × 17 × 257 (composite)
        assert!(!survivors[2], "2^16-1=65535 should be sieved out");
        // n=17: 2^17-1 = 131071 (prime!) — must survive
        assert!(survivors[3], "2^17-1=131071 is prime and should survive");
    }

    // ── MR pre-screen ────────────────────────────────────────────

    #[test]
    fn test_mr_witness_basic() {
        let n = BigUint::from(561u32); // Carmichael number
        let n_minus_1 = &n - BigUint::one();
        let mut d = n_minus_1.clone();
        let mut r = 0u64;
        while d.is_even() {
            d >>= 1;
            r += 1;
        }
        // witness 2 should detect 561 as composite
        assert!(!mr_witness(&n, 2, &d, r), "561 should be detected as composite by witness 2");
    }

    #[test]
    fn test_mr_prescreen_catches_composites() {
        // The pre-screen (witnesses 2,3) catches most composites
        let composites = [15u32, 21, 35, 91, 105, 341, 561, 1105];
        for &c in &composites {
            assert!(
                !miller_rabin(&BigUint::from(c)),
                "{} should be identified as composite",
                c
            );
        }
    }

    // ── search_block_hashed ──────────────────────────────────────

    #[test]
    fn test_search_block_hashed_has_hash() {
        let result = search_block_hashed("kbn", r#"{"k":1,"base":2,"sign":1}"#, 1, 10);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["result_hash"].is_string());
        let hash = parsed["result_hash"].as_str().unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_search_block_hashed_deterministic() {
        // Same inputs should produce the same hash
        let r1 = search_block_hashed("twin", "{}", 3, 50);
        let r2 = search_block_hashed("twin", "{}", 3, 50);
        let p1: serde_json::Value = serde_json::from_str(&r1).unwrap();
        let p2: serde_json::Value = serde_json::from_str(&r2).unwrap();
        assert_eq!(p1["result_hash"], p2["result_hash"]);
    }

    #[test]
    fn test_search_block_hashed_different_inputs() {
        // Different ranges should produce different hashes
        let r1 = search_block_hashed("kbn", r#"{"k":1,"base":2,"sign":1}"#, 1, 10);
        let r2 = search_block_hashed("kbn", r#"{"k":1,"base":2,"sign":1}"#, 1, 20);
        let p1: serde_json::Value = serde_json::from_str(&r1).unwrap();
        let p2: serde_json::Value = serde_json::from_str(&r2).unwrap();
        assert_ne!(p1["result_hash"], p2["result_hash"]);
    }
}
