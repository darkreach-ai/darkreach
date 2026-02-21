//! # Classify — Multi-Form Prime Classification Engine
//!
//! Determines which tags a prime should have based on its form, expression,
//! proof method, and mathematical properties. Classification runs at two points:
//!
//! 1. **Discovery time** ([`classify_at_discovery`]): assigns structural and proof
//!    tags based on the search form. Cheap — no primality tests needed.
//!
//! 2. **Verification time** ([`classify_at_verification`]): after a prime is verified,
//!    checks for cross-form memberships (palindromic structure, twin relation,
//!    Sophie Germain relation, safe prime). May require additional primality tests
//!    for relational properties, so expensive checks are gated by digit count.
//!
//! ## Tag Taxonomy
//!
//! | Category | Examples | Assigned When |
//! |----------|----------|---------------|
//! | Structural | `factorial`, `kbn`, `palindromic` | Discovery |
//! | Proof | `deterministic`, `probabilistic`, `prp-only` | Discovery |
//! | Property | `twin`, `safe-prime`, `sophie-germain` | Verification |
//! | Verification | `verified-tier-1`, `verified-tier-2`, `verified-tier-3` | Verification |
//!
//! ## References
//!
//! - Twin primes: OEIS [A001359](https://oeis.org/A001359)
//! - Sophie Germain primes: OEIS [A005384](https://oeis.org/A005384)
//! - Safe primes: OEIS [A005385](https://oeis.org/A005385)

use rug::integer::IsPrime;
use rug::Integer;

use crate::verify::{is_provable_form, VerifyResult};

/// Maximum digit count for expensive cross-form checks (twin, Sophie Germain, safe prime).
///
/// Testing whether p±2 or 2p+1 is prime requires a full primality test on a number
/// of similar size. For primes above this threshold, only structural checks
/// (palindrome, digit analysis) are performed.
const EXPENSIVE_CHECK_DIGIT_LIMIT: u64 = 50_000;

/// Assign tags at discovery time based on form, proof method, and form relationships.
///
/// This is cheap (no primality tests) and runs in the search form's hot loop.
/// Returns a deduplicated, sorted list of tags.
pub fn classify_at_discovery(form: &str, proof_method: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::with_capacity(4);

    // Primary form tag
    tags.push(form.to_string());

    // Proof classification
    if proof_method == "deterministic" {
        tags.push("deterministic".to_string());
    } else {
        tags.push("probabilistic".to_string());
    }
    if !is_provable_form(form) {
        tags.push("prp-only".to_string());
    }

    // Cross-form structural tags: forms that reuse kbn infrastructure
    match form {
        "twin" | "sophie_germain" | "cullen" | "woodall" | "cullen_woodall" | "carol" | "kynea"
        | "carol_kynea" | "gen_fermat" => {
            tags.push("kbn".to_string());
        }
        "near_repdigit" => {
            tags.push("palindromic".to_string());
        }
        _ => {}
    }

    tags.sort();
    tags.dedup();
    tags
}

/// Assign tags at verification time based on verification result and mathematical properties.
///
/// Runs after `verify_prime()` succeeds. Performs cheap structural checks on all
/// primes and expensive relational checks (twin, Sophie Germain, safe prime) only
/// on primes below [`EXPENSIVE_CHECK_DIGIT_LIMIT`] digits.
pub fn classify_at_verification(
    candidate: &Integer,
    digits: u64,
    verify_result: &VerifyResult,
) -> Vec<String> {
    let mut tags: Vec<String> = Vec::with_capacity(4);

    // Verification tier tag
    match verify_result {
        VerifyResult::Verified { tier: 1, .. } => tags.push("verified-tier-1".to_string()),
        VerifyResult::Verified { tier: 2, .. } => tags.push("verified-tier-2".to_string()),
        VerifyResult::Verified { tier: 3, .. } => tags.push("verified-tier-3".to_string()),
        _ => {}
    }

    // Structural check: is this a palindrome in base 10? O(digits) — always cheap.
    if is_palindrome_base10(candidate) {
        tags.push("palindromic".to_string());
    }

    // Relational checks: require primality tests on related numbers.
    // Only for primes below the digit limit.
    if digits < EXPENSIVE_CHECK_DIGIT_LIMIT {
        if is_twin_prime(candidate) {
            tags.push("twin".to_string());
        }
        if is_sophie_germain(candidate) {
            tags.push("sophie-germain".to_string());
        }
        if is_safe_prime(candidate) {
            tags.push("safe-prime".to_string());
        }
    }

    tags
}

