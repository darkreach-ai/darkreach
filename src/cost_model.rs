//! # Shared Cost Model — Power-Law Estimation for Prime-Hunting Forms
//!
//! Provides the single source of truth for cost model coefficients, fitting,
//! and per-candidate time estimation. Used by:
//!
//! - [`project::cost`] — Project cost estimation (dashboard, CLI)
//! - [`ai_engine`] — AI engine scoring and calibration
//! - [`strategy`] — Form scoring for scheduling decisions
//!
//! ## Power-Law Model
//!
//! Each search form follows: `secs = a * (digits / 1000)^b`
//!
//! Coefficients `(a, b)` are initially hardcoded from empirical benchmarks,
//! then replaced by OLS-fitted values from completed work block data stored
//! in the `cost_calibration` table.
//!
//! ## References
//!
//! - GIMPS timing benchmarks (calibration baseline)
//! - OLS regression on log-log transformed work block data

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// All 12 search forms with their default power-law coefficients.
///
/// Returns a map of form name → `(a, b)` where `secs = a * (digits/1000)^b`.
/// These are empirical defaults calibrated against GIMPS and darkreach benchmarks.
pub fn default_coefficients() -> HashMap<String, (f64, f64)> {
    let mut m = HashMap::with_capacity(12);
    m.insert("factorial".to_string(), (0.5, 2.5));
    m.insert("primorial".to_string(), (0.5, 2.5));
    m.insert("kbn".to_string(), (0.1, 2.0));
    m.insert("twin".to_string(), (0.1, 2.0));
    m.insert("sophie_germain".to_string(), (0.1, 2.0));
    m.insert("cullen_woodall".to_string(), (0.2, 2.2));
    m.insert("carol_kynea".to_string(), (0.2, 2.2));
    m.insert("wagstaff".to_string(), (0.8, 2.5));
    m.insert("palindromic".to_string(), (0.3, 2.0));
    m.insert("near_repdigit".to_string(), (0.3, 2.0));
    m.insert("repunit".to_string(), (0.4, 2.3));
    m.insert("gen_fermat".to_string(), (0.3, 2.2));
    m
}

/// Look up default `(a, b)` coefficients for a form.
///
/// Returns `(0.5, 2.5)` for unknown forms (conservative estimate matching factorial).
pub fn default_coefficients_for(form: &str) -> (f64, f64) {
    match form {
        "factorial" | "primorial" => (0.5, 2.5),
        "kbn" | "twin" | "sophie_germain" => (0.1, 2.0),
        "cullen_woodall" | "carol_kynea" => (0.2, 2.2),
        "wagstaff" => (0.8, 2.5),
        "palindromic" | "near_repdigit" => (0.3, 2.0),
        "repunit" => (0.4, 2.3),
        "gen_fermat" => (0.3, 2.2),
        _ => (0.5, 2.5),
    }
}

/// Estimate seconds per candidate using power-law coefficients.
///
/// Applies `secs = a * (digits/1000)^b` with an optional PFGW speedup
/// factor for large candidates (≥10K digits).
///
/// # Arguments
/// - `a`, `b` — Power-law coefficients
/// - `digits` — Decimal digit count of the candidate
/// - `pfgw_speedup` — If `Some(factor)`, divide result by `factor` for candidates ≥10K digits.
///   Pass `None` for no PFGW acceleration.
pub fn secs_per_candidate_from_coefficients(
    a: f64,
    b: f64,
    digits: u64,
    pfgw_speedup: Option<f64>,
) -> f64 {
    let d = digits as f64 / 1000.0;
    let base = a * d.powf(b);

    if let Some(speedup) = pfgw_speedup {
        if digits >= 10_000 {
            return base / speedup;
        }
    }
    base
}

/// A single cost observation from a completed work block.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CostObservation {
    pub digits: f64,
    pub secs: f64,
}

