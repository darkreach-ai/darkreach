# Concept

> *Reaching into the dark unknown of undiscovered primes.*

---

## The Problem

Prime number discovery is one of the oldest problems in mathematics. Yet the infrastructure used to hunt primes is stuck in the past:

- **GIMPS** (1996) uses custom C software, a proprietary work assignment server (PrimeNet), and a website from the mid-2000s. It searches exactly one form: Mersenne primes.
- **PrimeGrid** (2005) runs on BOINC, a general-purpose volunteer computing framework designed in 2004. Installing the client requires downloading BOINC, entering a project URL, and configuring subprojects through a clunky desktop UI.
- **Independent searchers** cobble together shell scripts around command-line tools (PFGW, LLR, mtsieve) with no centralized coordination, no dashboard, and no reproducibility.

No modern platform exists that combines:
1. Multi-form prime discovery (beyond just Mersenne)
2. AI-driven search orchestration
3. Distributed compute coordination with modern tooling
4. A real-time dashboard for monitoring and control
5. Automated verification and publication pipelines

The tools exist (GMP, GWNUM, PFGW, PRST, mtsieve). The math is well-understood. What's missing is the **platform** that ties them together.

---

## The Vision

**The world's most advanced prime discovery platform.**

darkreach is the system that makes prime hunting systematic, intelligent, and accessible:

- An **engine** that implements 12+ special-form prime searches with state-of-the-art sieving and proving
- An **AI brain** that researches forms, designs campaigns, allocates compute, and learns from results
- A **network** of compute nodes that claim work, report results, and earn trust
- A **dashboard** that shows everything in real-time: candidates tested, primes found, fleet status, AI decisions
- A **verification pipeline** that proves results and publishes them to t5k.org and OEIS

Every prime discovered is a permanent contribution to mathematics. darkreach makes those contributions faster, more reliable, and more accessible than anything that exists today.

---

## Core Beliefs

### 1. Proofs over PRPs

A probable prime (PRP) is not a prime. t5k.org rejects PRPs. The math community requires deterministic proofs. Every search form in darkreach must either produce a proof (Proth, LLR, Pocklington, Morrison, BLS, Pepin) or clearly mark results as PRP with a path toward proof generation.

### 2. Multi-form diversification

Mersenne primes get all the attention, but hundreds of other forms have minimal competition and realistic discovery potential. Non-base-2 Sierpinski/Riesel conjectures need 1-10 core-years per discovery. Twin prime records are a decade stale. Wagstaff has no active searcher. darkreach searches where others don't.

### 3. AI-first orchestration

Human researchers can't continuously monitor world records, evaluate ROI across 12 forms, rebalance compute allocation based on results, and adjust strategy in real-time. An AI engine can. The OODA loop (Observe-Orient-Decide-Act) runs autonomously, with human override via the dashboard.

### 4. Open algorithms, commercial platform

The engine (sieving, primality testing, proofs) is open-source. Trust in mathematical results requires transparency. The platform (coordination, AI, dashboard, fleet management) is the commercial product. This mirrors the relationship between GMP (open library) and the systems built on top of it.

### 5. Modern UX for science

Volunteer computing shouldn't require reading a FAQ from 2006. A prime hunter in 2026 should get a modern dashboard, real-time WebSocket updates, search management UI, and `docker run darkreach/node` to start contributing. The UX is a competitive advantage, not an afterthought.

### 6. Reproducible verification

Every discovery must be independently verifiable. Different software, different hardware, different algorithm where possible. The verification pipeline automates this: PFGW cross-check, PRST confirmation, certificate generation. No result is published without independent confirmation.

---

## The User

### Primary: Researchers & Record Hunters

Mathematicians and computational number theorists who want to discover primes in specific forms. They care about:
- Targeting underserved forms with realistic discovery potential
- Generating valid proofs for t5k.org submission
- Publication pipeline (OEIS, arXiv, journals)
- Understanding the AI's reasoning for search allocation
- Full control over search parameters when needed

### Secondary: Operators (Compute Contributors)

People who run darkreach nodes on their hardware. They care about:
- Easy setup (`darkreach register && darkreach run`)
- Seeing their contribution (blocks completed, primes found, compute hours)
- Choosing what forms to search and how much CPU to use
- Trust and credit tracking
- Auto-updating software

### Tertiary: Institutions & Universities

Research groups that want dedicated compute for specific campaigns. They care about:
- Targeted searches for specific conjectures or forms
- Publication-ready results with proper verification
- Integration with their existing compute infrastructure
- Reproducibility and auditability

### Explicitly NOT