/// Check if a number is a palindrome in base 10.
///
/// Converts to decimal string and compares with its reverse. O(digits).
pub fn is_palindrome_base10(n: &Integer) -> bool {
    let s = n.to_string_radix(10);
    let bytes = s.as_bytes();
    let len = bytes.len();
    for i in 0..len / 2 {
        if bytes[i] != bytes[len - 1 - i] {
            return false;
        }
    }
    true
}

/// Check if p is part of a twin prime pair (p-2 or p+2 is also prime).
///
/// Uses GMP's `is_probably_prime(25)` for the primality test on the neighbor.
/// Returns true if either p-2 or p+2 is probably prime.
fn is_twin_prime(p: &Integer) -> bool {
    if *p <= 2u32 {
        return false;
    }
    let p_minus_2 = Integer::from(p - 2u32);
    if p_minus_2 > 1u32 && p_minus_2.is_probably_prime(25) != IsPrime::No {
        return true;
    }
    let p_plus_2 = Integer::from(p + 2u32);
    p_plus_2.is_probably_prime(25) != IsPrime::No
}

/// Check if p is a Sophie Germain prime (2p+1 is also prime).
///
/// Sophie Germain primes satisfy: p prime AND 2p+1 prime.
/// OEIS A005384: 2, 3, 5, 11, 23, 29, 41, 53, 83, 89, ...
fn is_sophie_germain(p: &Integer) -> bool {
    let safe = Integer::from(p * 2u32) + 1u32;
    safe.is_probably_prime(25) != IsPrime::No
}