/// Fit power-law `secs = a * (digits/1000)^b` via OLS on log-log data.
///
/// Transforms `(digits, secs)` pairs to `(ln(digits/1000), ln(secs))` and
/// performs ordinary least squares linear regression. The slope gives `b`
/// and the intercept gives `ln(a)`.
///
/// Returns `(a, b, MAPE)` or `None` if:
/// - Fewer than 3 valid observations (positive digits and secs)
/// - Degenerate data (zero variance in x)
///
/// ## MAPE (Mean Absolute Percentage Error)
///
/// Measures fit quality: MAPE < 0.10 is excellent, < 0.30 is acceptable,
/// > 0.50 suggests the power-law model is a poor fit.
pub fn fit_power_law(observations: &[CostObservation]) -> Option<(f64, f64, f64)> {
    if observations.len() < 3 {
        return None;
    }

    // Filter valid points (positive digits and secs)
    let points: Vec<(f64, f64)> = observations
        .iter()
        .filter(|o| o.digits > 0.0 && o.secs > 0.0)
        .map(|o| ((o.digits / 1000.0).ln(), o.secs.ln()))
        .collect();

    if points.len() < 3 {
        return None;
    }

    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
    let sum_xx: f64 = points.iter().map(|(x, _)| x * x).sum();

    let denom = n * sum_xx - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let b = (n * sum_xy - sum_x * sum_y) / denom;
    let ln_a = (sum_y - b * sum_x) / n;
    let a = ln_a.exp();

    // Compute MAPE (Mean Absolute Percentage Error)
    let mape: f64 = observations
        .iter()
        .filter(|o| o.digits > 0.0 && o.secs > 0.0)
        .map(|o| {
            let predicted = a * (o.digits / 1000.0).powf(b);
            ((o.secs - predicted) / o.secs).abs()
        })
        .sum::<f64>()
        / points.len() as f64;

    Some((a, b, mape))
}

