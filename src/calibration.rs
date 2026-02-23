//! # Calibration — Local Machine Performance Benchmarking
//!
//! Measures per-machine timings for key arithmetic operations at multiple bit
//! sizes, persisting results to `~/.darkreach/calibration.json`. The cached
//! timings feed into [`crate::sieve::calibrated_sieve_depth()`] to select
//! optimal sieve depth based on actual hardware performance rather than
//! theoretical estimates.
//!
//! ## Motivation
//!
//! Prime95's self-tuning system (benchmarking FFT implementations at startup)
//! yields up to 10% performance improvement over default settings. darkreach's
//! sieve auto-tuning currently uses a hardcoded `bits^2` cost model that
//! doesn't account for hardware differences (Apple M1 vs. Xeon vs. Epyc).
//! By measuring actual squaring and primality test timings, we can select
//! sieve depths that are optimal for each specific machine.
//!
//! ## Design
//!
//! - **Cache location**: `~/.darkreach/calibration.json`
//! - **Freshness**: 24 hours (re-benchmark if older)
//! - **Benchmark suite**: `Integer::square()` and `Integer::is_probably_prime(2)`
//!   at 5 bit sizes (1K, 10K, 100K, 500K, 1M), plus sieve throughput
//! - **Startup cost**: ~2-5 seconds (amortized over 24 hours)
//! - **Interpolation**: log-log linear between measured points
//!
//! ## References
//!
//! - Prime95 self-tuning: <https://www.mersenne.org/download/>
//! - GIMPS benchmark methodology: <https://www.mersenne.org/various/math.php>

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::{info, warn};

/// Bit sizes at which we benchmark arithmetic operations.
/// Chosen to span the range of typical darkreach search candidates:
/// - 1K bits (~300 digits): small kbn, palindromic
/// - 10K bits (~3000 digits): medium kbn, factorial
/// - 100K bits (~30K digits): large kbn, primorial
/// - 500K bits (~150K digits): frontier factorial
/// - 1M bits (~300K digits): extreme range
const BENCHMARK_BIT_SIZES: [u64; 5] = [1_000, 10_000, 100_000, 500_000, 1_000_000];

/// Number of iterations for each benchmark to reduce noise.
/// We take the minimum time (least system interference).
const BENCHMARK_ITERATIONS: u32 = 3;

/// Cache freshness threshold: re-benchmark after 24 hours.
const FRESHNESS_HOURS: i64 = 24;

/// Local calibration cache persisted to `~/.darkreach/calibration.json`.
///
/// Stores per-bit-size timings measured on this specific machine. Used by
/// sieve auto-tuning to select optimal sieve depth based on actual hardware
/// performance rather than theoretical cost models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationCache {
    /// Timing samples keyed by `"operation:bit_size"` (e.g., `"square:10000"`).
    pub entries: BTreeMap<String, TimingSample>,
    /// Hostname of the machine that ran the benchmarks.
    pub hostname: String,
    /// When the benchmarks were last run.
    pub last_calibrated: DateTime<Utc>,
    /// darkreach version that generated this cache.
    pub version: String,
}

/// A single timing measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingSample {
    /// Mean time in nanoseconds across all iterations.
    pub mean_ns: f64,
    /// Number of iterations measured.
    pub sample_count: u32,
    /// Minimum time in nanoseconds (used for interpolation — least noise).
    pub min_ns: f64,
}

