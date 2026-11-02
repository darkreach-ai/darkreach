# Architecture

> *Technical design for the darkreach prime discovery platform.*

---

## Table of Contents

- [Overview](#overview)
- [Design Principles](#design-principles)
- [Tech Stack](#tech-stack)
- [System Architecture](#system-architecture)
- [Data Flow](#data-flow)
- [API Design](#api-design)
- [Database Schema](#database-schema)
- [AI Engine](#ai-engine)
- [Distributed Coordination](#distributed-coordination)
- [Security](#security)
- [Performance](#performance)

---

## Overview

darkreach is a distributed platform for hunting special-form prime numbers. It consists of:

- An **engine** that implements 12 search forms with sieve-then-test pipelines
- An **API server** (22 route modules) that coordinates distributed work via PostgreSQL (64 migrations, 24 DB submodules)
- A **dashboard** that provides real-time monitoring and search management
- An **AI engine** that autonomously designs search campaigns (OODA loop + 7-component scoring)
- An **ML engine** for data-driven optimization (Thompson Sampling, GP cost prediction, Bayesian search optimization, node intelligence)
- A **network** of compute nodes that claim work, execute searches, and report results
- A **public website** at darkreach.ai (deployed to Vercel, ~22 pages)

### Key Differentiators

| Feature | Traditional (GIMPS/PrimeGrid) | darkreach |
|---------|-------------------------------|-----------|
| Coordination | Custom server / BOINC | PostgreSQL (`FOR UPDATE SKIP LOCKED`) |
| Strategy | Human-managed | AI OODA loop with learned scoring weights |
| Forms | 1-10 per project | 12 in one platform |
| UX | Static HTML / BOINC client | Next.js dashboard + WebSocket |
| Verification | Quorum / Pietrzak proofs | 3-tier pipeline (deterministic + BPSW+MR + PFGW) |
| Proofs | Specialized per project | 6 proof systems (Proth, LLR, Pocklington, Morrison, BLS, Pepin) |

### Target Users

- **Researchers**: Full control over search parameters, proof generation, publication pipeline
- **Operators**: Easy setup, auto-updating binary, contribution tracking
- **Admins**: Dashboard for fleet monitoring, search management, AI oversight

---

## Design Principles

### 1. Proofs, not PRPs

t5k.org rejects probable primes. Every search form must produce deterministic proofs where mathematically possible. For forms without proofs (Wagstaff), results are clearly marked as PRP. The verification pipeline ensures no unproven result is published as proven.

### 2. Sieve-then-test pipeline

Every search form follows the same pattern:

```
Generate candidates
    → Sieve (eliminate composites cheaply)
    → P-1 factoring (eliminate more composites)
    → Quick MR screen (2-round, eliminate 99.9%)
    → Full primality test (expensive, form-specific)
    → Deterministic proof (form-specific)
    → Verify (3-tier pipeline)
    → Log to PostgreSQL
```

This pipeline maximizes the ratio of compute spent on actual primality testing vs. wasted on composites.

### 3. PostgreSQL as coordination bus

No message queues, no Redis, no in-memory coordination state. PostgreSQL is the single source of truth for:
- Which work blocks exist and their status
- Which nodes are online (heartbeat timestamps)
- Which searches are running
- AI engine state and decisions
- Prime records and verification status

Nodes claim work via `FOR UPDATE SKIP LOCKED`. The dashboard reads from PG. If the dashboard restarts, nothing is lost.

### 4. Static frontend

The Next.js dashboard is built as a static export and served by the Axum backend. No server-side rendering, no Node.js in production. The API server is a single Rust binary that serves both the REST API and the static frontend files.

### 5. Checkpoint everything

Searches checkpoint to JSON every 60 seconds. Atomic file rename prevents corruption. All 12 forms have checkpoint variants in the `CheckpointData` enum. A crash loses at most 60 seconds of work.

### 6. AI-first orchestration

The AI engine runs a continuous OODA loop (Observe-Orient-Decide-Act-Learn). It assembles a `WorldSnapshot` from parallel DB queries, scores all possible actions using a 7-component model, and executes the highest-confidence decision. Humans override via the dashboard, not by writing scripts.

---

## Tech Stack

### Engine

| Technology | Purpose | Rationale |
|-----------|---------|-----------|
| Rust | Language | Memory safety without GC, zero-cost abstractions, excellent for CPU-bound work |
| rug (GMP) | Arbitrary-precision arithmetic | Fastest bigint library. No Rust alternative comes close for million-digit numbers. |
| rayon | Parallelism | Data-parallel iterators for batch primality testing |
| PFGW | External primality tester | 50-100x acceleration for large candidates via FFT-based modular exponentiation |
| PRST | External k*b^n+/-1 tester | Gerbicz error checking, proof generation, GWNUM backend |
| GWNUM | FFT multiplication (feature-gated) | IBDWT for Mersenne-form, multi-threaded FFT for general |
| FLINT | Number theory library (feature-gated) | NTT, polynomial arithmetic, ECM |
| mtsieve | Sieving framework | GPU-accelerated BSGS sieving for 6 forms |

### Server

| Technology | Purpose | Rationale |
|-----------|---------|-----------|
| Axum 0.8 | HTTP server | Fast, Tokio-native, tower middleware, WebSocket support |
| Tokio | Async runtime | Industry standard for async Rust |
| sqlx 0.8 | Database driver | Compile-time checked queries, async PostgreSQL |
| clap 4 | CLI parsing | Derive macro for 12 subcommands + global flags |
| serde / serde_json | Serialization | Checkpoints, API responses, configuration |
| ureq 3 | Blocking HTTP client | Worker-to-coordinator communication (runs in rayon threads) |
| mimalloc | Allocator | 5-10% performance improvement over system allocator |

### Frontend

| Technology | Purpose | Rationale |
|-----------|---------|-----------|
| Next.js 16 | React framework | App Router, static export, good DX |
| React 19 | UI library | Component model, hooks, concurrent features |
| Tailwind 4 | Styling | Utility-first, no CSS-in-JS runtime |
| shadcn/ui | Component library | Accessible, customizable, not a dependency |
| Recharts | Charts | Declarative, React-native charting |
| Supabase JS | Auth + Realtime | JWT auth, real-time prime notifications |

### Infrastructure

| Technology | Purpose | Rationale |
|-----------|---------|-----------|
| PostgreSQL 15 | Database | ACID, `FOR UPDATE SKIP LOCKED`, JSONB, GIN indexes |
| Nginx | Reverse proxy | Rate limiting, WebSocket upgrade, static caching, TLS |
| systemd | Service management | Auto-restart, security hardening, resource limits |
| Docker | Containerization | Reproducible builds, multi-arch images |
| Prometheus | Metrics | Pull-based, PromQL, industry standard |
| Grafana | Dashboards | Visualization, alerting, free tier |
| SOPS + age | Secrets | Encrypted in-repo, no external secret manager |

---

## System Architecture

```
                    ┌─────────────────────────────────────────────┐
                    │              darkreach Platform              │
                    │                                             │
                    │  ┌────────┐  ┌──────────┐  ┌────────────┐  │
                    │  │ Static │  │ REST API │  │ WebSocket  │  │
     Browser ──────┼─▶│Frontend│  │ (Axum)   │  │ (push 2s)  │  │
                    │  │(Next.js│  │ 22 route │  │            │  │
                    │  │ export)│  │ modules  │  │            │  │
                    │  └────────┘  └────┬─────┘  └─────┬──────┘  │
                    │                   │              │          │
                    │         ┌─────────┴──────────────┘          │
                    │         │                                   │
                    │         ▼                                   │
                    │  ┌──────────────┐   ┌──────────────┐       │
                    │  │   AppState   │   │  AI Engine   │       │
                    │  │              │   │  (OODA loop) │       │
                    │  │ - db pool    │   │  - scoring   │       │
                    │  │ - fleet      │   │  - cost model│       │
                    │  │ - events     │   │  - decisions │       │
                    │  │ - metrics    │   │              │       │
                    │  └──────┬───────┘   └──────┬───────┘       │
                    │         │                  │               │
                    │         └────────┬─────────┘               │
                    │                  │                         │
                    │                  ▼                         │
                    │           ┌──────────────┐                │
                    │           │  PostgreSQL   │                │
                    │           │  (Supabase)   │                │
                    │           │               │                │
                    │           │ - primes      │                │
                    │           │ - search_jobs │                │
                    │           │ - work_blocks │                │
                    │           │ - workers     │                │
                    │           │ - ai_engine_* │                │
                    │           │ - agent_tasks │                │
                    │           │ - projects    │                │
                    │           │ - operators   │                │
                    │           └──────┬────────┘                │
                    │                  │                         │
                    └──────────────────┼─────────────────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │                  │                  │
                    ▼                  ▼                  ▼
              ┌──────────┐      ┌──────────┐      ┌──────────┐
              │  Node 1  │      │  Node 2  │      │  Node N  │
              │          │      │          │      │          │
              │ ┌──────┐ │      │ ┌──────┐ │      │ ┌──────┐ │
              │ │Sieve │ │      │ │Sieve │ │      │ │Sieve │ │
              │ └──┬───┘ │      │ └──┬───┘ │      │ └──┬───┘ │
              │    ▼     │      │    ▼     │      │    ▼     │
              │ ┌──────┐ │      │ ┌──────┐ │      │ ┌──────┐ │
              │ │ Test │ │      │ │ Test │ │      │ │ Test │ │
              │ └──┬───┘ │      │ └──┬───┘ │      │ └──┬───┘ │
              │    ▼     │      │    ▼     │      │    ▼     │
              │ ┌──────┐ │      │ ┌──────┐ │      │ ┌──────┐ │
              │ │Proof │ │      │ │Proof │ │      │ │Proof │ │
              │ └──────┘ │      │ └──────┘ │      │ └──────┘ │
              └──────────┘      └──────────┘      └──────────┘
```

---

## Data Flow

### Candidate Pipeline

```
1. GENERATE     Form-specific candidate generation
                (e.g., k*b^n+1 for k=3, base=2, n=1000..2000)
                    │
2. SIEVE        Sieve of Eratosthenes + wheel factorization
                Eliminates ~95% of candidates
                    │
3. P-1 FILTER   Pollard P-1 factoring (adaptive B1/B2)
                Eliminates ~5-10% more
                    │
4. MR SCREEN    2-round Miller-Rabin pre-screening
                Eliminates 99.9% of remaining composites
                    │
5. FROBENIUS    Grantham RQFT (for candidates > 10K bits)
                Additional composite detection
                    │
6. FULL TEST    Form-specific primality test
                - Proth test (k*b^n+1 where k < 2^n)
                - LLR test (k*b^n-1)
                - Pepin test (GFN)
                - PFGW subprocess (large candidates)
                    │
7. PROOF        Deterministic proof generation
                - Pocklington (N-1 partially factored)
                - Morrison (N+1 partially factored)
                - BLS (combination of N-1 and N+1)
                - Pepin (GFN-specific)
                    │
8. VERIFY       3-tier verification pipeline
                Tier 1: Deterministic proof verification
                Tier 2: BPSW + 25-round MR
                Tier 3: PFGW cross-check
                    │
9. CLASSIFY     Tag with form, proof method, properties
                    │
10. LOG         INSERT into PostgreSQL (primes table)
                Broadcast via WebSocket + Supabase Realtime
```

### Distributed Work Flow

```
Admin/AI creates search job
    │
    ▼
Search job row in PostgreSQL
    status: 'active'
    form: 'kbn'
    parameters: { k: 3, base: 2, min_n: 1000, max_n: 100000 }
    │
    ▼
Block generator creates work_blocks rows
    Each block: { job_id, block_number, range_start, range_end, status: 'pending' }
    │
    ▼
Node claims block
    SELECT ... FROM work_blocks WHERE status='pending'
    FOR UPDATE SKIP LOCKED LIMIT 1
    UPDATE work_blocks SET status='claimed', worker_id=...
    │
    ▼
Node executes search for block range
    Sieve → Test → Proof → Log
    Heartbeat every 10s (updates last_heartbeat)
    │
    ▼
Node reports completion
    UPDATE work_blocks SET status='completed', tested=N, found=M
    │
    ▼
When all blocks completed → job status = 'completed'
```

---

## API Design

### REST Routes

| Module | Base Path | Key Endpoints |
|--------|-----------|--------------|
| Health | `/api/health` | `GET` — health check, readiness |
| Status | `/api/status` | `GET` — coordinator summary (workers, jobs, primes) |
| Primes | `/api/primes` | `GET` — query/filter, `GET /:id` — detail, `POST /:id/verify` |
| Search Jobs | `/api/search_jobs` | `GET` — list, `POST` — create, `PATCH /:id` — update status |
| Searches | `/api/searches` | `POST` — create search (all 12 forms), `POST /:id/stop` |
| Workers | `/api/workers` | `GET` — list, `POST /heartbeat` — register/update |
| Fleet | `/api/fleet` | `GET` — overview (workers + searches combined) |
| Operators | `/api/v1/operators` | `POST /register`, `GET /me`, `POST /me/rotate-key` |
| Nodes | `/api/v1/nodes` | `GET` — list, `GET /:id` — detail |
| Projects | `/api/projects` | `GET`, `POST`, `PATCH /:id`, `GET /:id/events` |
| Agents | `/api/agents` | Tasks, budgets, memory, roles, schedules |
| Releases | `/api/releases` | `GET` — channels, `GET /latest` — latest version |
| Observability | `/api/observability` | Metrics, logs, charts, worker rates |
| Docs | `/api/docs` | `GET` — documentation list, `GET /:slug` — content |
| Notifications | `/api/notifications` | Push notification management |
| Verify | `/api/verify` | `POST` — re-verify a prime |
| Sieve | `/api/sieve` | Sieve configuration and status |
| Prime Verification | `/api/prime-verification` | Verification queue management |
| Strategy | `/api/strategy` | AI strategy engine configuration |
| Audit | `/api/audit` | Audit log queries |
| Contribute | `/api/contribute` | Browser-based operator contribution |
| Resources | `/api/resources` | Resource pool management |
| Schedules | `/api/schedules` | Scheduled automation management |
| Auth | `/api/auth` | Authentication endpoints |

### WebSocket Protocol

Endpoint: `/ws`

Push interval: 2 seconds

Message types:
```json
{ "type": "prime_found", "data": { "expression": "3*2^1000+1", "digits": 302, "form": "kbn" } }
{ "type": "search_status", "data": { "job_id": "...", "tested": 50000, "found": 3 } }
{ "type": "worker_status", "data": { "online": 5, "active": 4, "idle": 1 } }
{ "type": "ai_decision", "data": { "action": "start_search", "form": "factorial", "confidence": 0.85 } }
```

### Authentication

- **Dashboard:** Supabase JWT in `Authorization: Bearer <token>` header
- **API:** Same JWT, verified in Axum middleware (`middleware_auth.rs`)
- **Nodes:** Database connection string (`DATABASE_URL`) — PostgreSQL handles auth
- **Public endpoints:** `/api/health`, `/api/status`, `/api/primes` (read-only)

---

## Database Schema

64 migrations in `supabase/migrations/`. Core tables:

### Prime Records

```sql
primes (
    id              UUID PRIMARY KEY,
    expression      TEXT NOT NULL,           -- "3*2^1000+1"
    form            TEXT NOT NULL,           -- "kbn", "factorial", etc.
    digits          INTEGER NOT NULL,
    proof_method    TEXT,                    -- "proth", "pocklington", "prp", etc.
    params          JSONB,                   -- Form-specific parameters
    certificate     JSONB,                   -- PrimalityCertificate
    tags            TEXT[],                  -- GIN-indexed classification tags
    verified        BOOLEAN DEFAULT false,
    verification_level TEXT,                 -- "deterministic", "probabilistic"
    discovered_at   TIMESTAMPTZ DEFAULT now(),
    discovered_by   TEXT,                    -- Node/operator that found it
    search_job_id   UUID REFERENCES search_jobs(id)
)
```

### Search Coordination

```sql
search_jobs (
    id              UUID PRIMARY KEY,
    form            TEXT NOT NULL,
    parameters      JSONB NOT NULL,
    status          TEXT DEFAULT 'pending',  -- pending, active, paused, completed, failed
    total_blocks    INTEGER,
    completed_blocks INTEGER DEFAULT 0,
    primes_found    INTEGER DEFAULT 0,
    created_at      TIMESTAMPTZ DEFAULT now()
)

work_blocks (
    id              UUID PRIMARY KEY,
    job_id          UUID REFERENCES search_jobs(id),
    block_number    INTEGER NOT NULL,
    range_start     BIGINT NOT NULL,
    range_end       BIGINT NOT NULL,
    status          TEXT DEFAULT 'pending',  -- pending, claimed, completed, failed
    worker_id       UUID REFERENCES workers(id),
    claimed_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    tested          INTEGER DEFAULT 0,
    found           INTEGER DEFAULT 0
)
```

### Network State

```sql
workers (
    id              UUID PRIMARY KEY,
    hostname        TEXT,
    cores           INTEGER,
    os              TEXT,
    arch            TEXT,
    ram_gb          REAL,
    version         TEXT,
    status          TEXT DEFAULT 'offline',
    current_job_id  UUID,
    last_heartbeat  TIMESTAMPTZ,
    operator_id     UUID REFERENCES operators(id)
)
```

For full schema details, see `supabase/CLAUDE.md`.

---

## AI Engine

### OODA Decision Loop

The AI engine (`src/ai_engine.rs`) runs a continuous decision loop:

```
Observe  → Assemble WorldSnapshot (parallel DB queries, ~50ms)
Orient   → Score all possible actions using 7-component model
Decide   → Select highest-confidence action above threshold
Act      → Execute (create search, rebalance fleet, adjust allocation)
Learn    → Compare predictions to outcomes, update scoring weights
```

### WorldSnapshot

A single consistent view of the world, assembled in one tick:

```rust
struct WorldSnapshot {
    active_searches: Vec<SearchJobSummary>,
    worker_fleet: Vec<WorkerSummary>,
    recent_discoveries: Vec<PrimeRecord>,
    world_records: Vec<WorldRecord>,
    cost_calibrations: Vec<CostCalibration>,
    ai_state: AiEngineState,
    budget: BudgetSummary,
}
```

### 7-Component Scoring Model

Each possible action is scored across 7 dimensions:

| Component | Weight | What It Measures |
|-----------|--------|-----------------|
| `record_gap` | Learned | Distance from current record — how close is a discovery to being a record? |
| `yield_rate` | Learned | Historical primes per core-hour for this form/range |
| `cost_efficiency` | Learned | Compute cost per expected discovery |
| `opportunity_density` | Learned | How many untested candidates remain in the range |
| `fleet_fit` | Learned | How well the work matches available hardware (RAM, CPU, GPU) |
| `momentum` | Learned | Are we making progress or stalling? |
| `competition` | Learned | Is anyone else searching this range? (lower competition = higher score) |

Weights are learned via exponential weighted averaging (EWA) and persisted in the `ai_engine_state` table.

### Cost Model

Power-law cost estimation fitted via OLS regression on log-log work block data:

```
cost(digits) = a * digits^b
```

Where `a` and `b` are per-form coefficients stored in `cost_calibrations`. Falls back to hardcoded defaults when insufficient data.

### Decision Audit Trail

Every AI decision is logged to `ai_engine_decisions`:

```sql
ai_engine_decisions (
    id              UUID PRIMARY KEY,
    action          TEXT,           -- "start_search", "rebalance", "pause"
    form            TEXT,
    parameters      JSONB,
    reasoning       TEXT,           -- Human-readable explanation
    confidence      REAL,           -- 0.0 to 1.0
    scoring         JSONB,          -- All 7 component scores
    outcome         TEXT,           -- Filled in later after evaluation
    created_at      TIMESTAMPTZ
)
```

---

## Distributed Coordination

### Block Claiming

```sql
-- Node claims a work block (atomic, skip-locked)
SELECT id, job_id, range_start, range_end
FROM work_blocks
WHERE job_id = $1
  AND status = 'pending'
ORDER BY block_number
LIMIT 1
FOR UPDATE SKIP LOCKED;

-- Then update to claimed
UPDATE work_blocks
SET status = 'claimed', worker_id = $2, claimed_at = now()
WHERE id = $3;
```

`FOR UPDATE SKIP LOCKED` ensures no two nodes claim the same block. If a block is already locked by another transaction, it's skipped — no contention.

### Heartbeats

Nodes heartbeat every 10 seconds by updating their `last_heartbeat` timestamp:

```sql
UPDATE workers
SET last_heartbeat = now(), status = 'online'
WHERE id = $1;
```

### Stale Pruning

Nodes with `last_heartbeat` older than 120 seconds are marked stale. Their claimed work blocks are reclaimed (set back to `pending`) for reassignment.

### Trust Levels

| Level | Name | Requirements | Privileges |
|-------|------|-------------|------------|
| 0 | New | Just registered | Double-checked results, small blocks |
| 1 | Proven | 10 consecutive valid results | Single-check for routine work |
| 2 | Trusted | 100 valid, >0.98 reliability | Large blocks, priority assignment |
| 3 | Verified | Hardware benchmarked, identity confirmed | High-value work |
| 4 | Core | Long-term, known hardware | Verification duties |

---

## Security

### Authentication & Authorization

- **Dashboard auth:** Supabase JWT tokens. Verified in Axum middleware.
- **API auth:** Same JWT. Admin-only endpoints check role in JWT claims.
- **Node auth:** PostgreSQL connection string. RLS policies restrict access.
- **Operator auth:** API keys (hashed, stored in operators table). Used for node registration.

### Row-Level Security (RLS)

Every table with user-facing data has RLS policies:
- Operators can only see their own nodes and contributions
- Admins can see everything
- Public data (primes, leaderboard) is readable without auth

### Rate Limiting

Nginx layer:
- 10 requests/second per IP (general)
- 60 requests/second for API endpoints
- Burst allowance of 20

### Input Validation

- All API inputs validated via serde deserialization (type-safe)
- SQL injection prevented by sqlx parameterized queries
- No `unsafe` code in main crate (except macOS QoS syscall)

### systemd Hardening

```ini
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
MemoryMax=512M
LimitNOFILE=65536
```

---

## Performance

### Montgomery Multiplication

`MontgomeryCtx` in `sieve.rs` provides constant-time modular multiplication without division. Used in BSGS sieve for O(sqrt(p)) per-prime sieving.

### BitSieve

Packed u64 bitmaps for candidate tracking. 8x memory reduction over `Vec<bool>`. Supports bulk operations (AND, OR, NOT) for combining sieve results.

### PFGW/GWNUM Acceleration

Large candidates (>10K digits) are routed to external tools:
- **PFGW subprocess:** 50-100x acceleration via FFT-based modular exponentiation
- **GWNUM FFI:** Direct library calls (feature-gated), eliminates subprocess I/O overhead
- **PRST subprocess:** k*b^n+/-1 with Gerbicz error checking and proof generation

### PGO Builds

Profile-Guided Optimization via `deploy/pgo-build.sh`:
1. Instrument build with `-Cprofile-generate`
2. Run representative workload (kbn search)
3. Optimize build with `-Cprofile-use`
4. Result: 5-15% speedup on primality testing hot paths

### Release Profile

```toml
[profile.release]
lto = "fat"           # Whole-program link-time optimization
codegen-units = 1     # Maximum optimization (slower compile)
opt-level = 3         # Maximum optimization
```

### mimalloc

Global allocator (`#[global_allocator]`) for 5-10% improvement over system allocator, especially for allocation-heavy sieve operations.

### Wheel Factorization

Sieve uses wheel factorization (mod 30 wheel) to skip multiples of 2, 3, and 5 during candidate generation. Reduces sieve work by ~73%.

### Frobenius Test

Grantham's RQFT (Randomized Quadratic Frobenius Test) fires for candidates >10K bits in the `mr_screened_test` pipeline. Provides stronger composite detection than Miller-Rabin alone (Euler criterion + Frobenius automorphism).

---

## Related Documents

- [Concept](concept.md) — Vision and product definition
- [Platform](platform.md) — Repository map and open/closed split
- [Services](services.md) — External service catalog
- [Infrastructure](infrastructure.md) — Hosting and deployment
- [Business](business.md) — Market analysis and strategy
- [Architecture Roadmap](../roadmaps/architecture.md) — Migration plan (Phase 0-5)
- [Engine Roadmap](../roadmaps/engine.md) — Algorithm improvements
- [Network Roadmap](../roadmaps/network.md) — Distributed compute roadmap
- [Competitive Analysis](../roadmaps/competitive-analysis.md) — Technical gap analysis