- **Crypto miners.** darkreach doesn't produce tokens, coins, or financial value from compute.
- **Blockchain projects.** Prime discovery is pure mathematics, not consensus or proof-of-work.
- **Generic HPC users.** darkreach is purpose-built for prime hunting, not a general compute platform.

---

## What We Build

### Engine (Open Source)

The core prime-hunting library and CLI tool.

- **12 search forms**: factorial, palindromic, k*b^n +/- 1, near-repdigit, primorial, Cullen/Woodall, Wagstaff, Carol/Kynea, twin, Sophie Germain, repunit, generalized Fermat
- **Sieving**: Sieve of Eratosthenes, BSGS, Montgomery multiplication, wheel factorization, BitSieve, P-1 factoring
- **Primality testing**: Miller-Rabin, Proth, LLR, Frobenius, BPSW
- **Proofs**: Pocklington (N-1), Morrison (N+1), BLS, Pepin
- **External tool integration**: PFGW, PRST, GWNUM (FFI), FLINT, mtsieve
- **3-tier verification**: deterministic proof -> BPSW+MR -> PFGW cross-check
- **CLI**: `darkreach factorial --start 1 --end 100`, `darkreach kbn --k 3 --base 2 --min-n 1 --max-n 1000`

### Platform (Commercial)

The coordination, intelligence, and UX layer.

- **Dashboard**: Next.js 16 + React 19, 17 pages, 50+ components, real-time WebSocket
- **API server**: Axum REST API, 15 route modules, WebSocket push
- **AI engine**: OODA decision loop, 7-component scoring, cost model fitting, weight learning
- **Search management**: Create/stop/pause/resume searches, all 12 forms, presets, block generation
- **Network coordination**: PostgreSQL-based work claiming, heartbeats, stale pruning, trust levels
- **Project campaigns**: Multi-phase, cost tracking, record comparison, AI-designed campaigns
- **Agent infrastructure**: Tasks, budgets, memory, roles, schedules, autonomous execution
- **Operator system**: Registration, API keys, node management, credit tracking
- **Verification queue**: Distributed verification with quorum consensus

### Website (Open Source)

Public-facing presence at darkreach.ai.

- Marketing pages, feature descriptions, competitive positioning
- Documentation (engine usage, operator setup, API reference)
- Live stats (primes found, nodes online, compute power)
- Leaderboard (top contributors, recent discoveries)
- Download page with OS detection

---

## What We Don't Build

- **Blockchain anything.** No tokens, no mining, no consensus.
- **Generic HPC.** Not a general-purpose compute platform.
- **Non-prime compute.** No protein folding, no climate modeling, no SETI.
- **Mersenne-only search.** GIMPS has 30 years of infrastructure and 23,000+ devices. We don't compete on Mersenne; we go where they don't.
- **BOINC integration.** BOINC is a 2004 framework. We build native tooling.

---

## The Name

**darkreach** — reaching into the dark unknown of undiscovered primes.

The vast majority of prime numbers are unknown. For every form, there's a frontier beyond which no one has searched. Beyond that frontier is darkness — unexplored mathematical territory where new primes wait to be found.

darkreach extends humanity's reach into that darkness.

- **Domain**: darkreach.ai
- **Dashboard**: app.darkreach.ai
- **API**: api.darkreach.ai
- **GitHub**: github.com/darkreach-ai/darkreach
- **Registry**: ghcr.io/darkreach-ai/darkreach

---

## Success Metrics

| Metric | What It Measures | Target |
|--------|-----------------|--------|
| **Discoveries per month** | Engine effectiveness + AI strategy quality | 5+ primes in underserved forms |
| **t5k.org submissions** | Proof pipeline and publication readiness | First submission within 6 months |
| **Verification throughput** | Proof generation + cross-verification speed | < 24h from discovery to verified |
| **Operator fleet size** | Platform adoption and contributor experience | 10 nodes (Phase 2), 100 nodes (Phase 4) |
| **Time-to-publication** | End-to-end pipeline from discovery to recognition | < 30 days to t5k.org listing |
| **Search form coverage** | Breadth of active research | All 12 forms actively searched |
| **AI autonomy ratio** | Fraction of searches created by AI vs human | > 80% AI-initiated by Phase 4 |
| **World record** | The ultimate validation | At least one record in an underserved form |

---

## Related Documents

- [Platform Map](platform.md) — Repository structure and open/closed split
- [Services](services.md) — External service catalog
- [Infrastructure](infrastructure.md) — Hosting, deployment, monitoring
- [Business](business.md) — Market analysis and monetization options
- [Architecture](architecture.md) — Technical design and data flow
