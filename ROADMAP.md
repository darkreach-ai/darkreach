# darkreach Roadmap

**Goal:** Build the world's most advanced prime number discovery platform — then evolve it into a decentralized verified computation protocol. Make real discoveries along the way.

---

## Domain Roadmaps

| Domain | Scope | Roadmap |
|--------|-------|---------|
| **Engine** | Prime-hunting algorithms, sieving, primality testing, proofs | [engine.md](docs/roadmaps/engine.md) |
| **Server** | Dashboard API, WebSocket, coordination, DB, checkpoints | [server.md](docs/roadmaps/server.md) |
| **Frontend** | Next.js dashboard, charts, real-time monitoring | [frontend.md](docs/roadmaps/frontend.md) |
| **Website** | Public-facing website at darkreach.ai | [website.md](docs/roadmaps/website.md) |
| **Ops** | Deployment, build optimization, fleet management | [ops.md](docs/roadmaps/ops.md) |
| **Research** | Discovery strategy, publication pipeline, competitive analysis | [research.md](docs/roadmaps/research.md) |
| **AI Engine** | Autonomous search optimization, ML engine, workforce allocation | [ai-engine.md](docs/roadmaps/ai-engine.md) |
| **Agents** | AI agent execution, orchestration, budgets, automation | [agents.md](docs/roadmaps/agents.md) |
| **OpenClaw** | AI orchestration: Discord, kanban, multi-model routing | [openclaw.md](docs/roadmaps/openclaw.md) |
| **Projects** | Campaign management, orchestration, cost tracking, records | [projects.md](docs/roadmaps/projects.md) |
| **Database** | PostgreSQL scaling, TimescaleDB, Redis, replicas, HA | [database.md](docs/roadmaps/database.md) |
| **Network** | Distributed work claiming, operator client, trust system | [network.md](docs/roadmaps/network.md) |
| **Verification** | Tags system, distributed verification, cross-form discovery | [verification.md](docs/roadmaps/verification.md) |
| **Resources** | Heterogeneous resource pooling (GPU, storage, network relay) | [resources.md](docs/roadmaps/resources.md) |
| **Testing** | Test strategy, integration tests, benchmarks, CI | [testing.md](docs/roadmaps/testing.md) |
| **GWNUM/FLINT** | External library integration for 50-100x acceleration | [gwnum-flint.md](docs/roadmaps/gwnum-flint.md) |
| **Path C** | Hybrid compute platform strategy (open science + paid tiers) | [path-c.md](docs/roadmaps/path-c.md) |
| **Technology Vision** | Protocol architecture (WASM, libp2p, CRDTs, VCs) | [technology-vision.md](docs/roadmaps/technology-vision.md) |
| **Competitive Analysis** | Market landscape and competitor research | [competitive-analysis.md](docs/roadmaps/competitive-analysis.md) |

---

## What's Built (Feb 2026)

- **12 search forms** with sieve → test → proof → log pipeline
- **Deterministic proofs**: Proth, LLR, Pocklington, Morrison, BLS, Pepin
- **Core primitives**: Montgomery multiplication, wheel factorization, BitSieve, Frobenius test, Pollard P-1, ECM, Gerbicz error checking, batch GCD, Aurifeuillean factorizations
- **External tools**: PFGW subprocess, GWNUM FFI (feature-gated), PRST subprocess, FLINT (feature-gated)
- **3-tier verification**: deterministic → BPSW+MR → PFGW cross-check + adaptive methods + content-addressed result hashes
- **AI engine**: OODA loop, 7-component scoring, cost model fitting, EWA weight learning, agent integration
- **ML engine**: Thompson Sampling bandits, GP cost model, BayesOpt, node intelligence, model registry with shadow mode
- **Agent daemon**: 5s/10s tick loop, budget enforcement, task claiming, scheduling, subtask system with cascade completion
- **Dashboard**: 21 pages, 72+ components, 26 hooks, real-time via Supabase + WebSocket
- **Website**: darkreach.ai on Vercel, 23+ pages, MDX blog (3 posts), RSS feed, docs, operator recruitment, live operator stats
- **Distributed network**: PostgreSQL work claiming (`FOR UPDATE SKIP LOCKED`), batch block claiming, trust levels, Docker node image
- **Project campaigns**: TOML-defined, multi-phase orchestration, cost tracking, record comparison, auto-strategy for all 12 forms
- **Operator client**: registration (--name/--forms/--max-cpu), work claiming, startup version check, credit system, public stats API
- **CI/CD**: 10+ CI jobs, Docker builds (coordinator + node images), signed releases (x86_64 + aarch64)
- **Metrics partitioning**: weekly range partitions on `metric_samples`, auto-maintenance (create ahead, drop >90 days)
- **65 PostgreSQL migrations**, 24 DB submodules, 22 API route modules
- **1,111 tests passing** (7 ignored), 509 frontend tests

