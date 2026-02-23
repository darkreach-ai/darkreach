//! # Resource Types — Heterogeneous Resource Pooling
//!
//! Defines the resource type system for heterogeneous operator nodes: GPU compute,
//! distributed storage, and network relay. Provides auto-detection of GPU runtime
//! and form-specific GPU affinity scoring.
//!
//! ## Resource Capabilities
//!
//! Operators can contribute three resource types beyond CPU:
//! - **GPU**: CUDA, HIP (ROCm), Metal (Apple), or OpenCL compute
//! - **Storage**: Cache, archive, or proof vault roles
//! - **Network**: Relay and sieve seeder roles for distributed sieve data
//!
//! ## GPU Auto-Detection
//!
//! [`detect_gpu_runtime`] probes the system in priority order:
//! 1. `DARKREACH_GPU_RUNTIME` environment variable (explicit override)
//! 2. `nvidia-smi` presence (CUDA)
//! 3. `/sys/class/kfd` presence (AMD HIP/ROCm)
//! 4. Apple aarch64 target (Metal)
//! 5. Falls back to [`GpuRuntime::None`]
//!
//! ## GPU Affinity
//!
//! [`gpu_affinity`] returns a 0.0–1.0 score indicating how much a search form
//! benefits from GPU acceleration. Forms with heavy modular exponentiation
//! (kbn, gen_fermat) score highest; forms dominated by I/O or irregular memory
//! access (palindromic) score lowest.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ── GPU Runtime ─────────────────────────────────────────────────

/// GPU compute runtime available on an operator node.
///
/// Detection priority: env var → nvidia-smi → /sys/class/kfd → Apple aarch64 → None.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GpuRuntime {
    #[default]
    None,
    Cuda,
    Hip,
    Metal,
    Opencl,
}

impl fmt::Display for GpuRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuRuntime::None => write!(f, "none"),
            GpuRuntime::Cuda => write!(f, "cuda"),
            GpuRuntime::Hip => write!(f, "hip"),
            GpuRuntime::Metal => write!(f, "metal"),
            GpuRuntime::Opencl => write!(f, "opencl"),
        }
    }
}

impl FromStr for GpuRuntime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" | "" => Ok(GpuRuntime::None),
            "cuda" => Ok(GpuRuntime::Cuda),
            "hip" | "rocm" => Ok(GpuRuntime::Hip),
            "metal" => Ok(GpuRuntime::Metal),
            "opencl" => Ok(GpuRuntime::Opencl),
            other => Err(format!("unknown GPU runtime: {}", other)),
        }
    }
}

// ── Storage Role ────────────────────────────────────────────────

/// Role this operator's storage serves in the distributed sieve/proof pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageRole {
    #[default]
    None,
    Cache,
    Archive,
    ProofVault,
}

impl fmt::Display for StorageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageRole::None => write!(f, "none"),
            StorageRole::Cache => write!(f, "cache"),
            StorageRole::Archive => write!(f, "archive"),
            StorageRole::ProofVault => write!(f, "proofvault"),
        }
    }
}

impl FromStr for StorageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" | "" => Ok(StorageRole::None),
            "cache" => Ok(StorageRole::Cache),
            "archive" => Ok(StorageRole::Archive),
            "proofvault" | "proof_vault" => Ok(StorageRole::ProofVault),
            other => Err(format!("unknown storage role: {}", other)),
        }
    }
}

// ── Network Role ────────────────────────────────────────────────

/// Role this operator serves in the network topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetworkRole {
    #[default]
    Worker,
    Relay,
    SieveSeeder,
}

impl fmt::Display for NetworkRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkRole::Worker => write!(f, "worker"),
            NetworkRole::Relay => write!(f, "relay"),
            NetworkRole::SieveSeeder => write!(f, "sieveseeder"),
        }
    }
}

impl FromStr for NetworkRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "worker" | "" => Ok(NetworkRole::Worker),
            "relay" => Ok(NetworkRole::Relay),
            "sieveseeder" | "sieve_seeder" => Ok(NetworkRole::SieveSeeder),
            other => Err(format!("unknown network role: {}", other)),
        }
    }
}

