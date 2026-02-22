# Technology Vision: The Protocol Architecture

> *We're not building a compute marketplace. We're building a new kind of computer.*

---

## Overview

darkreach's long-term technology vision rests on four composable primitives that transform it from a centralized job queue into a **decentralized computation protocol**. Each primitive is independently production-proven; composed together, they create something no one else has.

| Primitive | Implementation | What It Gives You |
|-----------|---------------|-------------------|
| **Computation** | WASM modules + content-addressed I/O | Deterministic, portable, sandboxed, browser-native execution |
| **Trust** | W3C Verifiable Credentials + ZK proofs | Portable, composable, privacy-preserving, operator-owned reputation |
| **Coordination** | libp2p + CRDTs | Decentralized, partition-tolerant, no single point of failure |
| **Intelligence** | RL scheduling + federated learning | Self-improving, adaptive, learns from the network |

**The distinction:** BOINC is a job queue. AWS is a VM store. darkreach is a **protocol**. The difference is the same as between a website and the web.

---

## Primitive 1: WebAssembly Execution

### Why WASM, Not Containers

The Path C roadmap originally proposed containerized work blocks (Docker + gVisor/Firecracker). WASM is the superior choice for distributed untrusted computation:

| | Containers (Docker/gVisor) | WASM (Wasmtime + WASI) |
|---|---|---|
| **Image size** | 100MB – 2GB | 500KB – 5MB |
| **Cold start** | 1–10 seconds | **50 microseconds** |
| **Security model** | Bolted on (namespaces, seccomp, gVisor) | **Mathematical** (memory-safe by spec) |
| **Portability** | x86 or ARM, Linux only | **Any CPU, any OS, browsers** |
| **Sandboxing** | OS-level (can escape) | **Language-level (cannot escape)** |
| **Browser support** | No | **Yes — operators from a browser tab** |

WASM's security isn't "hardened" — it's **proven**. A WASM module literally cannot access memory outside its linear memory space. No syscall escape, no container breakout, no privilege escalation. The sandbox is the spec itself.

### WASI: Controlled System Access

WASI (WebAssembly System Interface) gives WASM modules controlled access to host capabilities — file I/O, network, clocks, random — but only the capabilities explicitly granted. This is **capability-based security** at the runtime level.

```
Operator grants WASM module:
  ✓ Read input.dat (content-addressed, pre-verified)
  ✓ Write output.dat (validated against schema)
  ✓ Use N CPU cores for M seconds
  ✗ Network access (denied)
  ✗ Filesystem access beyond input/output (denied)
  ✗ Environment variables (denied)
```

### WASM Component Model

The upcoming WASM Component Model makes modules composable: a drug screening module can import a molecular dynamics component without either seeing the other's memory. This enables a **module marketplace** — verified, reusable computation components.

### Browser-Based Operators

Because WASM runs natively in browsers, anyone can contribute compute by visiting `darkreach.ai/contribute` and clicking "Start." No installation. No Docker. No trust decision about running a binary. The WASM sandbox means the worst a malicious module can do is waste CPU cycles.

WebGPU (shipping in all major browsers) extends this to GPU computation from the browser.

### Production Precedent

| Company | WASM Usage | Scale |
|---------|-----------|-------|
| Cloudflare Workers | Edge compute | Millions of requests/second |
| Fastly Compute | Edge compute | Global CDN |
| Fermyon Cloud | Serverless WASM | Production |
| Cosmonic | Distributed WASM | Enterprise |
| Figma | In-browser rendering | 4M+ users |

### Migration Path

The current Rust engine compiles to WASM with minimal changes (`wasm32-wasi` target). Prime search becomes the first WASM workload. Operators can run both native (fast, for serious operators) and WASM (general, for browser contributors).

```
Phase 1 (current):  Native Rust binary on operator hardware
Phase 2:            Wasmtime runtime alongside native, prime search as first WASM workload
Phase 3:            All general compute runs as WASM, native Rust for prime-specific optimization
Phase 4:            Browser SDK for zero-install contribution via WebAssembly + WebGPU
```

---

## Primitive 2: Content-Addressed Computation (The Merkle Computer)

