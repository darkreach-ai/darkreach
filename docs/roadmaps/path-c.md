# Path C: The Hybrid Compute Platform

> *Open science runs free. Commercial compute pays the bills. Operators earn from both.*

---

## Executive Summary

darkreach began as a prime-hunting platform. The infrastructure built to coordinate that work — trust-scored operators, AI-driven scheduling, verified distributed computation, credit economies — is general-purpose. Path C evolves darkreach from a prime discovery tool into a **hybrid distributed compute platform** where:

1. **Open science** (primes, genomics, climate modeling) runs free on idle capacity
2. **Commercial customers** (biotech, pharma, materials science) pay for priority access
3. **Operators** earn reputation from open science and money from commercial work
4. **darkreach** sustains itself through platform fees on commercial compute

The primes remain the heartbeat — always running, always visible, always proving the network works. But they become the first application, not the ceiling.

---

## The Two-Sided Market

### Supply: Operators

Operators contribute compute (CPU, GPU, storage, bandwidth) to the network. They run a darkreach node on their hardware — a gaming PC, a home server, a university cluster, cloud spot instances.

**Why they contribute:**

| Motivation | Audience | What they get |
|-----------|----------|---------------|
| Curiosity | Hobbyists, students | Participate in real science, see your machine contribute |
| Altruism | Everyone | "Your computer helps screen drug candidates while you sleep" |
| Competition | Power users | Leaderboard rank, trust level, badges |
| Economics | Serious operators | Revenue share from commercial jobs |
| Idle resources | IT departments, crypto miners | Monetize hardware that would otherwise sit idle |

**What operators earn:**

| Work tier | Reward |
|-----------|--------|
| Open science (primes, public research) | Reputation credits, leaderboard rank, community status |
| Commercial (priority jobs) | Revenue share (60-70% of job fee) + reputation credits |
| Enterprise (dedicated capacity) | Revenue share (70-80% of contract) + reputation credits |

Operators don't choose which jobs they get — the AI engine handles scheduling. But they see a transparent breakdown: "Last month your node contributed 340 core-hours to protein folding and 120 to a pharma screen, earning $47 and 460 reputation credits."

### Demand: Researchers and Companies

Customers submit compute jobs to the network. Jobs are decomposed into work blocks, distributed to operators, and verified.

**Tier structure:**

| Tier | Who | Price | SLA | Verification |
|------|-----|-------|-----|-------------|
| **Open** | University labs, nonprofits, public challenges | Free | Best-effort, no deadline | Trust-based (double-check new operators) |
| **Priority** | Biotech startups, funded research groups | $0.02-0.05/core-hour | 95% within deadline | Double-verified by Level 2+ operators |
| **Enterprise** | Pharma, materials companies, defense | $0.08-0.15/core-hour | 99.9% completion, dedicated capacity | Triple-verified by Level 3+ operators, full audit trail |

**Pricing context:** AWS spot instances run $0.01-0.03/core-hour (no verification, you manage everything). On-demand is $0.04-0.10+. darkreach's value proposition is **cheaper than on-demand, more reliable than spot, with built-in result verification**.

---

## What We Sell

### To a university cancer research lab

"Submit your molecular dynamics simulation. It runs on 500 verified nodes across 12 countries, at zero cost. Results are cross-verified by independent operators. You get a DOI-citable computation certificate."

### To a Series B biotech startup

"Your drug candidate screening runs on thousands of cores at 60% of AWS cost. Results are verified by our trust-scored operator network. You get results in hours instead of days, with a compliance-ready audit trail."

### To a pharma enterprise

"Dedicated capacity from our highest-trust operators. 99.9% SLA. Full audit trail. GxP compliance documentation. Encrypted in-memory execution — data never touches disk on operator nodes. Annual contract, predictable pricing."

### To operators

"Your gaming PC earns $30-80/month while you sleep. Your old server earns more. You're contributing to cancer research, climate modeling, and mathematical discovery. Your trust score is your reputation — the longer you contribute, the more you earn."

---

## Trust as the Product

Most distributed computing platforms treat verification as overhead. In Path C, **trust is the product**.

A pharma company screening drug candidates needs to know results are correct. Not "probably correct" — provably correct. The trust scoring system directly addresses this:

| Trust Level | Operator Profile | Verification Overhead | Eligible Work |
|-------------|-----------------|----------------------|--------------|
| Level 1 (new) | <10 valid results | Triple redundancy | Open science only |
| Level 2 (proven) | 10+ consecutive valid | Double redundancy | Open + Priority |
| Level 3 (trusted) | 100+ consecutive valid | Spot-check 10% | Open + Priority + Enterprise |
| Level 4 (core) | 500+ consecutive valid | Audit trail only | All tiers, highest revenue share |

