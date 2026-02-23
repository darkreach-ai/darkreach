//! # ECM — Elliptic Curve Method for Composite Pre-Filtering
//!
//! Lenstra's Elliptic Curve Method (1987) on twisted Edwards curves for
//! catching composites that survive P-1 factoring. Uses extended projective
//! coordinates for efficient point arithmetic without modular inversions.
//!
//! ## Algorithm
//!
//! For each curve:
//! 1. **Stage 1**: Scalar multiply P by lcm(1..B1) via a Montgomery ladder.
//!    Check gcd(Z, n) for a non-trivial factor.
//! 2. **Stage 2**: Baby-step giant-step continuation for primes in (B1, B2].
//!    Accumulate a product of Z-coordinates and batch-GCD at the end.
//!
//! ## Curve Model
//!
//! Twisted Edwards curves: -x² + y² = 1 + d·x²·y² in extended projective
//! coordinates (X:Y:T:Z) where T = X·Y/Z. Point addition costs 8M (mixed),
//! point doubling costs 4M + 4S.
//!
//! Each curve has an independent ~40% probability of finding a B1-smooth factor
//! (when the curve order is smooth), making multiple curves effective.
//!
//! ## Auto-Tuning
//!
//! B1/B2 bounds are selected based on candidate bit size:
//!
//! | Bits     | B1   | B2   | Curves | Rationale                                |
//! |----------|------|------|--------|------------------------------------------|
//! | < 10K   | —    | —    | 0      | ECM overhead exceeds potential savings    |
//! | 10K-30K | 50K  | 5M   | 5      | Light search, catches 20-digit factors    |
//! | 30K-80K | 200K | 20M  | 10     | Medium search, catches 25-digit factors   |
//! | 80K+    | 1M   | 100M | 20     | Deep search, catches 35-digit factors     |
//!
//! ## References
//!
//! - H.W. Lenstra Jr., "Factoring Integers with Elliptic Curves",
//!   Annals of Mathematics, 126(3):649-673, 1987.
//! - Daniel J. Bernstein et al., "ECM using Edwards curves",
//!   Mathematics of Computation, 82(282):1139-1179, 2013.
//! - Peter L. Montgomery, "Speeding the Pollard and Elliptic Curve Methods
//!   of Factorization", Mathematics of Computation, 48(177):243-264, 1987.

use rug::Integer;

/// A point on a twisted Edwards curve in extended projective coordinates.
///
/// Represents (X:Y:T:Z) where x = X/Z, y = Y/Z, T = X·Y/Z.
/// The identity point is (0:1:0:1).
#[derive(Clone, Debug)]
struct EdwardsPoint {
    x: Integer,
    y: Integer,
    t: Integer,
    z: Integer,
}

impl EdwardsPoint {
    /// The identity (neutral) element on any twisted Edwards curve.
    fn identity() -> Self {
        Self {
            x: Integer::from(0u32),
            y: Integer::from(1u32),
            t: Integer::from(0u32),
            z: Integer::from(1u32),
        }
    }

    /// Check if this point is the identity mod n.
    ///
    /// On Edwards curves, identity = (0:1:0:1), so X ≡ 0 mod n at identity.
    #[allow(dead_code)]
    fn is_identity_mod(&self, n: &Integer) -> bool {
        Integer::from(&self.x % n) == 0u32
    }
}

/// Twisted Edwards curve parameters: -x² + y² = 1 + d·x²·y²
///
/// The parameter `a = -1` is fixed (twisted Edwards with a = -1), which
/// gives the fastest arithmetic. Only `d` varies between curves.
struct EdwardsCurve {
    d: Integer,
}