// ── Resource Capabilities ───────────────────────────────────────

/// Full resource capability declaration for an operator node.
///
/// Sent during registration and stored as JSONB in `operator_nodes.resource_capabilities`.
/// All fields are optional beyond the enum defaults (None/Worker).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceCapabilities {
    pub gpu_runtime: GpuRuntime,
    pub gpu_compute_units: Option<i32>,
    pub gpu_benchmark_gflops: Option<f32>,
    pub storage_role: StorageRole,
    pub storage_dedicated_gb: Option<i32>,
    pub storage_medium: Option<String>,
    pub network_role: NetworkRole,
    pub network_upload_mbps: Option<f32>,
    pub network_download_mbps: Option<f32>,
    pub network_region: Option<String>,
    pub network_public_ip: bool,
}

// ── GPU Auto-Detection ──────────────────────────────────────────

/// Detect the GPU runtime available on this system.
///
/// Priority:
/// 1. `DARKREACH_GPU_RUNTIME` env var (explicit override, case-insensitive)
/// 2. `nvidia-smi` binary on PATH → [`GpuRuntime::Cuda`]
/// 3. `/sys/class/kfd` directory exists → [`GpuRuntime::Hip`] (AMD ROCm)
/// 4. Apple aarch64 target → [`GpuRuntime::Metal`]
/// 5. [`GpuRuntime::None`]
///
/// All detection is best-effort and never panics.
pub fn detect_gpu_runtime() -> GpuRuntime {
    // 1. Explicit env var override
    if let Ok(val) = std::env::var("DARKREACH_GPU_RUNTIME") {
        if let Ok(rt) = val.parse::<GpuRuntime>() {
            return rt;
        }
    }

    // 2. nvidia-smi → CUDA
    if std::process::Command::new("nvidia-smi")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return GpuRuntime::Cuda;
    }

    // 3. /sys/class/kfd → HIP (AMD ROCm)
    if std::path::Path::new("/sys/class/kfd").exists() {
        return GpuRuntime::Hip;
    }

    // 4. Apple Silicon → Metal
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return GpuRuntime::Metal;
    }

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    GpuRuntime::None
}

// ── GPU Affinity ────────────────────────────────────────────────

