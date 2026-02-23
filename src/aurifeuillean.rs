//! # Aurifeuillean Factorizations — Algebraic Factor Detection
//!
//! Detects candidates with guaranteed algebraic (Aurifeuillean) factorizations,
//! allowing search forms to skip provably composite numbers without any
//! primality testing.
//!
//! ## Background
//!
//! An Aurifeuillean factorization occurs when a number of the form b^n ± 1
//! admits a non-trivial algebraic factorization beyond the obvious cyclotomic
//! factors. These factorizations were discovered by Léon-François-Antoine
//! Aurifeuille in 1871.
//!
//! ## Base 2 (Sophie Germain Identity)
//!
//! For exponents of the form 4k-2 (i.e., n ≡ 2 mod 4):
//!   2^(4k-2) + 1 = (2^(2k-1) - 2^k + 1)(2^(2k-1) + 2^k + 1)
//!
//! This is a special case of the Sophie Germain identity:
//!   a⁴ + 4b⁴ = (a² + 2b² + 2ab)(a² + 2b² - 2ab)
//!
//! ## General Aurifeuillean
//!
//! For base b = s²·t where t is square-free:
//! - If t ≡ 1 (mod 4) and n ≡ t (mod 2t): b^n + 1 has an Aurifeuillean factorization.
//! - If t ≡ 3 (mod 4) and n ≡ t (mod 2t): b^n - 1 has an Aurifeuillean factorization.
//!
//! ## References
//!
//! - Léon-François-Antoine Aurifeuille, 1871.
//! - Richard Brent, "Factoring Aurifeuillean Numbers", 1996.
//! - Samuel S. Wagstaff Jr., "The Cunningham Project", 2013.
//! - OEIS [A092559](https://oeis.org/A092559): Aurifeuillean factorizations of 2^n+1.

use rug::Integer;

/// Decompose base into s²·t where t is square-free.
///
/// Returns (s, t) such that base = s² × t and t has no square factor > 1.
fn square_free_decomposition(base: u64) -> (u64, u64) {
    let mut s = 1u64;
    let mut t = base;

    let mut d = 2u64;
    while d * d <= t {
        while t.is_multiple_of(d * d) {
            s *= d;
            t /= d * d;
        }
        d += 1;
    }

    (s, t)
}

/// Check if a generalized Fermat number b^(2^n) + 1 has an Aurifeuillean factorization.
///
/// For base b = s²·t (t square-free), an Aurifeuillean factorization of b^m + 1
/// exists when:
/// - t ≡ 1 (mod 4), and the exponent m satisfies certain congruence conditions
///
/// For the specific case of generalized Fermat numbers (m = 2^n), this applies when
/// the total exponent 2^n satisfies 2^n ≡ t (mod 2t).
///
/// # Base 2 special case
///
/// When base = 2 (s=1, t=2), and the exponent 2^n ≡ 2 (mod 4), i.e., n = 1:
///   2^2 + 1 = 5 (prime, too small for the factorization to be non-trivial)
///
/// For larger forms like repunits and gen_fermat, this check is more useful.
pub fn has_aurifeuillean_factorization(base: u64, exponent: u64) -> bool {
    if base <= 1 || exponent == 0 {
        return false;
    }

    let (_s, t) = square_free_decomposition(base);

    // If t = 1, base is a perfect square — no Aurifeuillean factorization for b^n+1
    // (perfect square bases have other algebraic factorizations, but not Aurifeuillean)
    if t == 1 {
        return false;
    }

    if t % 4 == 1 {
        // t ≡ 1 (mod 4): b^n + 1 has Aurifeuillean factorization when n ≡ t (mod 2t)
        exponent % (2 * t) == t
    } else if t % 4 == 3 {
        // t ≡ 3 (mod 4): b^n - 1 has Aurifeuillean factorization when n ≡ t (mod 2t)
        // This applies to repunit-like forms (b^n - 1)/(b - 1)
        exponent % (2 * t) == t
    } else {
        // t ≡ 2 (mod 4): use Sophie Germain identity for base 2
        // 2^n + 1 factors when n ≡ 2 (mod 4), i.e., n = 4k - 2
        if base == 2 && exponent >= 6 && exponent % 4 == 2 {
            return true;
        }
        // General even t: check if 2^(n/2) ≡ ±1 patterns apply
        false
    }
}

