# Heterogeneous Resource Pooling Roadmap

**Goal:** Extend darkreach's distributed compute model beyond CPU-only to support GPU compute, distributed storage, network relay, and commercial compute marketplace — all managed by the AI engine's OODA loop. See [Path C](path-c.md) for the hybrid platform strategy.

---

## Context

Darkreach currently treats all workers as CPU-only compute nodes. The AI engine scores forms, assigns work, and learns cost curves — but only in terms of core-hours. Meanwhile:

- GPU-accelerated sieving kernels (mtsieve OpenCL) exist for 6 forms
- Workers already report `has_gpu`, `gpu_model`, `gpu_vram_gb` in `operator_nodes` — but these are barely used (just a boolean gate)
- Large searches generate massive sieve tables (BitSieve can be GBs) that every worker regenerates independently
- The coordinator is a single-point bottleneck for distributing work data to large fleets

This roadmap broadens the resource model to **GPU compute, distributed storage, and network relay**.

---

## Data Model

### Resource Enums

```
GpuRuntime: Cuda | Opencl | Metal | Hip | None
StorageRole: None | Cache | Archive | ProofVault
NetworkRole: Worker | Relay | SieveSeeder
```

### ResourceCapabilities struct

Wraps GPU/storage/network capabilities into a single serializable struct sent during registration and stored as JSONB.

### FormResourceAffinity

Per-form GPU affinity scores, sieve memory estimates, and shared sieve benefit flags:

