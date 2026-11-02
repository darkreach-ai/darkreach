# Platform

> *What goes where, and why. See [Path C](../roadmaps/path-c.md) for the platform evolution roadmap.*

---

## Platform Overview

```
                              darkreach.ai Platform
 ┌──────────────────────────────────────────────────────────────────────┐
 │                                                                      │
 │   darkreach.ai          app.darkreach.ai         api.darkreach.ai    │
 │   ┌──────────┐          ┌──────────────┐         ┌──────────────┐    │
 │   │  Website │          │  Dashboard   │         │   REST API   │    │
 │   │ (public) │──fetch──▶│  (Next.js)   │◀──────▶│   (Axum)     │    │
 │   │          │          │  Static SPA  │         │  + WebSocket │    │
 │   └──────────┘          └──────────────┘         └──────┬───────┘    │
 │   darkreach-web          darkreach-app                   │           │
 │   (public repo)          (private repo)                  │           │
 │                                                          │           │
 │                          ┌───────────────────────────────┘           │
 │                          │                                           │
 │                          ▼                                           │
 │   ┌──────────────┐ ┌──────────────┐ ┌──────────────┐                │
 │   │  PostgreSQL  │ │  AI Engine   │ │  Job Submit  │                │
 │   │  + Redis     │◀│  OODA Loop   │ │  API / SDK   │                │
 │   └──────┬───────┘ └──────────────┘ └──────────────┘                │
 │          │          darkreach-app    (Python SDK, CLI)               │
 │          │                                                           │
 │          │  ┌─────────────────────────────────────────┐              │
 │          │  │         Compute Marketplace              │              │
 │          │  │  Open science (free) │ Priority (paid)  │              │
 │          │  │  Enterprise (SLA)   │ Primes (heartbeat)│              │
 │          │  └─────────────────────────────────────────┘              │
 │          │                                                           │
 │     ┌────┼────────────┬───────────────┐                              │
 │     ▼    ▼            ▼               ▼                              │
 │  ┌────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐          │
 │  │ Node 1 │  │  Node 2    │  │  Node N    │  │ GPU Node │          │
 │  │ (CPU)  │  │ (CPU+GPU)  │  │ (CPU)      │  │ (GPU)    │          │
 │  └────────┘  └────────────┘  └────────────┘  └──────────┘          │
 │        darkreach engine (public repo) + container runtime            │
 │        Claims work from PG, runs searches/containers, reports        │
 │                                                                      │
 └──────────────────────────────────────────────────────────────────────┘
```

---

## Recent Changes (Feb 2026)

- **Naming migration complete** (migration 049): volunteer -> operator, worker -> node, fleet -> network. Backward-compat re-exports preserved. Internal `workers` table not renamed (search workers, not operator nodes).
- **Website deployed** to Vercel (`darkreach.ai`) with ~22 pages.
- **ML engine** added (migration 059): Thompson Sampling bandits, GP cost model, Bayesian optimization, node intelligence.
- **22 route modules** (was 15), **24 DB submodules** (was 15), **64 migrations** (was ~40).

---

## Repository Map

### `darkreach` — Engine & CLI (Public, MIT)

The open-source core. Everything needed to hunt primes independently.

| Attribute | Value |
|-----------|-------|
| **Visibility** | Public (MIT license) |
| **Language** | Rust |
| **Contents** | 12 search forms, sieving primitives, primality testing, proofs, verification, certificates, PFGW/PRST/GWNUM integration, checkpoint system, CLI |
| **Deployment** | crates.io (`darkreach`), GitHub Releases (binaries: x86_64 + aarch64, Linux + macOS) |
| **CI** | Build, test (1,325 tests, 19 ignored), clippy, benchmark, cross-compile, release signing |