Enterprise customers pay more, but their work routes to Level 3-4 operators with proven track records. This is a quality guarantee no cloud provider offers.

**The trust moat:** An operator who's built Level 4 trust over months of reliable computation has a real asset — they earn more per core-hour than a new operator. This creates retention. They won't switch to a competitor because they'd start over at Level 1.

---

## Technical Evolution

> **See [Technology Vision](technology-vision.md) for the full protocol architecture** — WASM execution, content-addressed computation, decentralized coordination, and portable trust credentials.

### Phase 1: Prime-Specific Blocks (Current)

Work blocks encode prime search ranges. Each search form has its own sieve/test pipeline. The AI engine schedules across 12 forms.

```
WorkBlock {
    block_id, search_job_id,
    block_start, block_end,    // range to search
    search_type: "kbn",        // which form
    params: { k: 3, base: 2 }, // form-specific parameters
}
```

### Phase 2: WASM-Based General Compute

Work blocks become WASM modules with content-addressed I/O. Any computation that compiles to WASM can run on the network — with mathematical sandboxing guarantees (no gVisor/Firecracker needed).

```
WorkBlock {
    block_id, job_id,
    wasm_module_hash: "sha256:abc...",         // content-addressed WASM binary
    wasm_module_url: "ipfs://Qm.../module.wasm",
    input_data_hash: "sha256:def...",          // deterministic input
    input_data_url: "ipfs://Qm.../input.dat",
    output_schema: "json",
    timeout_seconds: 3600,
    min_cores: 4,
    min_ram_gb: 8,
    requires_gpu: false,
    verification_level: 2,
}
```

**Key advantages over containers:**
- **50μs cold start** vs 1-10s for containers — operators switch tasks instantly
- **500KB modules** vs 100MB+ images — minimal bandwidth for distribution
- **Mathematical sandbox** — WASM spec guarantees memory safety, no escape possible
- **Browser operators** — anyone can contribute compute from a browser tab, zero install
- Content-addressed I/O: same input always produces same output hash for verification
- Operators can opt out of general compute workloads (primes-only mode)

### Phase 3: Task Graphs (DAGs)

Complex computations decomposed into directed acyclic graphs of WASM blocks with dependencies:

```
Drug screening pipeline:
  [Prepare ligand library]
    → [Dock 10,000 candidates] (1,000 parallel WASM blocks)
    → [Score and rank]
    → [MD simulation top 50] (50 parallel WASM blocks)
    → [Final report]
```

Results form a **Merkle DAG** — every node commits to its inputs via cryptographic hash. The entire computation chain is auditable without re-execution. GxP compliance as a data structure.

### Phase 4: Decentralized Protocol

Coordination evolves from centralized PostgreSQL to libp2p gossip + CRDTs:
- Open science jobs distributed via gossip protocol (no central coordinator needed)
- Commercial jobs route through coordinator supernode for SLA guarantees
- Operator trust becomes W3C Verifiable Credentials — portable, cryptographic, operator-owned
- If darkreach the company disappeared, the open science network would keep running

---

## AI Engine Evolution

The AI engine gains new scoring dimensions for commercial scheduling:

### Current Scoring (7 components)
```
record_gap, yield_rate, cost_efficiency, opportunity_density,
fleet_fit, momentum, competition
```

### Path C Scoring (11 components)
```
record_gap, yield_rate, cost_efficiency, opportunity_density,
fleet_fit, momentum, competition,
+ revenue_potential      — commercial jobs weighted by payment
+ deadline_pressure      — time remaining vs estimated compute needed
+ verification_depth     — trust requirements for this job class
+ fairness_balance       — ensure free-tier doesn't starve completely
```

### Scheduling Priority