impl CalibrationCache {
    /// Path to the calibration cache file.
    pub fn cache_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".darkreach")
            .join("calibration.json")
    }

    /// Load the calibration cache from disk, returning `None` if it doesn't exist
    /// or can't be parsed.
    pub fn load() -> Option<Self> {
        let path = Self::cache_path();
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Save the calibration cache to disk, creating the directory if needed.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::cache_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        // Atomic write: write to temp file, then rename
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &data)?;
        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    /// Returns `true` if the cache was calibrated less than 24 hours ago.
    pub fn is_fresh(&self) -> bool {
        let age = Utc::now() - self.last_calibrated;
        age.num_hours() < FRESHNESS_HOURS
    }

    /// Look up a timing sample by operation and bit size.
    pub fn get(&self, operation: &str, bit_size: u64) -> Option<&TimingSample> {
        let key = format!("{}:{}", operation, bit_size);
        self.entries.get(&key)
    }

    /// Estimate the time in seconds for a primality test at the given bit size.
    ///
    /// Interpolates between measured `is_prime_2round` samples using log-log
    /// linear interpolation. This assumes a power-law relationship between
    /// bit size and test time (which holds for modular exponentiation:
    /// cost ~ bits^2 for schoolbook, bits^1.585 for Karatsuba, etc.).
    ///
    /// Falls back to `None` if no calibration data is available.
    pub fn estimate_test_secs(&self, bit_size: u64) -> Option<f64> {
        self.interpolate("is_prime_2round", bit_size)
            .map(|ns| ns / 1e9)
    }

    /// Estimate the time in seconds for a squaring operation at the given bit size.
    pub fn estimate_square_secs(&self, bit_size: u64) -> Option<f64> {
        self.interpolate("square", bit_size).map(|ns| ns / 1e9)
    }

    /// Log-log linear interpolation between two adjacent measured points.
    ///
    /// For power-law relationships (time ~ bits^k), log-log interpolation
    /// gives exact results: log(t) = k * log(bits) + c, so we interpolate
    /// linearly in log-space.
    fn interpolate(&self, operation: &str, bit_size: u64) -> Option<f64> {
        // Find the two bracketing measurements
        let mut lower: Option<(u64, f64)> = None;
        let mut upper: Option<(u64, f64)> = None;

        for &bs in &BENCHMARK_BIT_SIZES {
            if let Some(sample) = self.get(operation, bs) {
                if bs <= bit_size {
                    lower = Some((bs, sample.min_ns));
                }
                if bs >= bit_size && upper.is_none() {
                    upper = Some((bs, sample.min_ns));
                }
            }
        }

        match (lower, upper) {
            (Some((lb, lt)), Some((ub, ut))) => {
                if lb == ub {
                    // Exact match
                    return Some(lt);
                }
                // Log-log linear interpolation
                let log_lb = (lb as f64).ln();
                let log_ub = (ub as f64).ln();
                let log_lt = lt.ln();
                let log_ut = ut.ln();
                let log_bs = (bit_size as f64).ln();

                let t = (log_bs - log_lb) / (log_ub - log_lb);
                let log_result = log_lt + t * (log_ut - log_lt);
                Some(log_result.exp())
            }
            (Some((_lb, lt)), None) => {
                // Extrapolate above the highest measured point using last slope
                // Use the slope from the last two measured points
                let mut points: Vec<(u64, f64)> = Vec::new();
                for &bs in &BENCHMARK_BIT_SIZES {
                    if let Some(sample) = self.get(operation, bs) {
                        points.push((bs, sample.min_ns));
                    }
                }
                if points.len() >= 2 {
                    let (b1, t1) = points[points.len() - 2];
                    let (b2, t2) = points[points.len() - 1];
                    let slope =
                        (t2.ln() - t1.ln()) / ((b2 as f64).ln() - (b1 as f64).ln());
                    let log_result =
                        t2.ln() + slope * ((bit_size as f64).ln() - (b2 as f64).ln());
                    Some(log_result.exp())
                } else {
                    Some(lt)
                }
            }
            (None, Some((_ub, ut))) => {
                // Below the lowest measured point — use the lowest
                Some(ut)
            }
            (None, None) => None,
        }
    }
}