/// Compute the digit count where PFGW subprocess becomes faster than GMP native.
///
/// Solves for `d` where GMP timing equals PFGW subprocess overhead + PFGW timing:
/// `a * (d/1000)^b = overhead + a * (d/1000)^b / speedup`
///
/// Rearranging: `a * (d/1000)^b * (1 - 1/speedup) = overhead`
/// → `(d/1000)^b = overhead / (a * (1 - 1/speedup))`
/// → `d = 1000 * (overhead / (a * (1 - 1/speedup)))^(1/b)`
///
/// Assumes subprocess overhead of ~0.1 seconds (process spawn + IPC).
///
/// Returns `None` if speedup ≤ 1.0, coefficients are invalid, or the
/// crossover falls outside the valid range [100, 1_000_000] digits.
pub fn compute_pfgw_crossover(gmp_a: f64, gmp_b: f64, pfgw_speedup: f64) -> Option<u64> {
    if pfgw_speedup <= 1.0 || gmp_a <= 0.0 || gmp_b <= 0.0 {
        return None;
    }

    let overhead = 0.1; // subprocess spawn overhead in seconds
    let factor = 1.0 - 1.0 / pfgw_speedup;
    if factor <= 0.0 {
        return None;
    }

    let base = overhead / (gmp_a * factor);
    if base <= 0.0 {
        return None;
    }

    let d = 1000.0 * base.powf(1.0 / gmp_b);
    if d.is_finite() {
        Some((d as u64).clamp(100, 1_000_000))
    } else {
        None
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_coefficients_has_12_forms() {
        let defaults = default_coefficients();
        assert_eq!(defaults.len(), 12);
    }

    #[test]
    fn default_coefficients_for_known_forms() {
        assert_eq!(default_coefficients_for("factorial"), (0.5, 2.5));
        assert_eq!(default_coefficients_for("kbn"), (0.1, 2.0));
        assert_eq!(default_coefficients_for("wagstaff"), (0.8, 2.5));
        assert_eq!(default_coefficients_for("repunit"), (0.4, 2.3));
    }

    #[test]
    fn default_coefficients_for_unknown_form() {
        assert_eq!(default_coefficients_for("unknown_xyz"), (0.5, 2.5));
    }

    #[test]
    fn default_coefficients_match_hashmap() {
        let map = default_coefficients();
        for (form, expected) in &map {
            let (a, b) = default_coefficients_for(form);
            assert_eq!((a, b), *expected, "Mismatch for form '{}'", form);
        }
    }

    #[test]
    fn secs_per_candidate_basic() {
        // At 1000 digits, d=1.0, so secs = a * 1.0^b = a
        let secs = secs_per_candidate_from_coefficients(0.5, 2.5, 1000, None);
        assert!((secs - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn secs_per_candidate_scaling() {
        let t1 = secs_per_candidate_from_coefficients(0.1, 2.0, 1000, None);
        let t2 = secs_per_candidate_from_coefficients(0.1, 2.0, 2000, None);
        let ratio = t2 / t1;
        // d^2.0: ratio should be 2^2 = 4.0
        assert!(
            (ratio - 4.0).abs() < 0.01,
            "ratio should be ~4.0, got {}",
            ratio
        );
    }

    #[test]
    fn secs_per_candidate_pfgw_applied_at_10k() {
        let without = secs_per_candidate_from_coefficients(0.5, 2.5, 10_000, None);
        let with = secs_per_candidate_from_coefficients(0.5, 2.5, 10_000, Some(50.0));
        let ratio = without / with;
        assert!(
            (ratio - 50.0).abs() < 0.01,
            "PFGW should give 50x speedup at 10K digits, got {}",
            ratio
        );
    }

    #[test]
    fn secs_per_candidate_pfgw_not_applied_below_10k() {
        let without = secs_per_candidate_from_coefficients(0.5, 2.5, 9_999, None);
        let with = secs_per_candidate_from_coefficients(0.5, 2.5, 9_999, Some(50.0));
        assert!(
            (without - with).abs() < f64::EPSILON,
            "PFGW should not apply below 10K digits"
        );
    }

    #[test]
    fn secs_per_candidate_zero_digits() {
        let t = secs_per_candidate_from_coefficients(0.5, 2.5, 0, None);
        assert!(t == 0.0 || t >= 0.0);
    }

    #[test]
    fn fit_power_law_exact_data() {
        let obs: Vec<CostObservation> = (1..=10)
            .map(|i| {
                let digits = 1000.0 * i as f64;
                let secs = 0.5 * (digits / 1000.0).powf(2.5);
                CostObservation { digits, secs }
            })
            .collect();

        let (a, b, mape) = fit_power_law(&obs).expect("Should fit exact data");
        assert!((a - 0.5).abs() < 0.01, "a should be ~0.5, got {}", a);
        assert!((b - 2.5).abs() < 0.01, "b should be ~2.5, got {}", b);
        assert!(
            mape < 0.01,
            "MAPE should be near 0 for exact data, got {}",
            mape
        );
    }

    #[test]
    fn fit_power_law_insufficient_data() {
        let obs = vec![
            CostObservation {
                digits: 1000.0,
                secs: 0.5,
            },
            CostObservation {
                digits: 2000.0,
                secs: 2.8,
            },
        ];
        assert!(fit_power_law(&obs).is_none(), "Should need >= 3 points");
    }

    #[test]
    fn fit_power_law_filters_invalid() {
        let obs = vec![
            CostObservation {
                digits: 0.0,
                secs: 0.0,
            },
            CostObservation {
                digits: -1.0,
                secs: 0.5,
            },
            CostObservation {
                digits: 1000.0,
                secs: 0.5,
            },
        ];
        assert!(
            fit_power_law(&obs).is_none(),
            "Should filter invalid and need >= 3 valid points"
        );
    }

    // ── PFGW crossover ─────────────────────────────────────────

    #[test]
    fn pfgw_crossover_basic() {
        let crossover = compute_pfgw_crossover(0.5, 2.5, 50.0);
        assert!(crossover.is_some(), "Should compute crossover");
        let d = crossover.unwrap();
        assert!(d >= 100 && d <= 1_000_000, "Crossover {} out of range", d);
    }

    #[test]
    fn pfgw_crossover_higher_speedup_means_lower_crossover() {
        let d50 = compute_pfgw_crossover(0.5, 2.5, 50.0).unwrap();
        let d100 = compute_pfgw_crossover(0.5, 2.5, 100.0).unwrap();
        assert!(
            d100 <= d50,
            "Higher speedup should give lower crossover: d100={} vs d50={}",
            d100,
            d50
        );
    }

    #[test]
    fn pfgw_crossover_no_speedup() {
        assert!(compute_pfgw_crossover(0.5, 2.5, 1.0).is_none());
        assert!(compute_pfgw_crossover(0.5, 2.5, 0.5).is_none());
    }

    #[test]
    fn pfgw_crossover_invalid_coefficients() {
        assert!(compute_pfgw_crossover(0.0, 2.5, 50.0).is_none());
        assert!(compute_pfgw_crossover(-1.0, 2.5, 50.0).is_none());
        assert!(compute_pfgw_crossover(0.5, 0.0, 50.0).is_none());
    }

    // ── MAPE accuracy ───────────────────────────────────────────

    #[test]
    fn fit_mape_measures_prediction_error() {
        // Generate data with known 10% noise: each secs = true * (1 + noise)
        // MAPE should be approximately equal to the noise fraction.
        let obs: Vec<CostObservation> = (1..=20)
            .map(|i| {
                let digits = 1000.0 * i as f64;
                let true_secs = 0.3 * (digits / 1000.0).powf(2.0);
                // Alternate +5% and -5% noise → mean absolute = 5%
                let noise = if i % 2 == 0 { 1.05 } else { 0.95 };
                CostObservation {
                    digits,
                    secs: true_secs * noise,
                }
            })
            .collect();

        let (_, _, mape) = fit_power_law(&obs).unwrap();
        // With ±5% noise, MAPE should be around 0.05 ± some tolerance
        assert!(
            mape < 0.15,
            "MAPE should be low with ±5% noise, got {}",
            mape
        );
        assert!(
            mape > 0.01,
            "MAPE should be nonzero with noise, got {}",
            mape
        );
    }

    #[test]
    fn fit_power_law_many_points_improves_stability() {
        // More data points should produce a good fit even with noise
        let obs: Vec<CostObservation> = (1..=50)
            .map(|i| {
                let digits = 500.0 + 200.0 * i as f64;
                let true_secs = 0.2 * (digits / 1000.0).powf(2.2);
                // Small noise
                let noise = 1.0 + (i as f64 * 0.7).sin() * 0.03;
                CostObservation {
                    digits,
                    secs: true_secs * noise,
                }
            })
            .collect();

        let (a, b, mape) = fit_power_law(&obs).unwrap();
        assert!((a - 0.2).abs() < 0.05, "a should be ~0.2, got {}", a);
        assert!((b - 2.2).abs() < 0.1, "b should be ~2.2, got {}", b);
        assert!(
            mape < 0.10,
            "MAPE should be low with 50 points, got {}",
            mape
        );
    }

    #[test]
    fn secs_per_candidate_at_1000_digits_equals_a() {
        // d = 1000/1000 = 1.0, so secs = a * 1.0^b = a for any b
        for (a, b) in [(0.1, 2.0), (0.5, 2.5), (1.0, 3.0), (0.01, 1.5)] {
            let secs = secs_per_candidate_from_coefficients(a, b, 1000, None);
            assert!(
                (secs - a).abs() < f64::EPSILON,
                "At 1000 digits, secs should equal a={}, got {} (b={})",
                a,
                secs,
                b
            );
        }
    }

    #[test]
    fn secs_per_candidate_pfgw_exactly_at_10k() {
        // At exactly 10_000 digits, PFGW speedup SHOULD apply
        let without = secs_per_candidate_from_coefficients(0.5, 2.5, 10_000, None);
        let with = secs_per_candidate_from_coefficients(0.5, 2.5, 10_000, Some(100.0));
        let ratio = without / with;
        assert!(
            (ratio - 100.0).abs() < 0.01,
            "PFGW should apply at exactly 10K: ratio={}",
            ratio
        );
    }

    #[test]
    fn secs_per_candidate_pfgw_at_9999() {
        // At 9999 digits, PFGW speedup should NOT apply
        let without = secs_per_candidate_from_coefficients(0.5, 2.5, 9_999, None);
        let with = secs_per_candidate_from_coefficients(0.5, 2.5, 9_999, Some(100.0));
        assert!(
            (without - with).abs() < f64::EPSILON,
            "PFGW should NOT apply at 9999 digits: without={}, with={}",
            without,
            with
        );
    }

    #[test]
    fn fit_power_law_all_same_digits_returns_none() {
        // Zero variance in x → regression fails
        let obs = vec![
            CostObservation {
                digits: 1000.0,
                secs: 0.5,
            },
            CostObservation {
                digits: 1000.0,
                secs: 0.6,
            },
            CostObservation {
                digits: 1000.0,
                secs: 0.4,
            },
        ];
        assert!(
            fit_power_law(&obs).is_none(),
            "Same digits should produce zero variance → None"
        );
    }

    #[test]
    fn fit_power_law_three_points_minimum() {
        let obs = vec![
            CostObservation {
                digits: 1000.0,
                secs: 0.1,
            },
            CostObservation {
                digits: 2000.0,
                secs: 0.4,
            },
            CostObservation {
                digits: 3000.0,
                secs: 0.9,
            },
        ];
        assert!(fit_power_law(&obs).is_some(), "3 points should be enough");
    }
}
