# Business

> *Turning idle computers into collective intelligence — starting with primes, scaling to science.*

---

## Executive Summary

**darkreach** is a distributed compute platform that coordinates a global network of trust-scored operators to solve computationally hard problems. It began with prime number discovery (12 search forms, deterministic proofs, AI orchestration) and is evolving into a **hybrid compute marketplace** where open science runs free, commercial customers pay for verified computation, and operators earn from both.

The prime discovery ecosystem is dominated by GIMPS (Mersenne-only, 30 years old) and PrimeGrid (multi-form, BOINC-based, 20 years old). No modern distributed compute platform exists with AI-native scheduling, proper trust mechanics, and a real incentive system — for primes or any other domain.

darkreach fills this gap. The engine is open-source (for trust, reproducibility, and academic citations). The platform (coordination, AI, trust, marketplace) is the commercial product. The business model is **Path C: Hybrid** — open science fills idle capacity for free, commercial customers pay for priority access, operators earn revenue share. See [Path C Roadmap](../roadmaps/path-c.md) for the full plan.

---

## Mission & Vision

**Mission:** Democratize access to verified computation — for science, for discovery, for everyone.

**Vision:** A global compute network where anyone can contribute their idle hardware to solve humanity's hardest problems. AI designs optimal strategies, a trust-scored network of operators executes the work, and every result is independently verified. Primes are the proving ground. Science is the destination.

### Values

1. **Mathematical rigor.** Proofs, not probabilities. Every result is verified independently.
2. **Openness where it matters.** The engine is open-source. The math must be auditable.
3. **Modern experience.** No one should have to install BOINC in 2026.
4. **Intelligent search.** AI allocation beats human intuition for multi-form optimization.
5. **Sustainability.** The platform must fund itself or it dies. Open-source alone doesn't pay server bills.

---

## Market Analysis

### Target Segments

#### Segment 1: Researchers & Record Hunters

Mathematicians and computational number theorists actively searching for primes.

- **Size:** ~500-1,000 active searchers globally (based on GIMPS ~1,200 active users, PrimeGrid ~3,100, significant overlap, plus independents)
- **Needs:** Targeting underserved forms, proof generation, publication pipeline, full control over search parameters
- **Current tools:** PFGW, LLR, PRST, mtsieve (command-line), custom scripts
- **Pain points:** No coordination, no dashboard, no AI strategy, manual verification
- **Willingness to pay:** Low individually, but may have institutional budgets

#### Segment 2: Operators (Compute Contributors)

People who contribute CPU/GPU cycles to prime discovery. The "volunteers" of GIMPS and PrimeGrid.