---

## Master Milestones

### M1: First Discovery (Weeks 1-6)

**Objective:** Harden production, optimize the database, launch first real search campaign, and make darkreach's first prime discovery.

**Outcome:** A provably prime Sierpinski/Riesel number submitted to t5k.org.

| # | Work Item | Domain | Reference | Status |
|---|-----------|--------|-----------|--------|
| 1 | ✅ Configurable connection pool (`DB_MAX_CONNECTIONS` env var, default 5) | Database | [database.md Phase 1](docs/roadmaps/database.md) | Done |
| 2 | ✅ Switch to Supabase Supavisor transaction-mode pooler (port 6543) | Database | [database.md Phase 1](docs/roadmaps/database.md) | Done |
| 3 | ✅ Materialized views for dashboard (`dashboard_stats`, `form_leaderboard_mv`, `digit_distribution_mv`, 5-min refresh) | Database | [database.md Phase 1](docs/roadmaps/database.md) | Done |
| 4 | ✅ Partition `metric_samples` by time (weekly range partitions, auto-drop >90 days) | Database | [database.md Phase 1](docs/roadmaps/database.md) | Done |
| 5 | ✅ Persist actual cost tracking during orchestration (not just estimates) | Projects | [projects.md Tier 1](docs/roadmaps/projects.md) | Done |
| 6 | ✅ Link best prime to project (query largest prime per form) | Projects | [projects.md Tier 1](docs/roadmaps/projects.md) | Done |
| 7 | ✅ Validate phase dependencies at creation (detect cycles, typos) | Projects | [projects.md Tier 1](docs/roadmaps/projects.md) | Done |
| 8 | ✅ Website live data integration: stats bar, status page, leaderboard from `api.darkreach.ai` with graceful fallback | Website | [website.md Phase A](docs/roadmaps/website.md) | Done |
| 9 | ✅ Launch non-base-2 Sierpinski/Riesel campaigns on coordinator ($53/mo AX42) | Research | [research.md Tier 1](docs/roadmaps/research.md) | Done |
| 10 | Cross-verify any discovery with PFGW/PRST on separate hardware, submit to t5k.org + OEIS | Research | [research.md Publication Pipeline](docs/roadmaps/research.md) | Pending |

**Hardware:** $53/mo — 1x Hetzner AX42, 8 cores. Software optimization before hardware scaling.

**Key risks:**
- Supabase connection limits may require self-hosted PG sooner than planned ([database.md Phase 3](docs/roadmaps/database.md))
- Sierpinski/Riesel ROI estimate (1-10 core-years) may be optimistic for base-2 gaps

**Exit criteria:**
- Dashboard p95 < 200ms (materialized views eliminate N+1 queries)
- At least one search campaign running continuously with ML-optimized parameters
- Project cost tracking reflects real work block durations
- First discovery submitted or search range proven to be competitive

---

### M2: Operator Network (Weeks 6-14)

**Objective:** Ship a containerized node that anyone can run. Build the trust system. Get to 20+ external operators contributing compute.

**Outcome:** `docker run ghcr.io/darkreach-ai/darkreach-node` works end-to-end. Operator leaderboard is live.

| # | Work Item | Domain | Reference | Status |
|---|-----------|--------|-----------|--------|
| 1 | ✅ Node Docker image (`Dockerfile.node`), auto-detect cores, multi-arch (amd64 + arm64) | Network | [network.md Phase 2](docs/roadmaps/network.md) | Done |
| 2 | ✅ Docker Compose template: `docker-compose.operator.yml` | Network | [network.md Phase 2](docs/roadmaps/network.md) | Done |
| 3 | ✅ GitHub Actions CI/CD: build + push to GHCR on every release tag | Network | [network.md Phase 2](docs/roadmaps/network.md) | Done |
| 4 | ✅ Operator registration flow: `darkreach register --name --forms kbn,factorial --max-cpu 80` | Network | [network.md Phase 3](docs/roadmaps/network.md) | Done |
| 5 | ✅ Self-updating binary (check `GET /api/releases/worker/latest?channel=stable` on startup) | Network | [network.md Phase 3](docs/roadmaps/network.md) | Done |
| 6 | ✅ Trust progression: 10 valid results → "trusted", double-check mandatory for new nodes | Network | [network.md Phase 3](docs/roadmaps/network.md) | Done |
| 7 | ✅ Operator leaderboard: top contributors by form, by core-hours | Network | [network.md Phase 3](docs/roadmaps/network.md) | Done |
| 8 | ✅ Agent subtask system: tasks spawn child tasks with inheritance/retry | Agents | [agents.md Phase 3](docs/roadmaps/agents.md) | Done |
| 9 | ✅ Task templates: pre-defined workflows (implement-feature, fix-bug, run-search, update-docs) | Agents | [agents.md Phase 3](docs/roadmaps/agents.md) | Done |
| 10 | ✅ MDX blog: real content, syntax highlighting, RSS feed | Website | [website.md Phase B](docs/roadmaps/website.md) | Done |
| 11 | ✅ Website /operators page: live operator count from API, Docker quickstart | Website | [website.md Phase A](docs/roadmaps/website.md) | Done |