**What's included:**
```
src/
├── lib.rs                  # Core library (small primes, trial division, MR, Frobenius)
├── main.rs + cli.rs        # CLI entry point and subcommand dispatch
├── factorial.rs            # n! +/- 1 search
├── palindromic.rs          # Palindromic prime search
├── kbn.rs                  # k*b^n +/- 1 (Proth/LLR/Pocklington, BSGS sieve)
├── near_repdigit.rs        # Near-repdigit palindromic search
├── primorial.rs            # p# +/- 1 search
├── cullen_woodall.rs       # n*2^n +/- 1 search
├── wagstaff.rs             # (2^p+1)/3 search
├── carol_kynea.rs          # (2^n +/- 1)^2 - 2 search
├── twin.rs                 # Twin prime search
├── sophie_germain.rs       # Sophie Germain prime search
├── repunit.rs              # R(b,n) = (b^n-1)/(b-1)
├── gen_fermat.rs           # b^(2^n)+1 search
├── sieve.rs                # Sieve, Montgomery mult, wheel factorization, BitSieve
├── proof.rs                # Pocklington, Morrison, BLS deterministic proofs
├── verify.rs               # 3-tier verification pipeline
├── certificate.rs          # PrimalityCertificate enum
├── p1.rs                   # Pollard P-1 factoring
├── pfgw.rs                 # PFGW subprocess integration
├── gwnum.rs                # GWNUM FFI (feature-gated)
├── prst.rs                 # PRST subprocess integration
├── flint.rs                # FLINT integration (feature-gated)
├── checkpoint.rs           # JSON checkpoint save/load
├── progress.rs             # Atomic progress counters
├── worker_client.rs        # Node client (work claiming, heartbeat)
├── pg_worker.rs            # PostgreSQL work claiming
└── operator.rs             # Operator node management
```

**What's NOT included (lives in darkreach-app):**
- Dashboard server (`src/dashboard/`)
- AI engine (`src/ai_engine.rs`)
- Agent infrastructure (`src/agent.rs`)
- Search manager (`src/search_manager.rs`)
- Fleet coordination (`src/fleet.rs`)
- Deployment tools (`src/deploy.rs`)
- Event bus (`src/events.rs`)
- Metrics/Prometheus (`src/metrics.rs`, `src/prom_metrics.rs`)

---

### `darkreach-app` — Dashboard & API (Private)

The commercial coordination platform.

| Attribute | Value |
|-----------|-------|
| **Visibility** | Private |
| **Language** | Rust (API) + TypeScript (frontend) |
| **Contents** | Axum API server, Next.js dashboard, AI engine, agent system, search management, fleet coordination, project campaigns |
| **Deployment** | Docker (GHCR), deployed to api.darkreach.ai + app.darkreach.ai |
| **CI** | Build, test, lint, Docker build+push, deploy |

**Rust backend:**
```
src/
├── dashboard/              # Axum web server
│   ├── mod.rs              # Router, AppState, middleware, static serving
│   ├── websocket.rs        # WebSocket (2s push interval)
│   ├── routes_*.rs         # 22 route modules
│   └── middleware_auth.rs  # JWT auth middleware
├── db/                     # PostgreSQL via sqlx
│   ├── mod.rs              # Database struct, connection pool
│   └── *.rs                # 24 domain submodules
├── project/                # Campaign management
├── ai_engine.rs            # OODA decision loop
├── agent.rs                # Agent infrastructure
├── search_manager.rs       # Search lifecycle
├── fleet.rs                # In-memory node registry
├── deploy.rs               # SSH deployment
├── events.rs               # Event bus
├── metrics.rs              # System metrics
└── prom_metrics.rs         # Prometheus export
```

**Frontend:**
```
frontend/
├── src/app/                # 17 pages (Next.js App Router)
├── src/components/         # 50+ components (shadcn/ui)
├── src/hooks/              # 17+ custom hooks
├── src/lib/                # Supabase client, API helpers
└── public/                 # Static assets
```

**Depends on:** `darkreach` (as Cargo dependency for engine types, checkpoint, verification)

---

### `darkreach-web` — Public Website (Public, Deployed)

Marketing, docs, and community presence at darkreach.ai.

| Attribute | Value |
|-----------|-------|
| **Visibility** | Public |
| **Language** | TypeScript (Next.js + MDX) |
| **Contents** | Landing page, docs (~8 doc pages), blog, leaderboard, download, research, operators, status, about, platform |
| **Deployment** | **Vercel** (deployed), served at `darkreach.ai` via `website/vercel.json` |
| **CI** | Build, deploy on push to master (Vercel GitHub integration) |
| **Pages** | ~22 pages across docs, blog, research, operators, download, leaderboard, status, about, platform |