/// Return the GPU affinity score for a search form (0.0–1.0).
///
/// Higher values indicate forms that benefit more from GPU acceleration:
/// - **0.9**: kbn, gen_fermat — heavy modular exponentiation, regular memory access
/// - **0.8**: twin, sophie_germain — dual Proth/LLR tests, GPU-parallelizable
/// - **0.7**: wagstaff, repunit — large FFT-based primality tests
/// - **0.5**: factorial, primorial — mixed GMP + sieve, partial GPU benefit
/// - **0.3**: palindromic, near_repdigit — irregular digit manipulation
/// - **0.4**: cullen_woodall, carol_kynea — moderate GPU benefit
pub fn gpu_affinity(form: &str) -> f64 {
    match form {
        "kbn" | "gen_fermat" => 0.9,
        "twin" | "sophie_germain" => 0.8,
        "wagstaff" | "repunit" => 0.7,
        "factorial" | "primorial" => 0.5,
        "cullen_woodall" | "carol_kynea" => 0.4,
        "palindromic" | "near_repdigit" => 0.3,
        _ => 0.3,
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Enum Display/FromStr Roundtrips ──────────────────────────

    #[test]
    fn gpu_runtime_display_fromstr_roundtrip() {
        for rt in [
            GpuRuntime::None,
            GpuRuntime::Cuda,
            GpuRuntime::Hip,
            GpuRuntime::Metal,
            GpuRuntime::Opencl,
        ] {
            let s = rt.to_string();
            let parsed: GpuRuntime = s.parse().unwrap();
            assert_eq!(parsed, rt, "roundtrip failed for {:?}", rt);
        }
    }

    #[test]
    fn gpu_runtime_case_insensitive() {
        assert_eq!("CUDA".parse::<GpuRuntime>().unwrap(), GpuRuntime::Cuda);
        assert_eq!("Metal".parse::<GpuRuntime>().unwrap(), GpuRuntime::Metal);
        assert_eq!("rocm".parse::<GpuRuntime>().unwrap(), GpuRuntime::Hip);
    }

    #[test]
    fn gpu_runtime_empty_is_none() {
        assert_eq!("".parse::<GpuRuntime>().unwrap(), GpuRuntime::None);
    }

    #[test]
    fn storage_role_display_fromstr_roundtrip() {
        for role in [
            StorageRole::None,
            StorageRole::Cache,
            StorageRole::Archive,
            StorageRole::ProofVault,
        ] {
            let s = role.to_string();
            let parsed: StorageRole = s.parse().unwrap();
            assert_eq!(parsed, role, "roundtrip failed for {:?}", role);
        }
    }

    #[test]
    fn network_role_display_fromstr_roundtrip() {
        for role in [
            NetworkRole::Worker,
            NetworkRole::Relay,
            NetworkRole::SieveSeeder,
        ] {
            let s = role.to_string();
            let parsed: NetworkRole = s.parse().unwrap();
            assert_eq!(parsed, role, "roundtrip failed for {:?}", role);
        }
    }

    // ── GPU Detection ───────────────────────────────────────────

    /// Test GPU runtime detection via env var.
    /// These tests modify shared env state, so they use a serial guard.
    #[test]
    fn detect_gpu_runtime_env_overrides() {
        // Test cuda override
        unsafe { std::env::set_var("DARKREACH_GPU_RUNTIME", "cuda") };
        assert_eq!(detect_gpu_runtime(), GpuRuntime::Cuda);

        // Test none override
        unsafe { std::env::set_var("DARKREACH_GPU_RUNTIME", "none") };
        assert_eq!(detect_gpu_runtime(), GpuRuntime::None);

        // Clean up
        unsafe { std::env::remove_var("DARKREACH_GPU_RUNTIME") };
    }

    // ── GPU Affinity ────────────────────────────────────────────

    #[test]
    fn gpu_affinity_values() {
        assert!((gpu_affinity("kbn") - 0.9).abs() < f64::EPSILON);
        assert!((gpu_affinity("gen_fermat") - 0.9).abs() < f64::EPSILON);
        assert!((gpu_affinity("twin") - 0.8).abs() < f64::EPSILON);
        assert!((gpu_affinity("factorial") - 0.5).abs() < f64::EPSILON);
        assert!((gpu_affinity("palindromic") - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn gpu_affinity_unknown_form() {
        assert!((gpu_affinity("unknown_form") - 0.3).abs() < f64::EPSILON);
    }

    // ── ResourceCapabilities ────────────────────────────────────

    #[test]
    fn resource_capabilities_default() {
        let caps = ResourceCapabilities::default();
        assert_eq!(caps.gpu_runtime, GpuRuntime::None);
        assert_eq!(caps.storage_role, StorageRole::None);
        assert_eq!(caps.network_role, NetworkRole::Worker);
        assert!(!caps.network_public_ip);
    }

    #[test]
    fn resource_capabilities_serde_roundtrip() {
        let caps = ResourceCapabilities {
            gpu_runtime: GpuRuntime::Cuda,
            gpu_compute_units: Some(128),
            gpu_benchmark_gflops: Some(12.5),
            storage_role: StorageRole::Cache,
            storage_dedicated_gb: Some(500),
            storage_medium: Some("nvme".to_string()),
            network_role: NetworkRole::Relay,
            network_upload_mbps: Some(1000.0),
            network_download_mbps: Some(2000.0),
            network_region: Some("us-east-1".to_string()),
            network_public_ip: true,
        };
        let json = serde_json::to_string(&caps).unwrap();
        let parsed: ResourceCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.gpu_runtime, GpuRuntime::Cuda);
        assert_eq!(parsed.gpu_compute_units, Some(128));
        assert_eq!(parsed.storage_role, StorageRole::Cache);
        assert_eq!(parsed.network_role, NetworkRole::Relay);
        assert!(parsed.network_public_ip);
    }
}