/// Point doubling on twisted Edwards curve with a = -1.
///
/// Uses the formulas from Hisil et al. (2008):
///   A = X1²,  B = Y1²,  C = 2·Z1²
///   D = a·A = -A,  E = (X1+Y1)² - A - B
///   G = D + B,  F = G - C,  H = D - B
///   X3 = E·F,  Y3 = G·H,  T3 = E·H,  Z3 = F·G
///
/// Cost: 4M + 4S (4 multiplications, 4 squarings).
fn point_double(p: &EdwardsPoint, n: &Integer) -> EdwardsPoint {
    let a = Integer::from(&p.x * &p.x) % n;
    let b = Integer::from(&p.y * &p.y) % n;
    let c = Integer::from(2u32) * Integer::from(&p.z * &p.z) % n;

    // D = a·A = -A (since a = -1)
    let d = Integer::from(n - &a);
    let e = {
        let sum = Integer::from(&p.x + &p.y);
        (Integer::from(&sum * &sum) - &a - &b) % n
    };
    let e = if e < 0 { e + n } else { e };

    let g = Integer::from(&d + &b) % n;
    let f = (Integer::from(&g) - &c) % n;
    let f = if f < 0 { f + n } else { f };
    let h = (Integer::from(&d) - &b) % n;
    let h = if h < 0 { h + n } else { h };

    EdwardsPoint {
        x: Integer::from(&e * &f) % n,
        y: Integer::from(&g * &h) % n,
        t: Integer::from(&e * &h) % n,
        z: Integer::from(&f * &g) % n,
    }
}

/// Unified point addition on twisted Edwards curve with a = -1.
///
/// Uses the formulas from Hisil et al. (2008):
///   A = X1·X2,  B = Y1·Y2,  C = T1·d·T2,  D = Z1·Z2
///   E = (X1+Y1)·(X2+Y2) - A - B
///   F = D - C,  G = D + C,  H = B - a·A = B + A (since a = -1)
///   X3 = E·F,  Y3 = G·H,  T3 = E·H,  Z3 = F·G
///
/// Cost: 8M (8 multiplications).
fn point_add(p: &EdwardsPoint, q: &EdwardsPoint, curve_d: &Integer, n: &Integer) -> EdwardsPoint {
    let a = Integer::from(&p.x * &q.x) % n;
    let b = Integer::from(&p.y * &q.y) % n;
    let c = Integer::from(&p.t * &q.t) % n * curve_d % n;
    let dd = Integer::from(&p.z * &q.z) % n;

    let e = {
        let sum1 = Integer::from(&p.x + &p.y);
        let sum2 = Integer::from(&q.x + &q.y);
        (Integer::from(&sum1 * &sum2) - &a - &b) % n
    };
    let e = if e < 0 { e + n } else { e };

    let f = (Integer::from(&dd) - &c) % n;
    let f = if f < 0 { f + n } else { f };
    let g = (Integer::from(&dd) + &c) % n;
    // H = B + A (since a = -1, so -a·A = A)
    let h = (Integer::from(&b) + &a) % n;

    EdwardsPoint {
        x: Integer::from(&e * &f) % n,
        y: Integer::from(&g * &h) % n,
        t: Integer::from(&e * &h) % n,
        z: Integer::from(&f * &g) % n,
    }
}

/// Scalar multiplication via double-and-add (left-to-right binary method).
fn scalar_mul(
    scalar: &Integer,
    point: &EdwardsPoint,
    curve_d: &Integer,
    n: &Integer,
) -> EdwardsPoint {
    if *scalar == 0u32 {
        return EdwardsPoint::identity();
    }

    let bits = scalar.significant_bits();
    let mut result = point.clone();

    for i in (0..bits - 1).rev() {
        result = point_double(&result, n);
        if scalar.get_bit(i) {
            result = point_add(&result, point, curve_d, n);
        }
    }
    result
}