1. **Enterprise jobs** get first claim on matching nodes (they're paying for guarantees)
2. **Priority jobs** fill remaining capacity with deadline-aware scheduling
3. **Open science** fills everything else — this is still the majority of compute
4. **Prime hunting** runs as the default background job when nothing else needs the cycles

The primes become the network's heartbeat — always running, always verifiable, always producing visible results. They're the screensaver of distributed computing.

### Fairness Guarantees

Open science must never completely starve. Policy controls enforce minimum allocation:

```toml
[fairness]
min_open_science_pct = 20        # At least 20% of capacity goes to open science
max_single_customer_pct = 40     # No customer can monopolize more than 40%
prime_hunting_min_pct = 5        # Primes always run (network heartbeat)
```

---

## Revenue Model

### Unit Economics at Scale

**Target: 50,000 operators, 200,000 cores**

| Revenue Stream | Annual Estimate | Notes |
|----------------|----------------|-------|
| Enterprise contracts | $2-5M | 3-5 pharma/biotech at $500K-1M each |
| Priority compute | $1-3M | Hundreds of research groups, pay-as-you-go |
| Platform licensing | $500K-1M | Organizations running private darkreach networks |
| Open science | $0 | Free, subsidized by the above |

| Cost | Annual Estimate | Notes |
|------|----------------|-------|
| Operator payouts | 60-70% of compute revenue | Revenue share |
| Engineering team | $1-2M | 5-8 engineers |
| Infrastructure | $200-400K | Coordinator servers, data transit, monitoring |
| Partnerships & BD | $100-200K | Research collaborations, conferences |

This gets to profitability at relatively modest scale. You don't need millions of operators — you need thousands of reliable ones and dozens of paying customers.

### Early Revenue (pre-scale)

Before reaching 50K operators, smaller revenue opportunities:

| Source | Timeline | Revenue |
|--------|----------|---------|
| Research partnerships (2-3 university labs) | Month 12-18 | $5-20K/mo |
| Priority compute (startups, funded PIs) | Month 18-24 | $10-50K/mo |
| First enterprise pilot | Month 24-30 | $20-50K/mo |
| Platform licensing (private network) | Month 24-36 | $10-30K/mo |

The goal is to reach $30K/mo revenue by Month 24 — enough to cover infrastructure, start operator payouts, and fund 2-3 engineers.

---

## Corporate Structure

### Public Benefit Corporation (PBC)

darkreach should be structured as a **Public Benefit Corporation** — a for-profit company with a legally binding commitment to public good.

**Why PBC over nonprofit:**
- Can raise venture capital if needed
- Can pay competitive engineering salaries
- Can distribute operator payouts as business expenses
- Open science commitment is enshrined in corporate charter, not just marketing
- Precedent: Patagonia (outdoor gear), OpenAI (started as nonprofit, restructured), Kickstarter

**Charter commitment:**
- Minimum 20% of network capacity dedicated to open science at all times
- Open-source engine remains MIT-licensed permanently
- Operator trust data is portable (operators can export their reputation)
- Annual transparency report on compute allocation (open vs commercial)

**Why not pure nonprofit:**
- 501(c)(3) status creates IRS tensions with earned revenue
- Hard to attract engineering talent at below-market salaries
- Grant funding cycles are slow and unpredictable
- Can't offer equity compensation

---

## Competitive Moat

Why can't AWS, Google, or a well-funded startup just replicate this?

### 1. Cloud providers won't cannibalize themselves
They sell compute at high margins. A volunteer network undercuts their core business. They'll never build this.

### 2. Trust is earned, not bought
The operator trust graph takes months to build. A new competitor starts every operator at Level 1. darkreach's Level 3-4 operators are a compounding asset.

### 3. Community is the product
Operators who contribute to cancer research on darkreach won't switch to "Amazon Volunteer Compute" for an extra $5/month. Identity and mission matter.

### 4. AI scheduling is domain-specific
Cost models, yield estimates, fleet-fit scoring, verification depth calculations — these encode deep knowledge about computational workloads. This compounds over time as the system learns.

### 5. Verification is genuinely hard
Triple-redundant execution with trust-scored operators is a novel quality guarantee that cloud providers don't offer. Building this from scratch takes years.

### 6. Network effects
More operators → more capacity → more customers → more revenue → higher operator payouts → more operators. The flywheel compounds.

---

## Implementation Phases

### Phase 1: Foundation (Now - Month 6)

**Goal:** Prime hunting network at 1,000+ operators. Trust system proven. Credit economy working.

| Milestone | Target |
|-----------|--------|
| External operators | 100+ |
| Trust levels working | Levels 1-4 with verified progression |
| Credit system | Transparent earning and spending |
| First t5k.org discovery | 1+ verified prime |
| Security hardened | IP allowlisting, HMAC signing, audit trail |

**What we're proving:** The distributed coordination, trust, and verification infrastructure works at scale with real operators on real hardware.

### Phase 2: Research Partnerships (Month 6-12)

**Goal:** First non-prime workload running on the network. 2-3 academic partnerships.

| Milestone | Target |
|-----------|--------|
| Containerized work blocks | Prototype running |
| First research partner | University lab submitting jobs |
| Operator opt-in for general compute | Working preference system |
| SDK for job submission | Python/CLI interface |
| BOINC compatibility layer | Import existing BOINC tasks |

**Key partnerships to pursue:**
- **Folding@home** — protein folding, established community, aging infrastructure
- **Rosetta@home** — protein structure prediction (Baker lab, UW)
- **Climate modeling** — downscaling climate projections (computationally embarrassing)
- **Genomics** — sequence alignment, variant calling (highly parallel)

**What we're proving:** The network can run arbitrary computation, not just primes. Researchers trust the results.

### Phase 3: First Revenue (Month 12-18)

**Goal:** 2-3 paying priority customers. Operator payouts begin.

| Milestone | Target |
|-----------|--------|
| Priority tier live | Pay-as-you-go billing |
| First paying customer | Research group using priority compute |
| Operator payouts | First real-money distribution |
| Revenue | $5-20K/mo |
| Operators | 1,000+ |

**What we're proving:** Customers will pay for verified distributed compute. Operators stay engaged when earning real money.

### Phase 4: Platform (Month 18-24)

**Goal:** Self-service job submission. Public SDK. Enterprise pilot.

| Milestone | Target |
|-----------|--------|
| Self-service portal | Customers submit jobs via web UI or API |
| Python SDK | `pip install darkreach && darkreach.submit(job)` |
| Task graph support | DAG-based complex pipelines |
| Enterprise pilot | 1 pharma/biotech on annual contract |
| Revenue | $30-80K/mo |
| Operators | 5,000+ |

**What we're proving:** The platform can serve enterprise customers with SLAs and audit trails.

### Phase 5: Scale (Month 24-36)

**Goal:** 10,000+ operators. $1M+ ARR. Multiple enterprise customers.

| Milestone | Target |
|-----------|--------|
| Enterprise contracts | 3-5 customers |
| GPU compute | GPU workloads running at scale |
| Private networks | Organizations running darkreach internally |
| Revenue | $100K+/mo |
| Operators | 10,000+ |
| Problem domains | 5+ (primes, protein folding, drug screening, climate, genomics) |

### Phase 6: Global Compute Network (Month 36+)

**Goal:** darkreach is the default platform for distributed verified computation.

| Milestone | Target |
|-----------|--------|
| Operators | 50,000+ |
| Revenue | $3-8M/year |
| Problem domains | 10+ |
| Automated job optimization | AI designs optimal compute strategies per domain |
| Regulatory compliance | HIPAA, GxP, SOC2 for enterprise tier |

---

## The Primes Through All Phases

Prime hunting never stops. It serves multiple roles across the platform lifecycle:

| Role | Why it matters |
|------|---------------|
| **Proof of concept** | Demonstrates the network works: distributed coordination, verification, trust scoring |
| **Community identity** | "We hunt primes" is a compelling origin story. It attracts the first operators. |
| **Network heartbeat** | Always running, always verifiable. If primes are being found, the network is healthy. |
| **Benchmark workload** | Known computational cost. Perfect for calibrating operator performance and network throughput. |
| **Open science flagship** | The most visible free-tier workload. "Your computer found a 500,000-digit prime last night." |
| **Trust calibration** | Prime results are independently verifiable. Ideal for building and maintaining operator trust scores. |

---

## Key Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| **Can't attract paying customers** | Medium | High | Start with free academic partnerships to build case studies. Revenue is Phase 3, not Phase 1. |
| **Operators don't want general compute** | Low | Medium | Opt-in model. Prime-only mode always available. Container sandboxing addresses security concerns. |
| **Enterprise security requirements too high** | Medium | Medium | Build compliance incrementally (SOC2, HIPAA). Start with less regulated domains (climate, genomics). |
| **Cloud providers compete** | Low | High | They can't without cannibalizing their core business. Our moat is trust + community. |
| **Regulatory issues with distributed health data** | Medium | High | Enterprise tier uses encrypted in-memory execution. No data persistence on operator nodes. |
| **Operator payout logistics** | Medium | Low | Start with platform credits. Graduate to Stripe Connect or crypto for real payouts. |
| **Open science gets crowded out** | Low | Medium | Charter commitment: minimum 20% of capacity. Fairness guarantees in AI scheduler. |

---

## One-Sentence Pitches

| Audience | Pitch |
|----------|-------|
| **Operators** | Your computer cures cancer while you sleep, and you get paid for it. |
| **Researchers** | Supercomputer-scale verified computation, zero cost for open science. |
| **Enterprise** | Elastic, trust-verified distributed compute at 60% of cloud cost. |
| **Investors** | The Airbnb of computing — we don't own a single server, but we coordinate the world's largest verified compute network. |
| **The world** | We turn idle computers into collective intelligence. |

---

## Related Documents

- [Technology Vision](technology-vision.md) — Protocol architecture (WASM, libp2p, CRDTs, VCs)
- [Business Strategy](../internal/business.md) — Market analysis and monetization
- [Platform Architecture](../internal/platform.md) — Repository map and open/closed split
- [Network Roadmap](network.md) — Distributed compute infrastructure
- [Resources Roadmap](resources.md) — Heterogeneous resource pooling
- [AI Engine Roadmap](ai-engine.md) — Autonomous scheduling intelligence
- [Frontend Roadmap](frontend.md) — Dashboard and operator experience