- **Size:** ~25,000 active devices across GIMPS + PrimeGrid, but most are passive (set and forget)
- **Needs:** Easy setup, visibility into contribution, credit/recognition, auto-updating software
- **Current tools:** Prime95/mprime, BOINC client
- **Pain points:** Clunky installation, no modern UI, limited form choice
- **Willingness to pay:** Near zero (they're contributing compute, not buying it)

#### Segment 3: Institutions & Universities

Research groups, math departments, and organizations with compute infrastructure.

- **Size:** ~50-100 groups globally that do computational number theory
- **Needs:** Dedicated searches for specific conjectures, publication-ready results, reproducibility, integration with HPC clusters
- **Current tools:** Custom scripts, BOINC server setup (complex)
- **Pain points:** High barrier to run their own search infrastructure
- **Willingness to pay:** Moderate (grant-funded research budgets)

### Market Size

The prime hunting market is niche but passionate:

| Metric | Value | Source |
|--------|-------|--------|
| GIMPS active users | ~1,200 | mersenne.org (Feb 2026) |
| GIMPS active devices | ~23,000 | mersenne.org |
| PrimeGrid total users | 357,258 | primegrid.com |
| PrimeGrid active users | ~3,100 | primegrid.com |
| PrimeGrid active hosts | ~12,600 | primegrid.com |
| t5k.org tracked primes | 5,000 | t5k.org |
| OEIS sequences with prime terms | 10,000+ | oeis.org |

The total addressable market in dollar terms is small (this is not a billion-dollar market). But the impact is permanent — every prime discovered is an irreversible contribution to mathematics.

### Competitive Landscape

| | GIMPS | PrimeGrid | Independent | **darkreach** |
|---|---|---|---|---|
| **Forms** | 1 (Mersenne) | 10+ | 1-2 per searcher | **12+** |
| **Infrastructure** | PrimeNet (1997) | BOINC (2004) | Shell scripts | **Rust + PG + Next.js (2025)** |
| **AI/Strategy** | None | None | Manual | **OODA loop, 7-component scoring** |
| **Client** | Prime95 (1996) | BOINC (2004) | CLI tools | **Native Rust binary + Docker** |
| **UX** | Static HTML | BOINC template | Terminal | **Modern dashboard, WebSocket** |
| **Verification** | Pietrzak proofs | BOINC quorum | Manual | **3-tier + cross-software** |
| **GPU** | GpuOwl/PRPLL | GeneferOCL | Varies | **Planned** |
| **Onboarding** | Download Prime95, configure | Download BOINC, enter URL | Read docs, compile | **`docker run` or download binary** |
| **Years running** | 30 | 20 | N/A | **1** |
| **Trust** | Established | Established | N/A | **Building** |

**darkreach's advantages:** Modern stack, multi-form, AI orchestration, better UX, open engine.

**darkreach's disadvantages:** New (no track record), no major discovery yet, small fleet.

---

## Open Source Strategy

### What's Open

| Component | License | Why Open |
|-----------|---------|----------|
| Engine (12 forms, sieving, proofs) | MIT | **Trust.** Mathematical results must be reproducible. Researchers need to verify the algorithms. Academic papers cite the code. |
| CLI tool | MIT | **Adoption.** Anyone should be able to run `darkreach kbn --k 3 --base 2` without asking permission. |
| Worker client | MIT | **Participation.** Operators need to run trusted code. They won't run a black box on their hardware. |
| Website | MIT | **Community.** Contributions welcome. Fork-friendly. |

### What's Closed

| Component | Why Closed |
|-----------|-----------|
| Dashboard & API | **Sustainability.** The platform is the product. The coordination layer, AI engine, and UX are what make darkreach more than just a CLI tool. |
| AI engine | **Competitive advantage.** The OODA loop, scoring model, and learned weights are the "brain" that differentiates darkreach from running PFGW manually. |
| Operations | **Security.** Infrastructure configs, secrets, deployment procedures. |
| Database schema | **Security.** RLS policies, migration history, business logic embedded in schema. |

### The principle

**If someone needs to verify a mathematical result, they can.** The engine is fully open. They can compile it, run the same search, and confirm the prime.

**If someone wants to run their own darkreach platform, they build it themselves.** The coordination, AI, and UX layer is the commercial offering.

---

## Monetization: Path C (Hybrid)

**Decision: Path C — Hybrid Compute Marketplace.** Open science runs free, commercial customers pay for priority/enterprise access, operators earn revenue share. See [Path C Roadmap](../roadmaps/path-c.md) for full details.

### Why Path C

| Option considered | Verdict |
|-------------------|---------|
| **A: Donations/grants** | Not sustainable ($1-5K/yr doesn't cover $600-900/mo infra) |
| **B: Open Core SaaS** | Too narrow — selling dashboards to researchers is a small market |
| **C: Hybrid marketplace** | **Selected.** Aligns all incentives: operators earn, researchers get free compute, companies get verified results |
| **D: Research partnerships** | Absorbed into Path C as the enterprise tier |

### Revenue Tiers

| Tier | Who | Price | SLA |
|------|-----|-------|-----|
| **Open** | University labs, nonprofits, public challenges | Free | Best-effort |
| **Priority** | Biotech startups, funded research groups | $0.02-0.05/core-hour | 95% within deadline |
| **Enterprise** | Pharma, materials companies | $0.08-0.15/core-hour | 99.9%, dedicated capacity, audit trail |

### Revenue Targets

| Timeline | Revenue | Source |
|----------|---------|-------|
| Month 12-18 | $5-20K/mo | 2-3 research partnerships |
| Month 18-24 | $30-80K/mo | Priority tier + first enterprise pilot |
| Month 24-36 | $100K+/mo | Multiple enterprise contracts |
| Month 36+ | $250-650K/mo | 3-5 enterprise + hundreds of priority customers |

### Operator Economics

Revenue split: **60-70% to operators**, 30-40% to darkreach.

| Operator scale | Avg. earnings | Notes |
|---------------|--------------|-------|
| Gaming PC (8 cores, nights only) | $15-30/mo | ~240 core-hours/mo at $0.06-0.12 effective rate |
| Home server (24/7, 16 cores) | $40-80/mo | ~11,520 core-hours/mo |
| Small cluster (64 cores) | $150-300/mo | Serious operators with dedicated hardware |

### Corporate Structure

**Public Benefit Corporation (PBC)** — for-profit with legally binding commitment to public good:
- Minimum 20% of network capacity dedicated to open science (charter commitment)
- Engine remains MIT-licensed permanently
- Annual transparency report on compute allocation
- Can raise venture capital, pay market salaries, offer equity

---

## Go-to-Market Phases

### Phase 1: Foundation (Now - Month 6)

**Goal:** Prime hunting network at 1,000+ operators. Trust system proven.

- Complete proof pipeline (PRST proofs, PFGW proof collection)
- Submit first discovery to t5k.org
- Open-source the engine on GitHub
- Polish operator onboarding (`darkreach register && darkreach run`)
- Ship auto-updating binary downloads (Linux, macOS, Windows)
- Launch darkreach.ai with live stats, leaderboard, download page
- Security hardening complete (IP allowlisting, HMAC signing, audit trail)

**Success metric:** 100+ external operators, first t5k.org listing.

### Phase 2: Research Partnerships (Month 6-12)

**Goal:** First non-prime workload. 2-3 academic partnerships established.

- Containerized work block prototype
- SDK for job submission (Python/CLI)
- First research partner (university lab) submitting jobs
- BOINC compatibility layer (import existing tasks)
- Operator opt-in for general compute workloads

**Success metric:** Non-prime workload running on the network.

### Phase 3: First Revenue (Month 12-18)

**Goal:** 2-3 paying priority customers. Operator payouts begin.

- Priority tier live with pay-as-you-go billing (Stripe)
- First paying customer using priority compute
- Operator payouts: first real-money distribution
- Self-service job submission portal

**Success metric:** $5-20K/mo revenue, operator payouts flowing.

### Phase 4: Platform & Enterprise (Month 18-36)

**Goal:** Enterprise pilot, multiple problem domains, sustainable revenue.

- Task graph support (DAG-based pipelines)
- Enterprise pilot with pharma/biotech company
- GPU compute workloads at scale
- Compliance documentation (SOC2, GxP for pharma)
- 5+ problem domains running

**Success metric:** $100K+/mo revenue, 10,000+ operators, 3+ enterprise customers.

---

## Success Metrics

| Metric | Phase 1 (6mo) | Phase 2 (12mo) | Phase 3 (18mo) | Phase 4 (36mo) |
|--------|--------------|---------------|---------------|---------------|
| **Operators** | 100+ | 1,000+ | 5,000+ | 10,000+ |
| **Compute (cores)** | 500+ | 5,000+ | 20,000+ | 200,000+ |
| **Discoveries** | 1+ t5k.org | 10+ primes/mo | 50+ primes/mo | 100+ primes/mo |
| **Problem domains** | Primes only | Primes + 1 research | 3+ domains | 5+ domains |
| **Revenue** | $0 | $0 | $5-20K/mo | $100K+/mo |
| **Paying customers** | 0 | 0 | 2-3 | 10+ |
| **Operator payouts** | $0 | $0 | First payouts | $50K+/mo distributed |

---

## Risk Analysis

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| **No significant discovery** | Medium | High | Target underserved forms with realistic ROI; AI strategy maximizes discovery probability |
| **PrimeGrid/GIMPS modernize** | Low | Medium | They're slow to change (30-year codebase). Our advantage is speed of iteration. |
| **Can't attract paying customers** | Medium | High | Start with free academic partnerships to build case studies. Revenue is Phase 3. |
| **Can't attract operators** | Medium | Medium | Start with primes community. Revenue share from commercial jobs incentivizes growth. |
| **Infrastructure costs exceed budget** | Low | Medium | Scale software before hardware. GWNUM gives 50-100x speedup — worth more than $10K/mo in servers. |
| **Key person risk** | Medium | High | Open-source engine ensures continuity. Document everything. |
| **Verification dispute** | Low | High | 3-tier verification, cross-software confirmation, certificate generation. Independent reproducibility. |
| **Community backlash (commercial)** | Low | Medium | Engine is open. The precedent (Red Hat, GitLab, Supabase) is well-established. |
| **Legal issues (prime claims)** | Very low | Low | Primes are mathematical facts, not intellectual property. Discovery credit is social, not legal. |

---

## Related Documents

- [Path C Roadmap](../roadmaps/path-c.md) — Full hybrid compute platform roadmap
- [Technology Vision](../roadmaps/technology-vision.md) — Protocol architecture (WASM, libp2p, CRDTs, VCs)
- [Concept](concept.md) — Vision and product definition
- [Platform](platform.md) — Repository map and open/closed split
- [Services](services.md) — External service costs
- [Infrastructure](infrastructure.md) — Hosting and scaling costs
- [Architecture](architecture.md) — Technical design
- [Research Roadmap](../roadmaps/research.md) — Strategic targets and ROI analysis
- [Competitive Analysis](../roadmaps/competitive-analysis.md) — Detailed competitor research