Every computation in darkreach should be a **pure function**: deterministic input → deterministic output, where both are content-addressed by their cryptographic hash.

```
Computation = hash(WASM_module) + hash(input_data) → hash(output_data)
```

This is Git's insight applied to computation.

### Verification Becomes Hash Comparison

If two operators execute the same computation and produce the same output hash, the result is correct. No need to inspect the output — the hash commits to every bit. Triple redundancy means three operators compute independently; if 2/3 hashes match, you have consensus with mathematical certainty.

### Results Are Cacheable Forever

Same module + same input = same output. If someone ran this exact computation before, the answer already exists. A global content-addressed cache means the network never does the same work twice.

### Computation Chains Are Cryptographically Auditable

A drug screening pipeline becomes a Merkle DAG:

```
                    hash(report)
                   /            \
     hash(scored_results)    hash(visualization)
            |
  hash(dock_1) ... hash(dock_10000)
            |
  hash(ligand_prep) + hash(protein_prep)
            |                  |
  hash(ligand_library)   hash(protein_structure)
```

Every node commits to its inputs. A pharma company can hand this DAG to a regulator and they can verify the entire computation chain — which code ran, on which inputs, producing which intermediate results — without re-executing anything. **GxP compliance as a data structure.**

### For Primes

Prime verification already has this structure:

```
hash(kbn_module) + hash(params: k=3, b=2, n=4423) → hash(result: prime=true, certificate=Proth)
```

Path C generalizes it to arbitrary computation. The `PrimalityCertificate` enum becomes a special case of a `ComputationCertificate`.

### Content-Addressed Storage

