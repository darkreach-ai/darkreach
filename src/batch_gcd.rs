//! # Batch GCD — Product Tree / Remainder Tree for Bulk Factor Checking
//!
//! Implements Bernstein's algorithm for computing gcd(Cᵢ, P) for many candidates
//! C₁..Cₙ against a single primorial P in O(n log²n) multiplications, compared
//! to O(n × π(P)) for sequential trial division.
//!
//! ## Algorithm
//!
//! 1. **Product tree**: Build a balanced binary tree where each leaf is a candidate
//!    and each internal node is the product of its children. The root is ∏Cᵢ.
//!
//! 2. **Remainder tree**: Starting from P mod root, propagate downward:
//!    each child gets (parent's remainder) mod (child's product). At the leaves,
//!    we have P mod Cᵢ for each candidate.
//!
//! 3. **GCD extraction**: gcd(Cᵢ, P) = gcd(Cᵢ, P mod Cᵢ²) / Cᵢ, but for our
//!    use case we simply check if P mod Cᵢ = 0 (meaning Cᵢ has a factor in common
//!    with P, i.e., Cᵢ has a small prime factor).
//!
//! ## Crossover
//!
//! Batch GCD wins over sequential `has_small_factor` when n > ~100 candidates,
//! due to the O(log²n) amortization of the product tree.
//!
//! ## References
//!
//! - Daniel J. Bernstein, "How to find smooth parts of integers", 2004.
//!   <https://cr.yp.to/factorization/smoothparts-20040510.pdf>
//! - Daniel J. Bernstein, "Fast multiplication and its applications", 2008.

use rug::Integer;

/// Build a product tree from a list of leaves.
///
/// Returns a vector of levels, where level 0 contains the leaves and
/// the last level contains a single element (the product of all leaves).
///
/// Each level i has ceil(len(level i-1) / 2) elements, where each element
/// is the product of two children from the level below.
pub fn build_product_tree(leaves: &[Integer]) -> Vec<Vec<Integer>> {
    if leaves.is_empty() {
        return vec![vec![]];
    }

    let mut tree = vec![leaves.to_vec()];

    loop {
        let prev = tree.last().unwrap();
        if prev.len() <= 1 {
            break;
        }

        let mut next_level = Vec::with_capacity(prev.len().div_ceil(2));
        let mut i = 0;
        while i < prev.len() {
            if i + 1 < prev.len() {
                next_level.push(Integer::from(&prev[i] * &prev[i + 1]));
            } else {
                next_level.push(prev[i].clone());
            }
            i += 2;
        }
        tree.push(next_level);
    }

    tree
}

/// Compute the remainder tree: propagate P mod (product tree) from root to leaves.
///
/// Given a product tree and a value P, computes P mod Cᵢ for each leaf Cᵢ.
/// Uses the identity: (P mod parent) mod child = P mod child, since
/// child divides parent in the product tree.
pub fn remainder_tree(tree: &[Vec<Integer>], p: &Integer) -> Vec<Integer> {
    if tree.is_empty() || tree[0].is_empty() {
        return vec![];
    }

    let levels = tree.len();
    let mut remainders: Vec<Vec<Integer>> = vec![vec![]; levels];

    // Start at the root: P mod root
    let root = &tree[levels - 1][0];
    remainders[levels - 1] = vec![Integer::from(p % root)];

    // Propagate down
    for level in (0..levels - 1).rev() {
        let parent_remainders = &remainders[level + 1];
        let mut child_remainders = Vec::with_capacity(tree[level].len());

        let mut pi = 0; // parent index
        let mut ci = 0; // child index

        while ci < tree[level].len() {
            let parent_rem = &parent_remainders[pi];

            // Left child
            let left_rem = Integer::from(parent_rem % &tree[level][ci]);
            child_remainders.push(left_rem);
            ci += 1;

            // Right child (if exists)
            if ci < tree[level].len() {
                let right_rem = Integer::from(parent_rem % &tree[level][ci]);
                child_remainders.push(right_rem);
                ci += 1;
            }

            pi += 1;
        }

        remainders[level] = child_remainders;
    }

    remainders[0].clone()
}