/// Compute the Aurifeuillean L and M factors of 2^(4k-2) + 1.
///
/// Uses the Sophie Germain identity:
///   2^(4k-2) + 1 = L × M
/// where:
///   L = 2^(2k-1) - 2^k + 1
///   M = 2^(2k-1) + 2^k + 1
///
/// Returns `Some((L, M))` if the exponent has the form 4k-2 with k ≥ 2,
/// or `None` if the factorization doesn't apply.
pub fn aurifeuillean_factors_base2(exponent: u64) -> Option<(Integer, Integer)> {
    // exponent must be of the form 4k - 2, i.e., exponent ≡ 2 (mod 4)
    if exponent < 6 || exponent % 4 != 2 {
        return None;
    }

    let k = (exponent + 2) / 4;

    // L = 2^(2k-1) - 2^k + 1
    let power_2k_minus_1 = Integer::from(1u32) << (2 * k - 1) as u32;
    let power_k = Integer::from(1u32) << k as u32;
    let l = Integer::from(&power_2k_minus_1 - &power_k) + 1u32;

    // M = 2^(2k-1) + 2^k + 1
    let m = Integer::from(&power_2k_minus_1 + &power_k) + 1u32;

    Some((l, m))
}

/// General Aurifeuillean factor computation for b^n + 1.
///
/// For base b = s²·t (t square-free, t ≡ 1 mod 4) and n ≡ t (mod 2t),
/// the number b^n + 1 splits into two factors via the Aurifeuillean identity.
///
/// Currently only implements the base-2 case via [`aurifeuillean_factors_base2`].
/// General bases require computing the Aurifeuillean polynomial, which is
/// base-dependent and complex.
pub fn aurifeuillean_factors(base: u64, exponent: u64) -> Option<(Integer, Integer)> {
    if base == 2 {
        return aurifeuillean_factors_base2(exponent);
    }

    // General case: not yet implemented (requires computing Aurifeuillean polynomials
    // specific to each base, which involves cyclotomic polynomial evaluation).
    None
}

#[cfg(test)]
mod tests {
    //! # Aurifeuillean Factorization Tests
    //!
    //! Validates the detection and computation of Aurifeuillean factorizations.
    //!
    //! ## Base 2 Test Cases
    //!
    //! The Sophie Germain identity gives:
    //!   2^6 + 1 = 65 = 5 × 13 (k=2)
    //!   2^10 + 1 = 1025 = 25 × 41 (k=3)
    //!   2^14 + 1 = 16385 = 113 × 145... wait: 16385 = 5 × 29 × 113
    //!
    //! Actually the L,M factors are:
    //!   k=2: L = 2^3 - 2^2 + 1 = 5,  M = 2^3 + 2^2 + 1 = 13  → 5 × 13 = 65 = 2^6 + 1 ✓
    //!   k=3: L = 2^5 - 2^3 + 1 = 25, M = 2^5 + 2^3 + 1 = 41  → 25 × 41 = 1025 = 2^10 + 1 ✓
    //!   k=4: L = 2^7 - 2^4 + 1 = 113, M = 2^7 + 2^4 + 1 = 145 → 113 × 145 = 16385 = 2^14 + 1 ✓
    //!
    //! ## References
    //!
    //! - OEIS [A092559](https://oeis.org/A092559): Aurifeuillean factorizations of 2^n+1.

    use super::*;
    use rug::ops::Pow;

    /// Square-free decomposition: 12 = 2² × 3, so s=2, t=3.
    #[test]
    fn square_free_decomposition_basic() {
        assert_eq!(square_free_decomposition(1), (1, 1));
        assert_eq!(square_free_decomposition(2), (1, 2));
        assert_eq!(square_free_decomposition(3), (1, 3));
        assert_eq!(square_free_decomposition(4), (2, 1)); // 2² × 1
        assert_eq!(square_free_decomposition(12), (2, 3)); // 2² × 3
        assert_eq!(square_free_decomposition(18), (3, 2)); // 3² × 2
        assert_eq!(square_free_decomposition(50), (5, 2)); // 5² × 2
        assert_eq!(square_free_decomposition(72), (6, 2)); // 6² × 2
    }

    /// Base 2, exponent 6: 2^6 + 1 = 65 = 5 × 13.
    /// L = 2^3 - 2^2 + 1 = 5, M = 2^3 + 2^2 + 1 = 13.
    #[test]
    fn aurifeuillean_base2_k2() {
        let (l, m) = aurifeuillean_factors_base2(6).unwrap();
        assert_eq!(l, Integer::from(5u32));
        assert_eq!(m, Integer::from(13u32));
        assert_eq!(
            Integer::from(&l * &m),
            Integer::from(2u32).pow(6) + 1u32
        );
    }