/// Run benchmarks and return a fresh `CalibrationCache`.
///
/// Benchmarks three operations at each of the 5 standard bit sizes:
/// 1. `Integer::square()` — the inner loop of modular exponentiation
/// 2. `Integer::is_probably_prime(2)` — 2-round Miller-Rabin (the cheapest useful test)
/// 3. Sieve throughput — primes generated per second
///
/// Each benchmark runs `BENCHMARK_ITERATIONS` times; we record the minimum
/// (least system interference) and the mean.
pub fn run_benchmarks() -> CalibrationCache {
    use rug::Integer;
    use std::time::Instant;

    info!("Running startup calibration benchmarks...");
    let mut entries = BTreeMap::new();

    for &bits in &BENCHMARK_BIT_SIZES {
        // Generate a random odd number of the given bit size
        let mut candidate = Integer::from(1u32) << (bits as u32 - 1);
        // Fill with some non-trivial pattern
        candidate += Integer::from(0xDEAD_BEEFu64);
        candidate |= Integer::from(1u32); // ensure odd

        // --- Squaring benchmark ---
        {
            let mut times_ns = Vec::with_capacity(BENCHMARK_ITERATIONS as usize);
            for _ in 0..BENCHMARK_ITERATIONS {
                let mut x = candidate.clone();
                let start = Instant::now();
                // Do multiple squarings to get measurable time for small sizes
                let reps = if bits <= 10_000 { 100 } else { 1 };
                for _ in 0..reps {
                    x = Integer::from(&x * &x);
                    // Truncate back to target bit size to prevent unbounded growth.
                    // Without this, repeated squaring doubles the bit count each
                    // iteration, causing GMP overflow after ~20 iterations.
                    let sig = x.significant_bits();
                    let target = bits as u32;
                    if sig > target + target {
                        x >>= sig - target;
                    }
                }
                let elapsed = start.elapsed().as_nanos() as f64 / reps as f64;
                times_ns.push(elapsed);
                std::hint::black_box(&x);
            }
            let min_ns = times_ns.iter().copied().fold(f64::INFINITY, f64::min);
            let mean_ns = times_ns.iter().sum::<f64>() / times_ns.len() as f64;
            let key = format!("square:{}", bits);
            entries.insert(
                key,
                TimingSample {
                    mean_ns,
                    sample_count: BENCHMARK_ITERATIONS,
                    min_ns,
                },
            );
        }

        // --- Primality test benchmark (2-round MR) ---
        // Skip for 1M bits — takes too long for a startup benchmark
        if bits <= 500_000 {
            let mut times_ns = Vec::with_capacity(BENCHMARK_ITERATIONS as usize);
            for _ in 0..BENCHMARK_ITERATIONS {
                let start = Instant::now();
                let _ = std::hint::black_box(candidate.is_probably_prime(2));
                let elapsed = start.elapsed().as_nanos() as f64;
                times_ns.push(elapsed);
            }
            let min_ns = times_ns.iter().copied().fold(f64::INFINITY, f64::min);
            let mean_ns = times_ns.iter().sum::<f64>() / times_ns.len() as f64;
            let key = format!("is_prime_2round:{}", bits);
            entries.insert(
                key,
                TimingSample {
                    mean_ns,
                    sample_count: BENCHMARK_ITERATIONS,
                    min_ns,
                },
            );
        }
    }

    // --- Sieve throughput benchmark ---
    {
        let mut times_ns = Vec::with_capacity(BENCHMARK_ITERATIONS as usize);
        for _ in 0..BENCHMARK_ITERATIONS {
            let start = Instant::now();
            let primes = crate::sieve::generate_primes(1_000_000);
            let elapsed = start.elapsed().as_nanos() as f64;
            std::hint::black_box(&primes);
            times_ns.push(elapsed);
        }
        let min_ns = times_ns.iter().copied().fold(f64::INFINITY, f64::min);
        let mean_ns = times_ns.iter().sum::<f64>() / times_ns.len() as f64;
        entries.insert(
            "sieve_1m".to_string(),
            TimingSample {
                mean_ns,
                sample_count: BENCHMARK_ITERATIONS,
                min_ns,
            },
        );
    }

    let hostname = std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let cache = CalibrationCache {
        entries,
        hostname,
        last_calibrated: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Log summary
    for &bits in &BENCHMARK_BIT_SIZES {
        if let Some(sq) = cache.get("square", bits) {
            info!(
                bits,
                square_us = format!("{:.1}", sq.min_ns / 1000.0),
                "calibration: squaring"
            );
        }
        if let Some(pr) = cache.get("is_prime_2round", bits) {
            info!(
                bits,
                prime_test_ms = format!("{:.2}", pr.min_ns / 1_000_000.0),
                "calibration: 2-round MR"
            );
        }
    }
    if let Some(sieve) = cache.entries.get("sieve_1m") {
        info!(
            sieve_1m_ms = format!("{:.1}", sieve.min_ns / 1_000_000.0),
            "calibration: sieve(1M)"
        );
    }

    cache
}

/// Load the calibration cache if fresh, otherwise run benchmarks and save.
///
/// This is the main entry point called at startup. It:
/// 1. Tries to load `~/.darkreach/calibration.json`
/// 2. If the cache exists and is < 24 hours old, returns it
/// 3. Otherwise, runs benchmarks (~2-5 seconds) and saves the new cache
///
/// Returns the calibration cache for use by sieve auto-tuning.
pub fn ensure_calibrated() -> CalibrationCache {
    if let Some(cache) = CalibrationCache::load() {
        if cache.is_fresh() {
            let age_hours = (Utc::now() - cache.last_calibrated).num_hours();
            info!(
                age_hours,
                hostname = %cache.hostname,
                "Using cached calibration"
            );
            return cache;
        }
        info!("Calibration cache is stale, re-benchmarking...");
    }

    let cache = run_benchmarks();
    if let Err(e) = cache.save() {
        warn!(error = %e, "Failed to save calibration cache");
    } else {
        info!(
            path = %CalibrationCache::cache_path().display(),
            "Calibration cache saved"
        );
    }
    cache
}

#[cfg(test)]
mod tests {
    //! # Calibration Tests
    //!
    //! Validates the calibration system: benchmark execution, cache persistence,
    //! interpolation accuracy, and freshness detection.

    use super::*;

    /// Run benchmarks should produce entries for all expected operations and bit sizes.
    #[test]
    fn run_benchmarks_produces_entries() {
        let cache = run_benchmarks();

        // Should have squaring entries for all 5 bit sizes
        for &bits in &BENCHMARK_BIT_SIZES {
            assert!(
                cache.get("square", bits).is_some(),
                "missing square:{} entry",
                bits
            );
        }

        // Should have MR entries for bit sizes <= 500K
        for &bits in &[1_000u64, 10_000, 100_000, 500_000] {
            assert!(
                cache.get("is_prime_2round", bits).is_some(),
                "missing is_prime_2round:{} entry",
                bits
            );
        }

        // Should have sieve throughput
        assert!(
            cache.entries.contains_key("sieve_1m"),
            "missing sieve_1m entry"
        );
    }

    /// All timing samples should be positive (non-zero).
    #[test]
    fn benchmark_timings_are_positive() {
        let cache = run_benchmarks();
        for (key, sample) in &cache.entries {
            assert!(
                sample.min_ns > 0.0,
                "{}: min_ns should be > 0, got {}",
                key,
                sample.min_ns
            );
            assert!(
                sample.mean_ns > 0.0,
                "{}: mean_ns should be > 0, got {}",
                key,
                sample.mean_ns
            );
            assert!(
                sample.min_ns <= sample.mean_ns,
                "{}: min_ns ({}) should be <= mean_ns ({})",
                key,
                sample.min_ns,
                sample.mean_ns
            );
        }
    }

    /// Squaring time should increase monotonically with bit size.
    /// This validates both the benchmark and the underlying GMP performance model.
    #[test]
    fn squaring_time_increases_with_bit_size() {
        let cache = run_benchmarks();
        let mut prev_ns = 0.0f64;
        for &bits in &BENCHMARK_BIT_SIZES {
            let sample = cache.get("square", bits).unwrap();
            assert!(
                sample.min_ns >= prev_ns,
                "square:{} ({:.0} ns) should be >= previous ({:.0} ns)",
                bits,
                sample.min_ns,
                prev_ns
            );
            prev_ns = sample.min_ns;
        }
    }

    /// MR test time should increase monotonically with bit size.
    #[test]
    fn mr_time_increases_with_bit_size() {
        let cache = run_benchmarks();
        let mr_sizes = [1_000u64, 10_000, 100_000, 500_000];
        let mut prev_ns = 0.0f64;
        for &bits in &mr_sizes {
            let sample = cache.get("is_prime_2round", bits).unwrap();
            assert!(
                sample.min_ns >= prev_ns,
                "is_prime_2round:{} ({:.0} ns) should be >= previous ({:.0} ns)",
                bits,
                sample.min_ns,
                prev_ns
            );
            prev_ns = sample.min_ns;
        }
    }

    /// Interpolation at a measured point should return the exact value.
    #[test]
    fn interpolate_exact_match() {
        let cache = run_benchmarks();
        let sample = cache.get("square", 10_000).unwrap();
        let interpolated = cache.interpolate("square", 10_000).unwrap();
        assert!(
            (interpolated - sample.min_ns).abs() < 1e-6,
            "exact match should return exact value"
        );
    }

    /// Interpolation between two points should give a value between them.
    #[test]
    fn interpolate_between_points() {
        let cache = run_benchmarks();
        let lo = cache.get("square", 10_000).unwrap().min_ns;
        let hi = cache.get("square", 100_000).unwrap().min_ns;
        let mid = cache.interpolate("square", 50_000).unwrap();
        assert!(
            mid >= lo && mid <= hi,
            "interpolation at 50K should be between 10K ({:.0}) and 100K ({:.0}), got {:.0}",
            lo,
            hi,
            mid
        );
    }

    /// `estimate_test_secs` should return reasonable values for typical candidate sizes.
    #[test]
    fn estimate_test_secs_reasonable() {
        let cache = run_benchmarks();
        // 10K bits should take > 0 and < 1000 seconds
        if let Some(secs) = cache.estimate_test_secs(10_000) {
            assert!(secs > 0.0, "test time should be > 0");
            assert!(secs < 1000.0, "test time should be < 1000s for 10K bits");
        }
    }

    /// A freshly created cache should be fresh.
    #[test]
    fn fresh_cache_is_fresh() {
        let cache = CalibrationCache {
            entries: BTreeMap::new(),
            hostname: "test".to_string(),
            last_calibrated: Utc::now(),
            version: "test".to_string(),
        };
        assert!(cache.is_fresh());
    }

    /// A cache from 25 hours ago should not be fresh.
    #[test]
    fn stale_cache_is_not_fresh() {
        let cache = CalibrationCache {
            entries: BTreeMap::new(),
            hostname: "test".to_string(),
            last_calibrated: Utc::now() - chrono::Duration::hours(25),
            version: "test".to_string(),
        };
        assert!(!cache.is_fresh());
    }

    /// Save and load roundtrip should preserve all data.
    #[test]
    fn save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("calibration.json");

        let mut entries = BTreeMap::new();
        entries.insert(
            "square:1000".to_string(),
            TimingSample {
                mean_ns: 42.0,
                sample_count: 3,
                min_ns: 40.0,
            },
        );
        let cache = CalibrationCache {
            entries,
            hostname: "test-host".to_string(),
            last_calibrated: Utc::now(),
            version: "0.1.0".to_string(),
        };

        // Save to the temp path
        let data = serde_json::to_string_pretty(&cache).unwrap();
        std::fs::write(&path, &data).unwrap();

        // Load back
        let loaded_data = std::fs::read_to_string(&path).unwrap();
        let loaded: CalibrationCache = serde_json::from_str(&loaded_data).unwrap();

        assert_eq!(loaded.hostname, "test-host");
        assert_eq!(loaded.entries.len(), 1);
        let sample = loaded.entries.get("square:1000").unwrap();
        assert!((sample.mean_ns - 42.0).abs() < 1e-6);
        assert!((sample.min_ns - 40.0).abs() < 1e-6);
        assert_eq!(sample.sample_count, 3);
    }

    /// Interpolation for a missing operation should return None.
    #[test]
    fn interpolate_missing_operation() {
        let cache = CalibrationCache {
            entries: BTreeMap::new(),
            hostname: "test".to_string(),
            last_calibrated: Utc::now(),
            version: "test".to_string(),
        };
        assert!(cache.interpolate("nonexistent", 1000).is_none());
    }
}