**Hardware:** Same $53/mo coordinator. External operators provide the scale.

**Key dependencies:**
- M1 database optimization must land first (operators will 10x write load)
- Trust system requires hash verification (migration 054, already done) and adaptive verification (migration 055, already done)
- Operator leaderboard requires the `/api/leaderboard` endpoint ([server.md](docs/roadmaps/server.md))

**Key risks:**
- Docker image size: GMP + Rust static binary should be <100MB, but GWNUM/FLINT add complexity
- Operator churn: need clear value prop (credits, leaderboard, discoveries attributed)
- Supabase may hit connection ceiling with 20+ operators heartbeating every 30s → may need to pull Redis forward from M3

**Exit criteria:**
- Docker node image published to GHCR, works on Linux/macOS amd64+arm64
- 10+ external operators running with trust levels progressing
- Agent can autonomously create and manage search campaigns via subtasks
- Blog has 3+ real posts, /operators page pulls live data

---

### M3: Intelligence & Autonomy (Weeks 14-22)

**Objective:** Ship the OpenClaw agent team so darkreach largely runs itself. Researcher monitors the landscape, operator watches the fleet, strategist optimizes campaigns. Offload hot-path writes to Redis.

**Outcome:** <1 human intervention per day. Competitive intelligence feeds into AI engine scoring automatically.

| # | Work Item | Domain | Reference | Est. |
|---|-----------|--------|-----------|------|
| 1 | Redis sidecar on coordinator: heartbeats (60s TTL), rate limiting, session cache | Database | [database.md Phase 2](docs/roadmaps/database.md) | 3d |
| 2 | OpenClaw infrastructure: Discord server + bot, 6 channels, SOPS secrets, Docker Compose on coordinator | OpenClaw | [openclaw.md Phase 0](docs/roadmaps/openclaw.md) | 5d |
| 3 | Researcher agent: `darkreach-research` skill (scrape t5k.org, OEIS, mersenneforum via Firecrawl), Sonnet model, $5/day, every 6h | OpenClaw | [openclaw.md Phase 1](docs/roadmaps/openclaw.md) | 5d |
| 4 | Developer agent: `darkreach-search`/`darkreach-developer` skills, kanban integration, PR workflow, Sonnet/Opus, $10/day | OpenClaw | [openclaw.md Phase 2](docs/roadmaps/openclaw.md) | 8d |
| 5 | Operator agent: `darkreach-monitor`/`darkreach-deploy` skills, alert rules (stale node, API errors, disk, stalled search), Haiku, $3/day, every 15min | OpenClaw | [openclaw.md Phase 3](docs/roadmaps/openclaw.md) | 5d |
| 6 | Strategist agent: scoring weight feedback, campaign ROI analysis, weekly reports to #research, Opus, $15/day, weekly | OpenClaw | [openclaw.md Phase 4](docs/roadmaps/openclaw.md) | 4d |
| 7 | Competitive awareness: researcher agent feeds `competitive_intel` table, AI engine adjusts scoring weights | AI Engine | [ai-engine.md Phase 5-6](docs/roadmaps/ai-engine.md) | 3d |
| 8 | Multi-agent pipeline: research finding → strategy decision → development action → operator deployment | OpenClaw | [openclaw.md Phase 5](docs/roadmaps/openclaw.md) | 4d |
| 9 | Automatic context injection: CLAUDE.md + roadmaps + recent git log loaded based on task type | Agents | [agents.md Phase 5](docs/roadmaps/agents.md) | 3d |
| 10 | Model selection: haiku for research, sonnet for implementation, opus for architecture. Auto-escalate on failure. | Agents | [agents.md Phase 4](docs/roadmaps/agents.md) | 2d |
| 11 | Per-phase cost breakdown in project UI + cost estimate preview before launch | Projects | [projects.md Tier 2](docs/roadmaps/projects.md) | 3d |
| 12 | Phase skip/force-complete/retry API endpoints | Projects | [projects.md Tier 2](docs/roadmaps/projects.md) | 2d |