    /// Base 2, exponent 10: 2^10 + 1 = 1025 = 25 × 41.
    #[test]
    fn aurifeuillean_base2_k3() {
        let (l, m) = aurifeuillean_factors_base2(10).unwrap();
        assert_eq!(l, Integer::from(25u32));
        assert_eq!(m, Integer::from(41u32));
        assert_eq!(
            Integer::from(&l * &m),
            Integer::from(2u32).pow(10) + 1u32
        );
    }

    /// Base 2, exponent 14: 2^14 + 1 = 16385 = 113 × 145.
    #[test]
    fn aurifeuillean_base2_k4() {
        let (l, m) = aurifeuillean_factors_base2(14).unwrap();
        assert_eq!(l, Integer::from(113u32));
        assert_eq!(m, Integer::from(145u32));
        assert_eq!(
            Integer::from(&l * &m),
            Integer::from(2u32).pow(14) + 1u32
        );
    }

    /// L × M = 2^n + 1 for several exponents.
    #[test]
    fn aurifeuillean_base2_product_identity() {
        for &n in &[6u64, 10, 14, 18, 22, 26, 30] {
            let (l, m) = aurifeuillean_factors_base2(n).unwrap();
            let expected = Integer::from(2u32).pow(n as u32) + 1u32;
            assert_eq!(
                Integer::from(&l * &m),
                expected,
                "L*M != 2^{} + 1",
                n
            );
        }
    }

    /// Exponents not of the form 4k-2 should return None.
    #[test]
    fn aurifeuillean_base2_invalid_exponents() {
        assert!(aurifeuillean_factors_base2(0).is_none());
        assert!(aurifeuillean_factors_base2(1).is_none());
        assert!(aurifeuillean_factors_base2(2).is_none()); // too small (k=1)
        assert!(aurifeuillean_factors_base2(3).is_none()); // not 4k-2
        assert!(aurifeuillean_factors_base2(4).is_none()); // not 4k-2
        assert!(aurifeuillean_factors_base2(5).is_none());
        assert!(aurifeuillean_factors_base2(7).is_none());
        assert!(aurifeuillean_factors_base2(8).is_none());
    }

    /// Detection: base 2 with 4k-2 exponents should be flagged.
    #[test]
    fn has_aurifeuillean_base2() {
        assert!(has_aurifeuillean_factorization(2, 6));
        assert!(has_aurifeuillean_factorization(2, 10));
        assert!(has_aurifeuillean_factorization(2, 14));
        assert!(!has_aurifeuillean_factorization(2, 4));
        assert!(!has_aurifeuillean_factorization(2, 8));
        assert!(!has_aurifeuillean_factorization(2, 12));
    }

    /// Detection: base 5 with t=5 ≡ 1 (mod 4), n ≡ 5 (mod 10).
    #[test]
    fn has_aurifeuillean_base5() {
        // base 5: s=1, t=5. t ≡ 1 (mod 4). n ≡ 5 (mod 10).
        assert!(has_aurifeuillean_factorization(5, 5));
        assert!(has_aurifeuillean_factorization(5, 15));
        assert!(has_aurifeuillean_factorization(5, 25));
        assert!(!has_aurifeuillean_factorization(5, 10));
        assert!(!has_aurifeuillean_factorization(5, 20));
    }

    /// Perfect square bases (t=1) should not have Aurifeuillean factorizations.
    #[test]
    fn no_aurifeuillean_for_perfect_squares() {
        assert!(!has_aurifeuillean_factorization(4, 5)); // 4 = 2², t=1
        assert!(!has_aurifeuillean_factorization(9, 3)); // 9 = 3², t=1
        assert!(!has_aurifeuillean_factorization(16, 7)); // 16 = 4², t=1
    }

    /// Edge cases: base 0, base 1, exponent 0.
    #[test]
    fn edge_cases() {
        assert!(!has_aurifeuillean_factorization(0, 10));
        assert!(!has_aurifeuillean_factorization(1, 10));
        assert!(!has_aurifeuillean_factorization(2, 0));
    }

    /// Base 3: t=3 ≡ 3 (mod 4), n ≡ 3 (mod 6).
    /// This applies to b^n - 1 forms (repunits), not b^n + 1.
    #[test]
    fn has_aurifeuillean_base3() {
        // base 3: s=1, t=3. t ≡ 3 (mod 4). n ≡ 3 (mod 6).
        assert!(has_aurifeuillean_factorization(3, 3));
        assert!(has_aurifeuillean_factorization(3, 9));
        assert!(has_aurifeuillean_factorization(3, 15));
        assert!(!has_aurifeuillean_factorization(3, 6));
        assert!(!has_aurifeuillean_factorization(3, 12));
    }
}