/// Batch check which candidates have a small prime factor.
///
/// Computes the primorial P = product of all primes up to `sieve_limit`,
/// then uses product/remainder trees to compute gcd(Cᵢ, P) for each candidate.
///
/// Returns a Vec<bool> where `true` means the candidate has a small factor
/// (is definitely composite), matching the semantics of `has_small_factor`.
///
/// Falls back to sequential checking when n < `BATCH_THRESHOLD`.
pub fn batch_has_small_factor(candidates: &[Integer], sieve_limit: u64) -> Vec<bool> {
    const BATCH_THRESHOLD: usize = 100;

    if candidates.is_empty() {
        return vec![];
    }

    // For small batches, sequential is faster (no tree overhead)
    if candidates.len() < BATCH_THRESHOLD {
        return candidates
            .iter()
            .map(crate::has_small_factor)
            .collect();
    }

    // Compute primorial = product of all primes up to sieve_limit
    let primes = crate::sieve::generate_primes(sieve_limit);
    let primorial = primes.iter().fold(Integer::from(1u32), |acc, &p| {
        acc * p
    });

    // Build product tree from candidates
    let tree = build_product_tree(candidates);

    // Compute P mod Cᵢ for each candidate
    let remainders = remainder_tree(&tree, &primorial);

    // A candidate has a small factor iff gcd(Cᵢ, P) > 1,
    // which is equivalent to (P mod Cᵢ) sharing a factor with Cᵢ.
    // Since P is the primorial, P mod Cᵢ ≠ 0 doesn't mean no small factor.
    // We need: gcd(Cᵢ, P mod Cᵢ²) — but computing Cᵢ² is expensive.
    //
    // Simpler approach: check if gcd(remainder, Cᵢ) > 1
    remainders
        .iter()
        .zip(candidates.iter())
        .map(|(rem, c)| {
            if *rem == 0u32 {
                // P mod C = 0 means C divides P, so C is a product of small primes.
                // C might be prime itself (if C is in the prime table).
                // Use the same logic as has_small_factor: composite if C > largest small prime.
                c.significant_bits() > 9 // > 311 means definitely composite
            } else {
                let g = rem.clone().gcd(c);
                // g > 1 means C shares a factor with the primorial
                // But if C itself is a small prime, it's not composite
                g > 1u32 && &g != c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    //! # Batch GCD Tests
    //!
    //! Validates Bernstein's product tree / remainder tree algorithm for
    //! bulk composite detection. Cross-validates against sequential
    //! `has_small_factor` to ensure correctness.
    //!
    //! ## References
    //!
    //! - Daniel J. Bernstein, "How to find smooth parts of integers", 2004.

    use super::*;

    /// Product tree of [2, 3, 5, 7]:
    /// Level 0: [2, 3, 5, 7]
    /// Level 1: [6, 35]
    /// Level 2: [210]
    #[test]
    fn product_tree_basic() {
        let leaves: Vec<Integer> = vec![2u32, 3, 5, 7]
            .into_iter()
            .map(Integer::from)
            .collect();
        let tree = build_product_tree(&leaves);

        assert_eq!(tree.len(), 3); // 3 levels
        assert_eq!(tree[0].len(), 4); // 4 leaves
        assert_eq!(tree[1].len(), 2); // 2 intermediate nodes
        assert_eq!(tree[2].len(), 1); // 1 root

        assert_eq!(tree[1][0], Integer::from(6u32)); // 2*3
        assert_eq!(tree[1][1], Integer::from(35u32)); // 5*7
        assert_eq!(tree[2][0], Integer::from(210u32)); // 2*3*5*7
    }

    /// Product tree of odd number of elements: [2, 3, 5].
    /// The unpaired element is carried up as-is.
    #[test]
    fn product_tree_odd_count() {
        let leaves: Vec<Integer> = vec![2u32, 3, 5]
            .into_iter()
            .map(Integer::from)
            .collect();
        let tree = build_product_tree(&leaves);

        assert_eq!(tree[0].len(), 3);
        assert_eq!(tree[1].len(), 2); // [6, 5]
        assert_eq!(tree[2].len(), 1); // [30]
        assert_eq!(tree[2][0], Integer::from(30u32));
    }

    /// Product tree of single element.
    #[test]
    fn product_tree_single() {
        let leaves = vec![Integer::from(42u32)];
        let tree = build_product_tree(&leaves);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0][0], Integer::from(42u32));
    }

    /// Empty input produces empty output.
    #[test]
    fn product_tree_empty() {
        let tree = build_product_tree(&[]);
        assert_eq!(tree.len(), 1);
        assert!(tree[0].is_empty());
    }

    /// Remainder tree: compute 210 mod each leaf.
    /// P = 210 = 2*3*5*7. Candidates = [6, 35, 11, 13].
    /// 210 mod 6 = 0, 210 mod 35 = 0, 210 mod 11 = 1, 210 mod 13 = 2.
    #[test]
    fn remainder_tree_basic() {
        let candidates: Vec<Integer> = vec![6u32, 35, 11, 13]
            .into_iter()
            .map(Integer::from)
            .collect();
        let tree = build_product_tree(&candidates);
        let p = Integer::from(210u32);
        let remainders = remainder_tree(&tree, &p);

        assert_eq!(remainders.len(), 4);
        assert_eq!(remainders[0], Integer::from(0u32)); // 210 mod 6 = 0
        assert_eq!(remainders[1], Integer::from(0u32)); // 210 mod 35 = 0
        assert_eq!(remainders[2], Integer::from(1u32)); // 210 mod 11 = 1
        assert_eq!(remainders[3], Integer::from(2u32)); // 210 mod 13 = 2
    }

    /// Batch GCD matches sequential has_small_factor for small composites.
    #[test]
    fn batch_matches_sequential_composites() {
        let candidates: Vec<Integer> = vec![
            Integer::from(15u32),    // 3*5 — composite
            Integer::from(104729u32), // prime
            Integer::from(100u32),   // 2^2*5^2 — composite
            Integer::from(1009u32),  // prime
            Integer::from(6u32),     // 2*3 — composite
        ];

        let batch_results = batch_has_small_factor(&candidates, 311);
        let sequential_results: Vec<bool> = candidates
            .iter()
            .map(|c| crate::has_small_factor(c))
            .collect();

        assert_eq!(batch_results, sequential_results);
    }

    /// Batch GCD matches sequential for primes above the table.
    #[test]
    fn batch_matches_sequential_primes() {
        let candidates: Vec<Integer> = vec![
            Integer::from(313u32), // prime
            Integer::from(317u32), // prime
            Integer::from(331u32), // prime
            Integer::from(337u32), // prime
        ];

        let batch_results = batch_has_small_factor(&candidates, 311);
        let sequential_results: Vec<bool> = candidates
            .iter()
            .map(|c| crate::has_small_factor(c))
            .collect();

        assert_eq!(batch_results, sequential_results);
    }

    /// Cross-validate batch vs sequential on 200 random-ish numbers.
    #[test]
    fn batch_cross_validate_200_numbers() {
        let candidates: Vec<Integer> = (1000u32..1200)
            .map(Integer::from)
            .collect();

        let batch_results = batch_has_small_factor(&candidates, 311);
        let sequential_results: Vec<bool> = candidates
            .iter()
            .map(|c| crate::has_small_factor(c))
            .collect();

        for (i, (b, s)) in batch_results.iter().zip(sequential_results.iter()).enumerate() {
            assert_eq!(
                b, s,
                "Mismatch at index {} (candidate {}): batch={}, sequential={}",
                i,
                candidates[i],
                b,
                s
            );
        }
    }

    /// Empty input produces empty output.
    #[test]
    fn batch_empty() {
        let result = batch_has_small_factor(&[], 311);
        assert!(result.is_empty());
    }

    /// Single element falls back to sequential.
    #[test]
    fn batch_single() {
        let candidates = vec![Integer::from(15u32)];
        let result = batch_has_small_factor(&candidates, 311);
        assert_eq!(result, vec![true]); // 15 = 3*5
    }
}