**Hardware:** Add Redis (~$5/mo sidecar or small VPS). Agent costs: ~$33/day for the full OpenClaw team.

**Key dependencies:**
- M2 operator network must be running (operator agent needs nodes to monitor)
- Researcher agent needs Firecrawl API key + Discord bot token (SOPS encrypted)
- Redis offload critical before M4 node scaling (heartbeats at 20+ nodes saturate PG write path)

**Key risks:**
- Agent cost: $33/day = ~$1,000/mo. Must demonstrate ROI through fewer human interventions and better campaign selection
- Discord bot reliability: need exponential backoff, reconnect logic, message dedup
- Researcher scraping: t5k.org and mersenneforum may rate-limit or change HTML structure
- Multi-agent coordination failures: researcher posts finding, but strategist doesn't pick it up → need explicit handoff protocol

**Exit criteria:**
- All 4 OpenClaw agents running autonomously (researcher, developer, operator, strategist)
- <1 human intervention per day for routine operations
- Competitive intelligence table populated with data from t5k.org/OEIS
- Redis handling heartbeat writes (PG write load reduced ~90%)
- At least one multi-agent pipeline executed end-to-end (research → strategy → action)

---

### M4: Verification & Classification (Weeks 22-30)

**Objective:** Build a rigorous verification system with proof certificates, form-specific invariants, and cross-form discovery. Self-host PostgreSQL for full control. Target: every result is independently verified before announcement.

**Outcome:** Verification pipeline produces cryptographic certificates. Primes are automatically classified across multiple forms.

| # | Work Item | Domain | Reference | Est. |
|---|-----------|--------|-----------|------|
| 1 | Migrate from Supabase to self-hosted PostgreSQL on Hetzner CX32 (4 vCPU, 8GB, ~$15/mo) | Database | [database.md Phase 3](docs/roadmaps/database.md) | 5d |
| 2 | TimescaleDB hypertables for `system_metrics` + `system_logs` with automatic compression | Database | [database.md Phase 3](docs/roadmaps/database.md) | 3d |
| 3 | Continuous aggregates to replace manual rollup functions | Database | [database.md Phase 3](docs/roadmaps/database.md) | 2d |
| 4 | Full `postgresql.conf` tuning (shared_buffers, effective_cache_size, autovacuum) | Database | [database.md Phase 3](docs/roadmaps/database.md) | 1d |
| 5 | Proth/LLR witness certificates (10-20% replay cost) | Network | [network.md Phase 4](docs/roadmaps/network.md) | 5d |
| 6 | Pietrzak VDF proofs for PRP results (~1-5% overhead, 1% verification cost) | Network | [network.md Phase 4](docs/roadmaps/network.md) | 5d |
| 7 | Cross-software verification pipeline: PFGW + PRST + OpenPFGW on independent hardware | Network | [network.md Phase 4](docs/roadmaps/network.md) | 3d |
| 8 | Form-specific invariant validation: twin p+/-2, Sophie Germain 2p+1, palindrome structure, Cullen/Woodall form | Verification | [verification.md Phase 3](docs/roadmaps/verification.md) | 5d |
| 9 | Cross-form discovery: post-verification scan for secondary form memberships, auto-tag | Verification | [verification.md Phase 5](docs/roadmaps/verification.md) | 4d |
| 10 | Multi-stage work pipeline: sieve → P-1 → screen → test → prove as separate queue stages | Network | [network.md Phase 5](docs/roadmaps/network.md) | 8d |
| 11 | Stage-aware block claiming: nodes specialize (fast-sieve, heavy-test, GPU) | Network | [network.md Phase 5](docs/roadmaps/network.md) | 3d |
| 12 | Candidate queue between stages: "sieve once, test many" architecture | Server | [server.md Work Pipeline](docs/roadmaps/server.md) | 3d |
| 13 | Streaming replication to CX22 (read-only replica), dashboard queries routed to replica | Database | [database.md Phase 4](docs/roadmaps/database.md) | 3d |

**Hardware:** Add CX32 for self-hosted PG (~$15/mo). Coordinator now handles API + agents + Redis. Total: ~$70/mo + agent costs.