**Structure:**
```
website/
├── src/app/
│   ├── page.tsx            # Landing page (hero, stats, features, forms, CTA)
│   ├── docs/               # Documentation (engine, API, operator guide)
│   ├── blog/               # Discovery announcements, technical posts
│   ├── leaderboard/        # Top contributors, recent discoveries
│   └── download/           # OS-detected binary download
├── src/components/         # Shared UI components
└── public/                 # Static assets, favicons
```

**Data sources:**
- Live stats from `api.darkreach.ai/api/stats` (public endpoint, no auth)
- Discovery feed from `api.darkreach.ai/api/primes/recent` (public endpoint)
- Leaderboard from `api.darkreach.ai/api/leaderboard` (public endpoint)

---

### `darkreach-ops` — Infrastructure (Private)

Everything needed to deploy, monitor, and operate the platform.

| Attribute | Value |
|-----------|-------|
| **Visibility** | Private |
| **Language** | Shell, YAML, HCL, JSON |
| **Contents** | Terraform, Helm charts, Grafana dashboards, deploy scripts, systemd units, secrets (SOPS + age), CI/CD workflow definitions |
| **Deployment** | Applied to infrastructure (Hetzner, Cloudflare, Supabase) |

**Structure:**
```
ops/
├── terraform/
│   ├── modules/            # Hetzner servers, Cloudflare DNS, networking
│   └── environments/       # Per-environment tfvars (staging, production)
├── helm/
│   └── darkreach/          # Helm chart (coordinator, node, configmap, secrets)
├── deploy/
│   ├── deploy.sh           # SSH deployment script
│   ├── production-deploy.sh # Full production setup
│   └── pgo-build.sh        # Profile-Guided Optimization build
├── systemd/
│   ├── darkreach-coordinator.service
│   └── darkreach-worker@.service
├── nginx/
│   └── darkreach.conf      # Reverse proxy, rate limiting, WebSocket, TLS
├── grafana/
│   └── darkreach.json      # Dashboard definition
├── secrets/
│   ├── .sops.yaml          # SOPS age key configuration
│   └── *.enc.yaml          # Encrypted secrets
└── ci/
    └── *.yml               # GitHub Actions workflow definitions
```

---

### `darkreach-db` — Database (Private)

Schema, migrations, and database utilities.

| Attribute | Value |
|-----------|-------|
| **Visibility** | Private |
| **Language** | SQL, TypeScript (Supabase CLI) |
| **Contents** | PostgreSQL migrations, RLS policies, seed data, database utilities |
| **Deployment** | Applied to Supabase via migration CLI |

**Structure:**
```
db/
├── supabase/
│   ├── migrations/         # 64 sequential SQL migrations
│   └── seed.sql            # Development seed data
├── scripts/
│   ├── migrate.sh          # Migration runner
│   └── backup.sh           # Database backup
└── docs/
    └── schema.md           # Schema documentation
```

---

## Boundary Definition

### Why this split?

| Component | Open/Closed | Rationale |
|-----------|-------------|-----------|
| **Engine** | Open (MIT) | Trust. Mathematical results must be reproducible. Researchers need to audit the algorithms. Citations require open code. |
| **Dashboard + API** | Closed | Sustainability. The platform is the product. Coordination, AI, and UX are the value-add over running the CLI manually. |
| **Website** | Open | Community. The public face should be forkable and contributable. |
| **Job Submission SDK** | Open (MIT) | Adoption. Researchers need to submit jobs from their own tools. |
| **Container Task Runtime** | Open (MIT) | Trust. Operators run the sandboxed executor on their hardware. |
| **Ops** | Closed | Security. Infrastructure configs, secrets, and deployment procedures should not be public. |
| **Database** | Closed | Security. Schema + RLS policies + migration history contain business logic and security boundaries. |

### The boundary rule

**If it touches math, it's open. If it touches infrastructure, it's closed.**