/// Check if p is a safe prime ((p-1)/2 is also prime).
///
/// Safe primes satisfy: p prime AND (p-1)/2 prime.
/// OEIS A005385: 5, 7, 11, 23, 47, 59, 83, 107, 167, 179, ...
fn is_safe_prime(p: &Integer) -> bool {
    if *p <= 4u32 {
        return false;
    }
    // p must be odd for (p-1)/2 to potentially be prime (except p=2,5)
    let p_minus_1 = Integer::from(p - 1u32);
    if p_minus_1.is_divisible_2pow(1) {
        let half = Integer::from(&p_minus_1 >> 1);
        half.is_probably_prime(25) != IsPrime::No
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    //! Tests for the classification engine.
    //!
    //! Verifies both discovery-time tagging (form + proof tags) and
    //! verification-time property detection (palindrome, twin, Sophie Germain,
    //! safe prime).

    use super::*;

    // ── Discovery-Time Classification ─────────────────────────────

    #[test]
    fn discovery_factorial_deterministic() {
        let tags = classify_at_discovery("factorial", "deterministic");
        assert!(tags.contains(&"factorial".to_string()));
        assert!(tags.contains(&"deterministic".to_string()));
        assert!(!tags.contains(&"prp-only".to_string()));
    }

    #[test]
    fn discovery_wagstaff_probabilistic() {
        let tags = classify_at_discovery("wagstaff", "probabilistic");
        assert!(tags.contains(&"wagstaff".to_string()));
        assert!(tags.contains(&"probabilistic".to_string()));
        assert!(tags.contains(&"prp-only".to_string()));
    }

    #[test]
    fn discovery_twin_gets_kbn_tag() {
        let tags = classify_at_discovery("twin", "deterministic");
        assert!(tags.contains(&"twin".to_string()));
        assert!(tags.contains(&"kbn".to_string()));
    }

    #[test]
    fn discovery_near_repdigit_gets_palindromic_tag() {
        let tags = classify_at_discovery("near_repdigit", "deterministic");
        assert!(tags.contains(&"near_repdigit".to_string()));
        assert!(tags.contains(&"palindromic".to_string()));
    }

    #[test]
    fn discovery_tags_are_sorted_and_deduped() {
        let tags = classify_at_discovery("kbn", "deterministic");
        let mut sorted = tags.clone();
        sorted.sort();
        assert_eq!(tags, sorted);
        // No duplicates
        let mut deduped = tags.clone();
        deduped.dedup();
        assert_eq!(tags, deduped);
    }

    // ── Palindrome Detection ──────────────────────────────────────

    #[test]
    fn palindrome_single_digit() {
        assert!(is_palindrome_base10(&Integer::from(7u32)));
    }

    #[test]
    fn palindrome_known() {
        assert!(is_palindrome_base10(&Integer::from(10301u32)));
        assert!(is_palindrome_base10(&Integer::from(12321u32)));
    }

    #[test]
    fn not_palindrome() {
        assert!(!is_palindrome_base10(&Integer::from(12345u32)));
        assert!(!is_palindrome_base10(&Integer::from(100u32)));
    }

    // ── Twin Prime Detection ──────────────────────────────────────

    #[test]
    fn twin_prime_5() {
        // 5 is twin: 3 and 5, or 5 and 7
        assert!(is_twin_prime(&Integer::from(5u32)));
    }

    #[test]
    fn twin_prime_11() {
        // 11 is twin: 11 and 13
        assert!(is_twin_prime(&Integer::from(11u32)));
    }

    #[test]
    fn not_twin_prime_23() {
        // 23: 21=3*7 (not prime), 25=5^2 (not prime) → not twin
        assert!(!is_twin_prime(&Integer::from(23u32)));
    }

    // ── Sophie Germain Detection ──────────────────────────────────

    #[test]
    fn sophie_germain_11() {
        // 11: 2*11+1 = 23 (prime) → Sophie Germain
        assert!(is_sophie_germain(&Integer::from(11u32)));
    }

    #[test]
    fn sophie_germain_23() {
        // 23: 2*23+1 = 47 (prime) → Sophie Germain
        assert!(is_sophie_germain(&Integer::from(23u32)));
    }

    #[test]
    fn not_sophie_germain_13() {
        // 13: 2*13+1 = 27 = 3^3 (not prime) → not Sophie Germain
        assert!(!is_sophie_germain(&Integer::from(13u32)));
    }

    // ── Safe Prime Detection ──────────────────────────────────────

    #[test]
    fn safe_prime_7() {
        // 7: (7-1)/2 = 3 (prime) → safe prime
        assert!(is_safe_prime(&Integer::from(7u32)));
    }

    #[test]
    fn safe_prime_23() {
        // 23: (23-1)/2 = 11 (prime) → safe prime
        assert!(is_safe_prime(&Integer::from(23u32)));
    }

    #[test]
    fn not_safe_prime_13() {
        // 13: (13-1)/2 = 6 (not prime) → not safe prime
        assert!(!is_safe_prime(&Integer::from(13u32)));
    }

    // ── Verification-Time Classification ──────────────────────────

    #[test]
    fn verification_tier1_tag() {
        let result = VerifyResult::Verified {
            method: "tier1-proth".into(),
            tier: 1,
        };
        let tags = classify_at_verification(&Integer::from(97u32), 2, &result);
        assert!(tags.contains(&"verified-tier-1".to_string()));
    }

    #[test]
    fn verification_detects_palindrome() {
        let result = VerifyResult::Verified {
            method: "tier2-bpsw+mr10".into(),
            tier: 2,
        };
        let tags = classify_at_verification(&Integer::from(10301u32), 5, &result);
        assert!(tags.contains(&"palindromic".to_string()));
        assert!(tags.contains(&"verified-tier-2".to_string()));
    }

    #[test]
    fn verification_detects_twin() {
        let result = VerifyResult::Verified {
            method: "tier2-bpsw+mr10".into(),
            tier: 2,
        };
        // 11 is twin (11, 13)
        let tags = classify_at_verification(&Integer::from(11u32), 2, &result);
        assert!(tags.contains(&"twin".to_string()));
    }

    #[test]
    fn verification_skips_expensive_checks_for_large_primes() {
        let result = VerifyResult::Verified {
            method: "tier2-bpsw+mr10".into(),
            tier: 2,
        };
        // Use digit count above limit — twin/sophie/safe checks should be skipped
        let tags = classify_at_verification(&Integer::from(11u32), 60_000, &result);
        assert!(!tags.contains(&"twin".to_string()));
        assert!(!tags.contains(&"sophie-germain".to_string()));
        assert!(!tags.contains(&"safe-prime".to_string()));
        // But verification tier tag should still be present
        assert!(tags.contains(&"verified-tier-2".to_string()));
    }
}