**Key dependencies:**
- M3 Redis offload must be complete (migration from Supabase involves downtime)
- Proof certificates require `src/certificate.rs` (exists) to emit structured data that survives network transit
- Multi-stage pipeline requires `src/pipeline.rs` (exists) to route candidates between stages
- Stage specialization requires `src/resource.rs` (exists) for capability-aware routing

**Key risks:**
- Supabase → self-hosted migration: data migration window, potential Auth disruption (Supabase Auth still needed?)
- VDF proof overhead may be higher than 5% for small candidates → need size threshold
- Multi-stage pipeline adds latency per candidate; must measure throughput vs. single-stage baseline
- Read replica lag may cause stale dashboard reads during high-throughput campaigns

**Exit criteria:**
- Self-hosted PostgreSQL running with TimescaleDB, <50ms dashboard p95 latency
- Every prime discovery produces a verification certificate (witness or VDF proof)
- Cross-form discovery detects at least palindromic + twin memberships automatically
- Multi-stage pipeline running: sieve results queue into P-1 filter, then testing, then proof
- Read replica handling all dashboard queries

---

### M5: Scale & Hardware (Weeks 30-42)

**Objective:** Scale hardware (GWNUM unlocks 50-100x speedup), target Wagstaff PRP and palindromic records, launch GPU sieving, prove the platform works at 100+ operators.

**Outcome:** A world-record-sized PRP or provably prime discovery. GPU-accelerated sieving operational.

| # | Work Item | Domain | Reference | Est. |
|---|-----------|--------|-----------|------|
| 1 | Add 1-2x AX162-S nodes (48 cores each, $215-430/mo) | Ops | [ops.md Phase 2](docs/roadmaps/ops.md) | 2d |
| 2 | Launch Wagstaff PRP campaign: (2^p+1)/3 with multiplicative order sieve + GWNUM acceleration | Research | [research.md Tier 2](docs/roadmaps/research.md) | 3d |
| 3 | Launch palindromic record campaign: BLS N+1 proofs, target current record gap | Research | [research.md Tier 2](docs/roadmaps/research.md) | 3d |
| 4 | GPU resource model: `GpuRuntime`, `StorageRole`, `NetworkRole` enums in `src/resource.rs` | Resources | [resources.md Phase 1](docs/roadmaps/resources.md) | 3d |
| 5 | GPU auto-detection: nvidia-smi, HIP, Metal. `ResourceCapabilities` JSONB in `operator_nodes` | Resources | [resources.md Phase 1](docs/roadmaps/resources.md) | 2d |
| 6 | GPU-aware work routing: `claim_operator_block()` matches GPU requirements to node capabilities | Resources | [resources.md Phase 2](docs/roadmaps/resources.md) | 3d |
| 7 | AI engine GPU awareness: add `gpu_fit` + `storage_fit` to scoring model (9-component → 11-component) | Resources | [resources.md Phase 3](docs/roadmaps/resources.md) | 3d |
| 8 | OpenCL/CUDA sieve kernels (mtsieve integration or custom), 10-100x sieve speedup | Engine | [engine.md Phase 7](docs/roadmaps/engine.md) | 10d |
| 9 | GPU node Docker image: NVIDIA runtime, CUDA toolkit, sieve + FFT libraries | Network | [network.md Phase 7](docs/roadmaps/network.md) | 3d |
| 10 | HA failover: Patroni/repmgr for automatic PostgreSQL failover, PgBouncer connection multiplexing | Database | [database.md Phase 5](docs/roadmaps/database.md) | 5d |
| 11 | Adaptive phase generation in projects: auto-generate follow-up phases based on intermediate results | Projects | [projects.md Tier 3](docs/roadmaps/projects.md) | 4d |
| 12 | Frontend: live cost graph (time-series), record comparison deep-dive (gap to record, ETA), phase dependency visualization | Frontend | [frontend.md Phase 6 + projects.md Tier 4](docs/roadmaps/frontend.md) | 5d |

**Hardware:** $215-430/mo for AX162-S nodes + existing infrastructure. GPU nodes contributed by operators or rented.