- Sieve of Eratosthenes? Open.
- BSGS sieve implementation? Open.
- Proth test? Open.
- AI engine that decides which Proth tests to run? Closed.
- WebSocket that streams results? Closed.
- CLI that a node uses to claim work? Open (it's how operators participate).
- Dashboard that an admin uses to manage searches? Closed.

---

## Cross-Repo Dependencies

```
darkreach-app ──depends on──▶ darkreach (Cargo dependency)
                               Types: CheckpointData, PrimalityCertificate, SearchForm
                               Functions: verify_prime(), test_prime(), sieve()

darkreach-web ──fetches from──▶ darkreach-app (REST API)
                                 GET /api/stats
                                 GET /api/primes/recent
                                 GET /api/leaderboard

darkreach-ops ──deploys──▶ darkreach-app (Docker image from GHCR)
              ──deploys──▶ darkreach-web (static build to CDN)
              ──applies──▶ darkreach-db (migrations to Supabase)

darkreach-db ──consumed by──▶ darkreach-app (sqlx queries)
```

### Cargo dependency

In `darkreach-app/Cargo.toml`:
```toml
[dependencies]
darkreach = { git = "https://github.com/darkreach-ai/darkreach", branch = "main" }
```

Or for local development:
```toml
[dependencies]
darkreach = { path = "../darkreach" }
```

---

## Domain Routing

| Domain | Target | Content |
|--------|--------|---------|
| `darkreach.ai` | Vercel (deployed) | Public website (landing, docs, blog, research, operators, leaderboard) |
| `app.darkreach.ai` | Hetzner coordinator (Nginx -> port 7001) | Dashboard SPA (static export served by Axum) |
| `api.darkreach.ai` | Hetzner coordinator (Nginx -> port 7001) | REST API + WebSocket |

### Nginx routing on coordinator

```
# api.darkreach.ai -> Axum backend (all paths)
server {
    server_name api.darkreach.ai;
    location / {
        proxy_pass http://127.0.0.1:7001;
    }
    location /ws {
        proxy_pass http://127.0.0.1:7001/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}

# app.darkreach.ai -> Axum backend (serves static frontend + API)
server {
    server_name app.darkreach.ai;
    location / {
        proxy_pass http://127.0.0.1:7001;
    }
}
```

### DNS (Cloudflare)

| Record | Type | Value |
|--------|------|-------|
| `darkreach.ai` | CNAME | Vercel deployment |
| `app.darkreach.ai` | A | 178.156.211.107 (coordinator) |
| `api.darkreach.ai` | A | 178.156.211.107 (coordinator) |

---

## Migration Path (Current Monorepo -> Multi-Repo)

The current codebase is a single monorepo. The split happens incrementally:

### Phase 1: Extract darkreach-web (Low risk)

The `website/` directory is already self-contained and deployed to Vercel. Move it to its own repo when ready. Currently deployed directly from monorepo via Vercel GitHub integration.

### Phase 2: Extract darkreach-ops (Low risk)

Move `deploy/`, `secrets/`, Helm charts, Terraform, and CI workflow definitions. These are already somewhat independent.

### Phase 3: Extract darkreach-db (Low risk)

Move `supabase/` directory. Set up migration CI.

### Phase 4: Split darkreach / darkreach-app (High complexity)

This is the major split. Requires:
1. Defining the public API surface of the `darkreach` crate
2. Moving dashboard, AI engine, agent, fleet, search manager, deploy to `darkreach-app`
3. Setting up the Cargo dependency from app -> engine
4. Ensuring the CLI works both standalone (open-source user) and as part of the platform (node connecting to PG)

**Gating:** This split should happen after the architecture migration (see `docs/roadmaps/architecture.md`) is complete, since that migration already reorganizes the codebase boundaries.

---

## Related Documents

- [Path C Roadmap](../roadmaps/path-c.md) — Hybrid compute platform strategy
- [Technology Vision](../roadmaps/technology-vision.md) — Protocol architecture (WASM, libp2p, CRDTs, VCs)
- [Concept](concept.md) — Vision and product definition
- [Services](services.md) — External service catalog
- [Infrastructure](infrastructure.md) — Hosting and deployment details
- [Architecture](architecture.md) — Technical design
- [Architecture Roadmap](../roadmaps/architecture.md) — Migration plan (Phase 0-5)