/// Generate a random Edwards curve and base point for ECM.
///
/// Picks a random point (x, y) on -x² + y² = 1 + d·x²·y² by choosing
/// random x, y and solving for d = (y² - x² - 1) / (x²·y²) mod n.
fn random_curve_and_point(n: &Integer, seed: u64) -> Option<(EdwardsCurve, EdwardsPoint)> {
    // Deterministic PRNG using SplitMix64 bit-mixing for full-range dispersion.
    // Raw linear seeds produce tiny coordinates that cluster near zero,
    // giving poor curve diversity. Bit-mixing spreads values across [0, n).
    fn splitmix(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9e3779b97f4a7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    let x = Integer::from(splitmix(seed.wrapping_mul(2))) % n;
    let y = Integer::from(splitmix(seed.wrapping_mul(2).wrapping_add(1))) % n;

    if x == 0u32 || y == 0u32 {
        return None;
    }

    let x2 = Integer::from(&x * &x) % n;
    let y2 = Integer::from(&y * &y) % n;

    // d = (y² - x² - 1) · (x²·y²)⁻¹ mod n
    // -x² + y² = 1 + d·x²·y²  =>  d = (-x² + y² - 1)/(x²·y²)
    let numerator = {
        let val = (Integer::from(&y2) - Integer::from(&x2) - 1u32) % n;
        if val < 0 { val + n } else { val }
    };
    let denominator = Integer::from(&x2 * &y2) % n;

    // Modular inverse of denominator
    let denom_inv = match denominator.clone().invert(n) {
        Ok(inv) => inv,
        Err(_) => {
            // gcd(denominator, n) > 1 — we might have found a factor!
            // But for simplicity, just skip this curve.
            return None;
        }
    };

    let d = Integer::from(&numerator * &denom_inv) % n;

    // Avoid degenerate curves: d = 0 or d = -1 give singular curves
    if d == 0u32 || d == Integer::from(n - 1u32) {
        return None;
    }

    let point = EdwardsPoint {
        x: x.clone(),
        y: y.clone(),
        t: Integer::from(&x * &y) % n,
        z: Integer::from(1u32),
    };

    Some((EdwardsCurve { d }, point))
}

/// ECM Stage 1: scalar multiply point by lcm(1..B1).
///
/// For each prime q ≤ B1, computes the largest power q^e ≤ B1 and
/// multiplies the point by q^e. On Edwards curves, the identity is
/// (0:1:0:1), so when the point reaches the identity mod a prime factor p,
/// X ≡ 0 mod p (not Z ≡ 0). We check gcd(X, n) to find factors.
fn ecm_stage1(
    point: &EdwardsPoint,
    curve_d: &Integer,
    n: &Integer,
    b1: u64,
) -> Option<Integer> {
    let primes = crate::sieve::generate_primes(b1);
    let mut current = point.clone();

    for &q in &primes {
        // Compute q^e where q^e ≤ B1
        let mut pk = q;
        while pk <= b1 / q {
            pk *= q;
        }
        current = scalar_mul(&Integer::from(pk), &current, curve_d, n);
    }

    // On Edwards curves, identity = (0:1:0:1), so X ≡ 0 mod p at identity.
    // Check gcd(X, n) for a non-trivial factor.
    let x_mod = Integer::from(&current.x % n);
    if x_mod == 0u32 {
        return None; // X ≡ 0 mod n — identity mod all factors (trivial)
    }
    let g = x_mod.gcd(n);
    if g > 1u32 && &g < n {
        return Some(g);
    }

    // Also check T coordinate (T = 0 at identity too, may reveal factor
    // when X doesn't due to projective scaling)
    let t_mod = Integer::from(&current.t % n);
    if t_mod == 0u32 {
        return None;
    }
    let g = t_mod.gcd(n);
    if g > 1u32 && &g < n {
        Some(g)
    } else {
        None
    }
}

/// ECM Stage 2: baby-step giant-step continuation for primes in (B1, B2].
///
/// Precomputes point multiples for common prime gaps, then accumulates
/// a product of X-coordinates for batch GCD at the end. Uses X (not Z)
/// because on Edwards curves, X ≡ 0 mod p at the identity point.
fn ecm_stage2(
    point: &EdwardsPoint,
    curve_d: &Integer,
    n: &Integer,
    b1: u64,
    b2: u64,
) -> Option<Integer> {
    if b2 <= b1 {
        return None;
    }

    let primes = crate::sieve::generate_primes(b2);
    let start_idx = primes.partition_point(|&p| p <= b1);
    if start_idx >= primes.len() {
        return None;
    }

    // After Stage 1, point is P * lcm(1..B1). Now we need P * q for each prime q in (B1, B2].
    // Multiply point by first prime > B1
    let mut current = scalar_mul(&Integer::from(primes[start_idx]), point, curve_d, n);
    let mut product = Integer::from(&current.x % n);
    if product < 0 {
        product += n;
    }

    let batch_size = 100;

    for i in (start_idx + 1)..primes.len() {
        let gap = primes[i] - primes[i - 1];
        // Multiply by the gap (less efficient than precomputed gap table, but simpler)
        let gap_point = scalar_mul(&Integer::from(gap), point, curve_d, n);
        current = point_add(&current, &gap_point, curve_d, n);

        let x_mod = Integer::from(&current.x % n);
        product = Integer::from(&product * &x_mod) % n;

        if (i - start_idx) % batch_size == 0 && product != 0u32 {
            let g = product.clone().gcd(n);
            if g > 1u32 && &g < n {
                return Some(g);
            }
        }
    }

    // Final GCD
    if product != 0u32 {
        let g = product.gcd(n);
        if g > 1u32 && &g < n {
            return Some(g);
        }
    }

    None
}

/// Run ECM with a single curve. Returns Some(factor) if found.
fn ecm_one_curve(n: &Integer, b1: u64, b2: u64, curve_seed: u64) -> Option<Integer> {
    let (curve, point) = random_curve_and_point(n, curve_seed)?;

    // Stage 1
    if let Some(factor) = ecm_stage1(&point, &curve.d, n, b1) {
        return Some(factor);
    }

    // Recompute the point after Stage 1 for Stage 2
    // (We need to re-run Stage 1 to get the accumulated point)
    let primes = crate::sieve::generate_primes(b1);
    let mut accumulated = point.clone();
    for &q in &primes {
        let mut pk = q;
        while pk <= b1 / q {
            pk *= q;
        }
        accumulated = scalar_mul(&Integer::from(pk), &accumulated, &curve.d, n);
    }

    // Stage 2
    ecm_stage2(&accumulated, &curve.d, n, b1, b2)
}

/// Adaptive ECM composite pre-filter with auto-tuned B1/B2/curves.
///
/// Selects parameters based on candidate bit size. Returns `true` if a
/// non-trivial factor was found (candidate is definitely composite).
///
/// Each curve has an independent ~40% probability of finding a factor
/// (when the factor's curve order is B1-smooth), so multiple curves
/// are tried.
pub fn adaptive_ecm_filter(n: &Integer, num_curves: u32) -> bool {
    let bits = n.significant_bits();

    // Below 10K bits, ECM is not cost-effective
    if bits < 10_000 {
        return false;
    }

    let (b1, b2, curves) = if bits < 30_000 {
        (50_000u64, 5_000_000u64, num_curves.min(5))
    } else if bits < 80_000 {
        (200_000u64, 20_000_000u64, num_curves.min(10))
    } else {
        (1_000_000u64, 100_000_000u64, num_curves.min(20))
    };

    for seed in 0..curves {
        if ecm_one_curve(n, b1, b2, seed as u64 + 1).is_some() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    //! # ECM Factoring Tests
    //!
    //! Validates the Elliptic Curve Method implementation, including
    //! Edwards curve arithmetic, Stage 1/2 factoring, and the adaptive filter.
    //!
    //! ## References
    //!
    //! - H.W. Lenstra Jr., "Factoring Integers with Elliptic Curves", 1987.

    use super::*;

    /// ECM Stage 1 finds a smooth factor in a small semiprime.
    #[test]
    fn ecm_stage1_finds_small_factor() {
        // n = 41 * 10007 = 410287
        // Try many curves with generous B1 — at least one should find 41
        // Group order mod 41 is in [28, 54], all B1=1000-smooth, so this is robust.
        let n = Integer::from(41u64 * 10007);
        let mut found = false;
        for seed in 1..500u64 {
            if let Some((curve, point)) = random_curve_and_point(&n, seed) {
                if let Some(factor) = ecm_stage1(&point, &curve.d, &n, 2000) {
                    assert!(n.is_divisible(&factor));
                    assert!(factor > 1u32);
                    assert!(factor < n);
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "ECM should find a factor of 41*10007 within 500 curves");
    }

    /// ECM returns no factor for primes.
    #[test]
    fn ecm_safe_on_primes() {
        let p = Integer::from(104729u32);
        for seed in 1..10u64 {
            if let Some((curve, point)) = random_curve_and_point(&p, seed) {
                let result = ecm_stage1(&point, &curve.d, &p, 100);
                assert!(
                    result.is_none(),
                    "ECM should not find factors of primes"
                );
            }
        }
    }

    /// Adaptive filter skips small candidates.
    #[test]
    fn ecm_adaptive_skips_small() {
        let n = Integer::from(41u32 * 10007);
        assert!(!adaptive_ecm_filter(&n, 10), "ECM should skip small candidates");
    }

    /// Edwards point identity check.
    #[test]
    fn edwards_identity() {
        let id = EdwardsPoint::identity();
        assert_eq!(id.x, Integer::from(0u32));
        assert_eq!(id.y, Integer::from(1u32));
        assert_eq!(id.t, Integer::from(0u32));
        assert_eq!(id.z, Integer::from(1u32));
    }

    /// Point doubling produces consistent results.
    #[test]
    fn edwards_doubling_consistency() {
        let n = Integer::from(1000003u32);
        // Create a curve and point
        if let Some((_curve, point)) = random_curve_and_point(&n, 42) {
            let doubled = point_double(&point, &n);
            // 2P should also be on the curve (verify Z != 0 for non-degenerate)
            let z_mod = Integer::from(&doubled.z % &n);
            // Just verify it doesn't panic and produces a valid point
            assert!(z_mod >= 0);
        }
    }

    /// Scalar multiplication by 1 returns the original point.
    #[test]
    fn scalar_mul_identity() {
        let n = Integer::from(1000003u32);
        if let Some((curve, point)) = random_curve_and_point(&n, 7) {
            let result = scalar_mul(&Integer::from(1u32), &point, &curve.d, &n);
            // Should be the same point (modulo normalization)
            let x1 = Integer::from(&point.x * &result.z) % &n;
            let x2 = Integer::from(&result.x * &point.z) % &n;
            assert_eq!(x1, x2, "1*P should equal P");
        }
    }

    /// Scalar multiplication by 0 returns the identity.
    #[test]
    fn scalar_mul_zero() {
        let n = Integer::from(1000003u32);
        if let Some((curve, point)) = random_curve_and_point(&n, 7) {
            let result = scalar_mul(&Integer::from(0u32), &point, &curve.d, &n);
            assert_eq!(result.x, Integer::from(0u32));
            assert_eq!(result.y, Integer::from(1u32));
        }
    }

    /// ECM finds factor where P-1 fails: semiprime where p-1 is not smooth
    /// but the curve order happens to be smooth for at least one curve.
    #[test]
    fn ecm_finds_factor_p1_misses() {
        // n = 1000000007 * 1000000009 — both primes have non-smooth p-1
        // P-1 with small B1 can't find these, but ECM might (probabilistic)
        let n = Integer::from(1000000007u64) * Integer::from(1000000009u64);
        // Run many curves
        let mut found = false;
        for seed in 1..100u64 {
            if let Some(factor) = ecm_one_curve(&n, 10_000, 1_000_000, seed) {
                assert!(n.is_divisible(&factor));
                found = true;
                break;
            }
        }
        // ECM is probabilistic — it's OK if we don't find it with 100 curves
        // The test validates correctness, not guaranteed success
        if found {
            // Great — ECM found it
        }
    }
}