Input and output data stored in a content-addressed store (similar to IPFS or Git's object store). Benefits:
- **Deduplication**: identical inputs stored once regardless of how many jobs reference them
- **Integrity**: hash verification on retrieval — corrupted data is detected immediately
- **Distribution**: popular inputs can be cached at operator nodes, reducing coordinator bandwidth

---

## Primitive 3: Decentralized Coordination (libp2p + CRDTs)

### The Problem with Central Coordination

The current architecture has a single PostgreSQL coordinator. This works for hundreds of operators but creates:
- **Single point of failure** — coordinator down = network down
- **Scalability ceiling** — all heartbeats, block claims, and results flow through one server
- **Geographic latency** — operators in Asia route through European coordinator

### Gossip Protocol (libp2p)

libp2p (used by IPFS, Filecoin, Ethereum, Polkadot) provides:

| Component | Purpose |
|-----------|---------|
| **Kademlia DHT** | Decentralized operator discovery — no central registry |
| **GossipSub** | Pub/sub for job announcements, results, trust updates |
| **Hole punching** | NAT traversal — operators behind firewalls can participate |
| **mTLS** | Every connection authenticated and encrypted |
| **Circuit relay** | Fallback connectivity through relay nodes |

### CRDTs for Distributed State

CRDTs (Conflict-free Replicated Data Types) provide eventually consistent state without coordination:

| State | CRDT Type | Behavior |
|-------|-----------|----------|
| Operator registry | OR-Set (add/remove) | Join/leave propagates, no conflicts |
| Job status | LWW-Register | Last-writer-wins on status transitions |
| Trust scores | G-Counter | Monotonically increasing, merge = max per operator |
| Capacity ads | LWW-Map | Latest capability snapshot per operator |
| Discovery log | G-Set (append-only) | Prime discoveries are permanent |

### Hybrid Architecture

The coordinator doesn't disappear — it becomes a **supernode** in the gossip network:

```
Current architecture:
  Operator → HTTP → Coordinator (PostgreSQL) ← HTTP ← Operator

Protocol architecture:
  Operator ←→ Gossip ←→ Operator ←→ Gossip ←→ Operator
            ↑                                    ↑
       Local CRDT state                    Local CRDT state
            ↓                                    ↓
       Eventually consistent global view
                         ↕
              Supernode (coordinator)
         Authoritative for: billing, SLAs,
         enterprise routing, customer accounts
```

What stays centralized:
- Billing and payments (needs strong consistency and ACID guarantees)
- Customer accounts (needs authoritative state)
- Enterprise SLA enforcement (needs global view)
- AI engine decisions (needs aggregated data for RL)

What becomes decentralized:
- Operator discovery and heartbeats
- Open science job distribution
- Result propagation and verification consensus
- Trust score updates
- Capacity advertisement

### Resilience Guarantee

If darkreach the company disappeared tomorrow, the open science network would keep running. **The protocol is the product, not the server.** This is the strongest possible trust signal to the academic community.

### Migration Path

```
Phase 1 (current):  PostgreSQL-only coordination, HTTP heartbeats
Phase 2:            Add libp2p for operator-to-operator gossip (heartbeats, capacity discovery)
Phase 3:            Open science jobs distributed via gossip; commercial via coordinator
Phase 4:            Full CRDT state for network metadata; coordinator is just a supernode
```

---

## Primitive 4: Portable Trust (Verifiable Credentials)

### Trust as a Cryptographic Asset

Operator trust shouldn't be a row in a database. It should be a **cryptographic credential** that the operator owns.

W3C Verifiable Credentials provide:

```json
{
  "@context": ["https://www.w3.org/ns/credentials/v2"],
  "type": ["VerifiableCredential", "DarkreachTrustCredential"],
  "issuer": "did:web:darkreach.ai",
  "credentialSubject": {
    "id": "did:key:z6Mkh...operatorPublicKey",
    "trustLevel": 3,
    "validComputations": 847,
    "consecutiveValid": 847,
    "domains": ["prime-search", "molecular-dynamics"],
    "totalCoreHours": 12400,
    "since": "2026-01-15"
  },
  "proof": { "type": "Ed25519Signature2020", "..." }
}
```

### Properties

| Property | Implication |
|----------|------------|
| **Operator-owned** | Signed by darkreach but held by the operator. Exportable. |
| **Portable** | Usable on any platform that accepts darkreach trust credentials. Signals confidence: operators stay because we're the best platform, not because we hold their data hostage. |
| **Privacy-preserving** | ZK proofs allow proving "trust ≥ 3" without revealing exact score, identity, or history. Enterprise customers verify trust without doxxing operators. |
| **Composable** | darkreach credential + university lab credential + auditor credential = composite trust profile stronger than any single attestation. |
| **Revocable** | If an operator produces bad results, darkreach publishes a revocation. Credential becomes invalid network-wide. |

### Decentralized Identity (DIDs)

Each operator gets a DID (Decentralized Identifier) — a globally unique, self-sovereign identity:

```
did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK
```

DIDs are independent of darkreach. They're controlled by the operator's private key. Trust credentials are linked to the DID, not to a darkreach account.

### Migration Path

```
Phase 1 (current):  Trust scores in PostgreSQL (trust_level, consecutive_valid)
Phase 2:            Issue W3C VCs for trust milestones, stored alongside DB records
Phase 3:            Operators present VCs for authentication; DB becomes cache
Phase 4:            ZK trust proofs for privacy-preserving enterprise verification
```

---

## Intelligence Layer: Reinforcement Learning

### Beyond Heuristic Scoring

The current AI engine uses a 7-component linear scoring model with EWA-learned weights. This is a good start but fundamentally limited — linear combinations can't capture the complex interactions between network state, job characteristics, operator capabilities, and market dynamics.

### RL Scheduling

```
Current:  score = w1*record_gap + w2*yield_rate + w3*cost_efficiency + ...
          (hand-tuned weights, linear combination)

Target:   policy = RL_agent(state) → action
          state  = {network_topology, pending_jobs, operator_capabilities,
                    trust_levels, SLA_deadlines, market_prices, ...}
          action = {job_assignments, capacity_reservations, price_adjustments}
          reward = f(SLA_compliance, cost_efficiency, fairness, discovery_rate,
                     operator_satisfaction, customer_retention)
```

The RL agent discovers policies that no hand-tuned heuristic could find:
- "Pre-position capacity in European time zones before London biotech firms submit daily batch"
- "This operator's CPU has thermal throttling on Thursdays — schedule lighter work"
- "Route genomics jobs to operators who've completed similar jobs, even if general trust is lower"

### Federated Learning

Each operator's node learns local patterns (hardware characteristics, availability, thermal behavior) and shares **model updates** (not raw data) with the network. The global model improves from thousands of local observations without centralizing sensitive data.

### Migration Path

```
Phase 1 (current):  7-component linear scoring with EWA weight learning
Phase 2:            Neural network scoring model trained on historical decisions
Phase 3:            RL agent with multi-objective reward (SLA, cost, fairness, discovery)
Phase 4:            Federated learning across operator nodes
```

---

## Enterprise Confidentiality: TEE Attestation

For enterprise customers with sensitive data (drug candidates, proprietary sequences), Trusted Execution Environments provide hardware-guaranteed confidentiality:

```
Enterprise job submission:
  Customer encrypts input with TEE public key
  → WASM module runs inside Intel TDX / AMD SEV-SNP enclave
  → Hardware attestation proves: this exact WASM module ran on this input
  → Output encrypted with customer's key
  → Operator never sees plaintext data
  → Nobody — not even darkreach — can read the computation
```

### Why TEE Over Homomorphic Encryption

FHE (Fully Homomorphic Encryption) is theoretically ideal but 1000–10,000x slower than plaintext computation. TEE provides near-native performance with cryptographic attestation — practical today, not in five years.

### TEE Premium

Operators with TEE-capable hardware (Intel TDX, AMD SEV-SNP) earn a premium on enterprise jobs. The attestation is cryptographic — a remote verifier confirms the exact code that ran, on which hardware, with which security configuration.

### Production Precedent

| Platform | TEE Usage |
|----------|----------|
| Azure Confidential Computing | Production, GA |
| AWS Nitro Enclaves | Production, GA |
| Google Confidential VMs | Production, GA |

---

## Full Architecture

```
╔══════════════════════════════════════════════════════════════╗
║                    darkreach Protocol                        ║
║                                                              ║
║  ┌─────────────────────────────────────────────────────┐     ║
║  │                 Interface Layer                      │     ║
║  │  Next.js Dashboard │ CLI │ Python SDK │ Browser SDK  │     ║
║  │  REST + gRPC + GraphQL                               │     ║
║  └──────────────────────┬──────────────────────────────┘     ║
║                         │                                     ║
║  ┌──────────────────────┴──────────────────────────────┐     ║
║  │              Intelligence Layer                      │     ║
║  │  RL Scheduling │ Federated Learning │ Cost Models    │     ║
║  │  Drift Detection │ Anomaly Detection │ Price Engine  │     ║
║  └──────────────────────┬──────────────────────────────┘     ║
║                         │                                     ║
║  ┌──────────────────────┴──────────────────────────────┐     ║
║  │              Coordination Layer                      │     ║
║  │  libp2p (GossipSub + Kademlia + mTLS)               │     ║
║  │  CRDTs for distributed state                         │     ║
║  │  Supernode coordinator for commercial ops            │     ║
║  └──────────────────────┬──────────────────────────────┘     ║
║                         │                                     ║
║  ┌──────────────────────┴──────────────────────────────┐     ║
║  │              Trust Layer                             │     ║
║  │  W3C Verifiable Credentials │ ZK Trust Proofs        │     ║
║  │  Merkle DAG computation audit │ TEE Attestation      │     ║
║  └──────────────────────┬──────────────────────────────┘     ║
║                         │                                     ║
║  ┌──────────────────────┴──────────────────────────────┐     ║
║  │              Execution Layer                         │     ║
║  │  Wasmtime + WASI (standard operators)                │     ║
║  │  Wasmtime + TEE (enterprise confidential)            │     ║
║  │  Browser WASM + WebGPU (zero-install contribution)   │     ║
║  │  Native Rust (prime-specific optimization)           │     ║
║  │  Content-addressed I/O (Merkle DAG)                  │     ║
║  └──────────────────────────────────────────────────────┘     ║
╚══════════════════════════════════════════════════════════════╝
```

---

## Implementation Timeline

### Year 1: Foundation + First Extensions

| Quarter | Focus | Primitives Used |
|---------|-------|----------------|
| Q1-Q2 | Prime network at scale, trust system proven, operator base growing | Current stack (Rust + PG) |
| Q3 | Wasmtime runtime integration, prime search as first WASM workload | Computation |
| Q4 | Content-addressed I/O, Merkle DAG audit trail, computation certificates | Computation + Trust |

### Year 2: Protocol Emergence

| Quarter | Focus | Primitives Used |
|---------|-------|----------------|
| Q1 | libp2p integration, gossip-based heartbeats and capacity discovery | Coordination |
| Q2 | W3C Verifiable Credentials for operator trust milestones | Trust |
| Q3 | General compute via WASM (first non-prime workload), browser SDK | Computation + Coordination |
| Q4 | RL scheduling prototype, federated learning for operator models | Intelligence |

### Year 3: Full Protocol

| Quarter | Focus | Primitives Used |
|---------|-------|----------------|
| Q1 | TEE attestation for enterprise tier, confidential computing | Trust |
| Q2 | CRDT-based state, coordinator becomes supernode | Coordination |
| Q3 | ZK trust proofs, privacy-preserving verification | Trust + Intelligence |
| Q4 | Full protocol: decentralized open science + centralized commercial | All four |

### Key Principle

Each phase works without the later ones. The current Rust + PostgreSQL architecture is the correct Phase 1. WASM extends it. libp2p decentralizes it. VCs formalize the trust model. RL optimizes scheduling. Each layer adds a primitive. Each primitive makes the system fundamentally more capable.

---

## Technology Choices: Why These, Not Others

| Choice | Why | Not |
|--------|-----|-----|
| **WASM** over containers | Mathematical sandbox, microsecond startup, browser support | Docker + gVisor (OS-level, heavyweight, no browser) |
| **libp2p** over custom protocol | Battle-tested (Ethereum, IPFS), NAT traversal, extensible | Custom TCP/UDP (years of engineering for solved problems) |
| **CRDTs** over distributed DB | No coordination needed, partition-tolerant, offline-capable | CockroachDB/Spanner (requires always-on connectivity) |
| **W3C VCs** over database scores | Portable, cryptographic, privacy-preserving, standard | Proprietary reputation (lock-in, not verifiable) |
| **RL** over heuristic scoring | Discovers non-obvious policies, improves continuously | Static weights (ceiling on optimization) |
| **TEE** over FHE | Near-native performance, production-ready today | FHE (1000x slower, research-stage) |
| **Merkle DAGs** over flat storage | Cryptographic audit trail, deduplication, verifiable chains | S3/blob storage (no integrity guarantees) |

---

## Competitive Implications

No existing platform has this combination:

| Capability | BOINC | Golem | Akash | Render | **darkreach** |
|------------|-------|-------|-------|--------|--------------|
| WASM execution | No | Partial | No | No | **Yes** |
| Content-addressed computation | No | No | No | No | **Yes** |
| Decentralized coordination | No | Ethereum-based | Cosmos-based | No | **libp2p (no blockchain)** |
| Cryptographic trust credentials | No | No | No | No | **W3C VCs** |
| AI-driven scheduling | No | No | No | No | **RL + federated** |
| TEE attestation | No | No | No | No | **Yes** |
| Browser-based operators | Partial (BOINC WebAssembly) | No | No | No | **Yes** |
| Prime discovery heartbeat | PrimeGrid (BOINC) | No | No | No | **Native** |

The blockchain-based platforms (Golem, Akash) chose the wrong coordination primitive. Blockchains are slow (seconds to minutes per transaction), expensive (gas fees), and overkill for job scheduling. libp2p + CRDTs give you decentralization without the overhead.

---

## Related Documents

- [Path C Roadmap](path-c.md) — Hybrid compute platform business strategy
- [Network Roadmap](network.md) — Distributed compute infrastructure
- [AI Engine Roadmap](ai-engine.md) — Autonomous scheduling intelligence
- [Resources Roadmap](resources.md) — Heterogeneous resource pooling
- [Engine Roadmap](engine.md) — Prime-hunting algorithms
- [Server Roadmap](server.md) — Backend infrastructure
- [Frontend Roadmap](frontend.md) — Dashboard and operator experience
- [Ops Roadmap](ops.md) — Deployment and infrastructure
- [Business Strategy](../internal/business.md) — Market analysis and monetization
- [Platform Architecture](../internal/platform.md) — Repository map and open/closed split