**Key dependencies:**
- M4 multi-stage pipeline enables GPU sieving as a separate stage (sieve on GPU, test on CPU)
- M4 self-hosted PG + HA required before 100+ operators (Supabase limits won't work)
- Wagstaff campaign requires GWNUM FFI working in production (feature-gated, needs testing at scale)
- Palindromic record requires deep sieve + BLS proofs (both implemented, need campaign orchestration)

**Key risks:**
- GWNUM on Hetzner AMD CPUs may have different performance profile than Intel benchmarks
- Palindromic record attempt may take 100-1,000 core-years → need to validate feasibility before committing hardware budget
- GPU sieving kernel development is complex; mtsieve integration may be faster than custom CUDA
- HA failover adds operational complexity; Patroni requires etcd/consul cluster

**Exit criteria:**
- At least one campaign targeting a world record actively running with projected ETA
- GPU sieving operational (10x+ speedup measured on at least one form)
- 100+ operators with functioning trust progression and leaderboard
- PostgreSQL HA with automatic failover tested (simulated primary failure + recovery)
- AI engine scoring includes GPU-fit and produces measurably better campaign allocation

---

### M6: WASM & Browser Compute (Weeks 42-56)

**Objective:** Compile the engine to WebAssembly. Ship browser-based contribution. Anyone can contribute compute by visiting `darkreach.ai/contribute` — zero install.

**Outcome:** Browser-based prime hunting works. Content-addressed results enable trustless verification.

| # | Work Item | Domain | Reference | Est. |
|---|-----------|--------|-----------|------|
| 1 | Compile core engine to `wasm32-wasi` (sieve + test logic, no PFGW/GWNUM) | Engine | [engine.md Phase 6](docs/roadmaps/engine.md) | 8d |
| 2 | Pure-Rust bignum fallback (`num-bigint` or `crypto-bigint`) for WASM builds, feature-gated | Engine | [engine.md Phase 6](docs/roadmaps/engine.md) | 5d |
| 3 | Content-addressed I/O: `hash(WASM_module) + hash(input) → hash(output)`, SHA-256 | Engine | [technology-vision.md Phase 1](docs/roadmaps/technology-vision.md) | 3d |
| 4 | Wasmtime runtime embedded in server alongside native Rust | Server | [server.md Wasmtime](docs/roadmaps/server.md) | 5d |
| 5 | WASI sandboxing: filesystem, network isolation, fuel-based CPU metering | Server | [technology-vision.md Phase 1](docs/roadmaps/technology-vision.md) | 3d |
| 6 | WASM module registry: OCI-compatible, Ed25519 signed, content-addressed | Ops | [ops.md WASM Pipeline](docs/roadmaps/ops.md) | 4d |
| 7 | CI/CD: compile to `wasm32-wasi` alongside native targets on every release | Ops | [ops.md WASM Pipeline](docs/roadmaps/ops.md) | 2d |
| 8 | `/contribute` page: WASM module in Web Worker, contribution meter, gamification | Frontend | [frontend.md Phase 11](docs/roadmaps/frontend.md) | 5d |
| 9 | WebGPU compute shaders for browser-based sieving (where available) | Frontend | [frontend.md Phase 11](docs/roadmaps/frontend.md) | 8d |
| 10 | Battery-aware throttling: reduce work on mobile/battery, respect visibility API | Frontend | [resources.md Phase 8](docs/roadmaps/resources.md) | 2d |
| 11 | Hash-addressed sieve blob storage: shared sieves across nodes, `PUT /api/v1/sieve/{form}/{hash}` | Resources | [resources.md Phase 4](docs/roadmaps/resources.md) | 4d |
| 12 | Merkle DAG audit: every result linked in content-addressed chain, auditable by anyone | Server | [technology-vision.md Phase 1](docs/roadmaps/technology-vision.md) | 4d |

**Hardware:** No new hardware. Browser operators donate compute. WASM adds distribution channel.

**Key dependencies:**
- M5 GPU sieving validates that stage-separated pipeline works (browser does sieve, server does heavy test)
- Content-addressed I/O from M4 hash verification (migrations 053-055) provides foundation
- Pure-Rust bignum is significantly slower than GMP (~10-50x) — browser contribute is for lightweight sieve work, not full primality testing
- WebGPU availability is limited (Chrome 113+, no Firefox/Safari as of early 2026)

**Key risks:**
- WASM performance: `num-bigint` is ~20-50x slower than GMP for large integers → browser nodes only viable for sieving, not testing
- WebGPU API is unstable and browser coverage is low → fallback to CPU-only Web Worker
- Content-addressed I/O requires deterministic computation (floating-point differences between platforms)
- Module signing: need key management, rotation policy, revocation for compromised keys

**Exit criteria:**
- `darkreach.ai/contribute` works: visitor runs sieve jobs in browser, results submitted and verified
- WASM module published to registry, signed, content-addressed
- Wasmtime runtime in server can execute same WASM module that browsers run
- At least 100 browser-contributed results verified and merged
- Merkle DAG contains auditable chain of all browser-contributed results

---

### M7: Platform & Revenue (Weeks 56-78)

**Objective:** Open darkreach to general-purpose verified computation. Land first research partnerships. Build the marketplace. Start generating revenue to sustain the network.

**Outcome:** Non-prime workloads running on the network. First paying customer or research partnership signed.

| # | Work Item | Domain | Reference | Est. |
|---|-----------|--------|-----------|------|
| 1 | Containerized work blocks: gVisor/Firecracker sandbox for arbitrary WASM workloads | Network | [network.md Phase 8](docs/roadmaps/network.md) | 10d |
| 2 | Task DAGs with dependencies: multi-step compute workflows (map → reduce → aggregate) | Network | [network.md Phase 8](docs/roadmaps/network.md) | 5d |
| 3 | Operator opt-in for non-prime workloads (per-category: bio, climate, crypto, ML) | Network | [network.md Phase 8](docs/roadmaps/network.md) | 3d |
| 4 | Compute marketplace UI: capacity overview, pricing, problem domains | Frontend | [frontend.md Phase 10](docs/roadmaps/frontend.md) | 5d |
| 5 | Job submission portal: upload container image, define I/O schema, set budget | Frontend | [frontend.md Phase 10](docs/roadmaps/frontend.md) | 5d |
| 6 | Operator earnings dashboard: lifetime earnings, pending payouts, monthly trend | Frontend | [frontend.md Phase 10](docs/roadmaps/frontend.md) | 3d |
| 7 | Revenue-tracked work blocks: per-block cost, operator credit, platform fee | Resources | [resources.md Phase 7](docs/roadmaps/resources.md) | 4d |
| 8 | Operator earnings ledger + payout system (monthly, Stripe Connect) | Resources | [resources.md Phase 7](docs/roadmaps/resources.md) | 5d |
| 9 | Trust-gated compute tiers: Level 1 (new) → Level 4 (enterprise, TEE-attested) | Resources | [resources.md Phase 7](docs/roadmaps/resources.md) | 3d |
| 10 | AI engine: 11-component scoring (+revenue_potential, deadline_pressure, verification_depth, fairness_balance) | AI Engine | [path-c.md AI Engine Evolution](docs/roadmaps/path-c.md) | 4d |
| 11 | Scheduling priority: enterprise (highest) → priority → open science (20% minimum) → prime hunting | AI Engine | [path-c.md Scheduling](docs/roadmaps/path-c.md) | 3d |
| 12 | Land 2-3 research partnerships (Folding@home, Rosetta, climate modeling) | Research | [path-c.md Phase 2](docs/roadmaps/path-c.md) | ongoing |
| 13 | Python SDK for job submission (pip install darkreach) | Path C | [path-c.md Phase 4](docs/roadmaps/path-c.md) | 5d |
| 14 | Fairness enforcement: minimum 20% of compute reserved for open science at all times | Path C | [path-c.md](docs/roadmaps/path-c.md) | 2d |

**Hardware:** Operators provide capacity. Platform takes 30-40% fee. Target $30K/mo revenue by end of M7.

**Key dependencies:**
- M6 WASM runtime is prerequisite for general compute workloads
- M5 HA PostgreSQL required (paying customers need SLAs)
- M4 proof certificates and verification pipeline provide the "verified computation" USP
- Research partnerships require proof that prime network works at scale (M5 exit criteria)

**Key risks:**
- Market timing: enterprise distributed compute is competitive (cloud, BOINC, Golem, Render)
- darkreach's edge is **verification quality** and **trust system** — must be demonstrably better than alternatives
- Operator payout system requires financial/legal compliance (money transmitter regulations vary by jurisdiction)
- Fairness enforcement may conflict with revenue maximization during peak demand

**Exit criteria:**
- At least one non-prime workload running on the network (research partnership)
- Marketplace UI live with capacity/pricing display
- Operator earnings tracked and at least one test payout executed
- Python SDK published on PyPI
- Open science minimum enforced by AI engine scheduler

---

### M8: Protocol Decentralization (Month 18+)

**Objective:** Transform darkreach from a centralized service into a decentralized computation protocol. The coordinator becomes a supernode. The network can survive without any single entity.

**Outcome:** darkreach is a protocol, not just a product.

| # | Work Item | Domain | Reference | Est. |
|---|-----------|--------|-----------|------|
| 1 | libp2p integration: Kademlia DHT for operator discovery, GossipSub for job announcements | Server | [technology-vision.md Phase 2 Q1](docs/roadmaps/technology-vision.md) | 15d |
| 2 | Heartbeats via gossip (no central coordinator dependency for liveness) | Network | [technology-vision.md Phase 2](docs/roadmaps/technology-vision.md) | 5d |
| 3 | Coordinator becomes supernode: SLA enforcement, billing, trust oracle — not required for basic operation | Server | [technology-vision.md Phase 2](docs/roadmaps/technology-vision.md) | 5d |
| 4 | W3C Verifiable Credentials for operator trust (DID-based identity, portable, self-sovereign) | Network | [technology-vision.md Phase 2 Q2](docs/roadmaps/technology-vision.md) | 8d |
| 5 | ZK trust proofs: privacy-preserving verification (prove trust level without revealing history) | Network | [technology-vision.md Phase 3 Q3](docs/roadmaps/technology-vision.md) | 10d |
| 6 | TEE attestation: Intel TDX / AMD SEV-SNP support for enterprise confidential compute | Ops | [technology-vision.md Phase 3 Q1](docs/roadmaps/technology-vision.md) | 8d |
| 7 | CRDT-based state: coordinator is no longer single source of truth, state converges via CRDTs | Server | [technology-vision.md Phase 3 Q2](docs/roadmaps/technology-vision.md) | 15d |
| 8 | Multi-objective RL scheduling: replace linear 7-component scoring with learned policy | AI Engine | [technology-vision.md Phase 2 Q4](docs/roadmaps/technology-vision.md) | 10d |
| 9 | Decentralized network visualization: libp2p topology, peer connections, gossip propagation | Frontend | [frontend.md Phase 11](docs/roadmaps/frontend.md) | 5d |
| 10 | Trust credential wallet in dashboard: operators manage their W3C VCs | Frontend | [frontend.md Phase 11](docs/roadmaps/frontend.md) | 4d |

**This is the long-term vision.** Timelines are approximate. Each item is a substantial engineering effort.

**Strategic rationale (from [technology-vision.md](docs/roadmaps/technology-vision.md)):**
- Cloud providers won't build this (cannibalizes margins)
- Trust is compounding (operators earn more as history grows, can't be replicated)
- Community identity creates altruism moat
- Protocol architecture enables the network to outlive any single company

---

## Strategic Targets by Milestone

| Target | ROI (core-years) | Provable? | Competition | Milestone |
|--------|-------------------|-----------|-------------|-----------|
| Sierpinski/Riesel (non-base-2) | **1-10** | Yes | Low | **M1** |
| Palindromic record | 100-1,000 | Yes (BLS) | 1 team | M5 |
| Wagstaff PRP | ~3,000 | No | None | M5 |
| Factorial | ~2,300 | Yes | PrimeGrid | M5+ |

See [research.md](docs/roadmaps/research.md) for full analysis including twin primes, multi-factorial, and long-range targets.

---

## Hardware Scaling by Milestone

| Milestone | Hardware | Monthly Cost | Notes |
|-----------|----------|-------------|-------|
| M1-M2 | 1x AX42 (8 cores) | $53 | Software optimization first |
| M3 | + Redis sidecar | $58 | Agent costs add ~$1,000/mo |
| M4 | + CX32 self-hosted PG | $73 | Full PostgreSQL control |
| M5 | + 1-2x AX162-S (48 cores) | $290-500 | GWNUM justifies hardware spend |
| M6 | No new hardware | Same | Browser operators add capacity |
| M7 | Operator-provided | Revenue-funded | Platform fee covers costs |
| M8 | Decentralized | Self-sustaining | Protocol runs independently |

---

## Cross-Cutting Concerns

### Safety & Correctness
Every milestone builds on verification: Gerbicz error checking (done) → content-addressed hashes (done) → proof certificates (M4) → cross-software verification (M4) → VDF proofs (M4) → ZK trust proofs (M8). See [verification.md](docs/roadmaps/verification.md).

### AI as Core Product
The AI engine evolves from 7-component linear scoring (done) → ML engine with Thompson Sampling + GP cost model (done) → competitive awareness via researcher agent (M3) → GPU-aware 11-component scoring (M5) → Path C revenue-aware scheduling (M7) → multi-objective RL policy (M8). See [ai-engine.md](docs/roadmaps/ai-engine.md).

### Trust is the Moat
Trust builds through: basic heartbeat (done) → trust progression levels (M2) → proof certificates (M4) → trust-gated compute tiers (M7) → W3C Verifiable Credentials (M8). See [network.md](docs/roadmaps/network.md) and [technology-vision.md](docs/roadmaps/technology-vision.md).

### Protocol Before Platform
Architecture evolves from: centralized coordinator (done) → PG + Redis (M3) → self-hosted PG + HA (M4-M5) → WASM execution (M6) → libp2p gossip layer (M8) → CRDT state convergence (M8) → full protocol (M8). See [technology-vision.md](docs/roadmaps/technology-vision.md).