| Form | GPU Affinity | Sieve Memory | Shared Sieve |
|------|-------------|--------------|--------------|
| kbn | 0.9 | High | Yes |
| palindromic | 0.7 | Medium | Yes |
| factorial | 0.1 | Low | No |
| twin | 0.8 | High | Yes |
| sophie_germain | 0.8 | High | Yes |

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    AI Engine (OODA)                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ gpu_fit   │ │storage_fit│ │net_locale │ + 7 base │
│  └──────────┘ └──────────┘ └──────────┘            │
├─────────────────────────────────────────────────────┤
│                  Resource Router                     │
│  GPU work → GPU nodes    CPU work → any node        │
│  Sieve data → storage nodes    Relay → relay nodes  │
├─────────────────────────────────────────────────────┤
│           operator_nodes (extended schema)           │
│  gpu_runtime, gpu_compute_units, gpu_benchmark      │
│  storage_role, storage_dedicated_gb                  │
│  network_role, network_upload_mbps, region           │
│  resource_capabilities (JSONB catch-all)             │
└─────────────────────────────────────────────────────┘
```

---

## Phases

### Phase 1: Resource Model + Extended Registration
**Files:** `src/resource.rs`, `src/lib.rs`, `src/operator.rs`, `src/metrics.rs`, `src/db/operators.rs`, migration
**Ship:** Nodes register with full capabilities. CPU-only nodes unchanged.

- Define `GpuRuntime`, `StorageRole`, `NetworkRole` enums
- Define `ResourceCapabilities` struct with GPU/storage/network fields
- Define `FormResourceAffinity` table with per-form GPU scores
- Extend `operator_nodes` table with new columns (all optional, backward-compat)
- Auto-detect GPU runtime (nvidia-smi → CUDA, /sys/class/kfd → HIP, Apple Silicon → Metal)
- Extend `HardwareMetrics` with optional GPU/storage/network fields
- Send full `ResourceCapabilities` as JSONB alongside typed columns

### Phase 2: GPU-Aware Work Routing
**Files:** `src/db/operators.rs`, `src/operator.rs`, `src/dashboard/routes_operator.rs`, `src/project/config.rs`
**Ship:** GPU work goes to GPU nodes. CPU workers unaffected.

- Extend `WorkerCapabilities` with `gpu_runtime`, `gpu_vram_gb`, `storage_role`, `network_region`
- Extend `claim_operator_block()` SQL: match `gpu_runtime` and `min_gpu_vram_gb` from `search_jobs.params`
- CPU-only workers never receive GPU-required blocks; GPU workers can still take CPU work
- Extend `InfrastructureConfig` with `gpu_runtime`, `min_gpu_vram_gb`, `preferred_region`

### Phase 3: AI Engine Resource Awareness
**Files:** `src/ai_engine.rs`, `src/db/ai_engine.rs` or `src/db/resources.rs`, `src/db/workers.rs`
**Ship:** AI engine optimizes across CPU+GPU fleet, learns GPU cost curves.

- OBSERVE: new parallel queries for resource fleet summary, shared sieve inventory, resource contributions
- ORIENT: 7 → 10 scoring components (add `gpu_fit`, `storage_fit`, `network_locality`)
- DECIDE: new decision types (`AssignGpuWork`, `AssignStorageRole`, `PromoteRelay`, `DistributeSieve`)
- LEARN: fit separate cost curves for GPU-accelerated work blocks

### Phase 4: Storage Pooling
**Files:** `src/db/resources.rs` (new), `src/dashboard/routes_sieve.rs` (new), `src/sieve.rs`, engine forms
**Ship:** Workers share pre-computed sieves. Avoids redundant sieve generation.

- Hash-addressed immutable sieve blobs (SHA-256)
- `PUT /api/v1/sieve/{form}/{hash}` — upload sieve blob
- `GET /api/v1/sieve/{form}/{hash}` — download sieve blob
- `BitSieve::serialize()` / `BitSieve::deserialize()` for export/import
- Before generating sieve, check `shared_sieves` table for matching hash

### Phase 5: Network Relay + Bandwidth
**Files:** `src/dashboard/routes_operator.rs`, `src/ai_engine.rs`, `src/db/resources.rs`
**Ship:** High-bandwidth nodes relay sieve data, reducing coordinator load.

- Work assignments include `relay_hint: {url}` for nearest relay node
- AI engine promotes nodes with `network_upload_mbps > 100` and `network_public_ip = true`
- Relays cache sieve data and serve it to nearby compute workers

### Phase 6: Resource Credits + Dashboard
**Files:** `src/db/operators.rs`, `src/prom_metrics.rs`, `src/events.rs`, frontend
**Ship:** Operators see and earn credits for all resource contributions.

- `resource_contributions` table: per-hour cpu core-hours, gpu-hours, storage-gb-hours, bandwidth-gb
- Each resource type has its own credit conversion rate
- Prometheus metrics: `gpu_workers`, `storage_gb`, `relay_bandwidth`
- Dashboard resource visualization on fleet page

### Phase 7: Commercial Resource Marketplace
**Files:** `src/db/operators.rs`, `src/db/resources.rs`, `src/dashboard/routes_operator.rs`, `src/dashboard/routes_resources.rs`, frontend, migrations
**Ship:** Operators earn real money from commercial compute jobs. Full revenue tracking.

> See [Path C Roadmap](path-c.md) for the full business strategy.

- **Revenue-tracked work blocks**: Commercial jobs carry a `price_per_core_hour` field
- **Operator earnings**: Per-block earnings calculated as `duration_hours * cores * price * revenue_share_pct`
- **Earnings ledger**: `operator_earnings` table tracks all payouts with line-item detail
- **Trust-gated tiers**: Level 1-2 operators earn from open science credits only. Level 3+ earn from commercial jobs.
- **Payout system**: Monthly aggregation → Stripe Connect or manual wire for verified operators
- **Dashboard**: Operator earnings page showing lifetime earnings, pending balance, payout history
- **Admin dashboard**: Revenue analytics — total ARR, per-customer revenue, operator cost, margin
- **Fairness enforcement**: AI scheduler guarantees minimum 20% of capacity for open science at all times

---

## Backward Compatibility

- All new `operator_nodes` columns are optional with sensible defaults
- Old workers send null for new fields — coordinator ignores them
- `WorkerCapabilities` fields default to current behavior (CPU-only, no region)
- Existing `has_gpu` boolean preserved (new `gpu_runtime` is additive)
- `resource_capabilities` JSONB provides forward-compat escape hatch

## Phase 8: WASM Module Resources

> See [Technology Vision](technology-vision.md) for the full protocol architecture.

### 8.1 WASM Module Registry

Content-addressed registry for WASM computation modules:
- Modules indexed by SHA-256 hash — integrity verified on download
- OCI-compatible distribution (leverage existing container registry infrastructure)
- Module signing with Ed25519 — operators verify darkreach signature
- Version tagging for human-readable references alongside content addresses

### 8.2 WASM Resource Metering

Fine-grained resource tracking for WASM execution:
- **Fuel-based CPU metering**: Wasmtime fuel system tracks instruction count per work block
- **Memory limits**: per-module linear memory caps enforced by the runtime
- **I/O quotas**: content-addressed input/output size limits per work block
- **Cost attribution**: precise resource usage per job for billing accuracy

### 8.3 Browser Resource Contribution

Lightweight resource model for browser-based operators:
- Battery-aware: reduce computation when on battery power
- Tab visibility: throttle when tab is backgrounded (respect browser power policies)
- Bandwidth-aware: smaller work blocks for mobile/metered connections
- WebGPU capability detection and automatic work type routing

---

## Verification

After each phase:
1. `cargo build` — no compilation errors
2. `cargo test` — all tests pass
3. Existing CPU-only worker registration still works
4. `cargo run -- factorial --start 1 --end 100` — basic search unaffected
