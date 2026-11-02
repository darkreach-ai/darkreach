Research prime forms, discovery strategies, and competitive landscape.

## Context

Read `docs/CLAUDE.md` for the full research structure. The `docs/` directory contains prime form research and 14 strategic roadmaps.

## Key Resources

### Prime Form Research
- `docs/factorial-primes.md` — n! ± 1 (records, OEIS A002981/A002982, testing methods)
- `docs/palindromic-primes.md` — Palindromic (even-digit proof, distribution, records)
- `docs/kbn-primes.md` — k·b^n ± 1 (Proth/LLR theory, Mersenne, records)
- `docs/landscape.md` — Distributed compute landscape (BOINC, GIMPS, PrimeGrid)
- `docs/references.md` — OEIS, PrimePages, GIMPS, PrimeGrid, software links

### Strategic Roadmaps (14 files in `docs/roadmaps/`)
| File | Topic | Size |
|------|-------|------|
| `research.md` | Discovery strategy, ROI, publication pipeline | 17 KB |
| `competitive-analysis.md` | Market analysis, competitor landscape | 27 KB |
| `engine.md` | Algorithm improvements, new forms | 18 KB |
| `agents.md` | AI agent infrastructure | 20 KB |
| `projects.md` | Project lifecycle, cost tracking | 17 KB |
| `testing.md` | Testing strategy, benchmarks | 16 KB |
| `fleet.md` | Cluster coordination | 15 KB |
| `gwnum-flint.md` | GWNUM/FLINT integration | 14 KB |
| `public-compute.md` | Volunteer compute platform | 10 KB |
| `ops.md` | Deployment, optimization | 9 KB |
| `website.md` | Public website | 8 KB |
| `server.md` | Backend infrastructure | 7 KB |
| `frontend.md` | Dashboard features | 7 KB |
| `cluster.md` | Multi-node management | 3 KB |

## ROI Comparison

| Search Type | Core-years/discovery | Provable? | Competition | Best for |
|------------|---------------------|-----------|-------------|----------|
| Sierpinski/Riesel (non-base-2) | 1-10 | Yes | Low | Quick wins |
| Palindromic record | 100-1,000 | Yes (BLS) | 1 team | High visibility |
| Factorial | ~2,300 | Yes | PrimeGrid | Auto-provable |
| Wagstaff PRP | ~3,000 | No | None | Prestige |

## Evaluating New Forms

1. **Competition**: Who else is searching? Check PrimeGrid, GIMPS, Conjectures 'R Us
2. **ROI**: Estimate core-years per discovery
3. **Provability**: Can results be proven? (t5k.org requires proofs for ranking)
4. **Architecture fit**: Does it work with Rust + rug/GMP + rayon?
5. **Code reuse**: Can `kbn::test_prime` or existing sieves be reused?

## Publication Pipeline

```
Discovery → Cross-verify (different software + hardware)
  → DO NOT announce publicly until verified
  → t5k.org submission (with proof)
  → OEIS update
  → arXiv preprint
  → Journal paper (3-6 months)
```

## Currently Implemented Forms (12)

factorial, palindromic, kbn, near_repdigit, primorial, cullen_woodall, wagstaff, carol_kynea, twin, sophie_germain, repunit, gen_fermat

## Updating Roadmaps

1. Domain roadmaps in `docs/roadmaps/` — organized by impact tier
2. Include: current state, target, algorithm/pseudocode, rationale
3. Root `ROADMAP.md` links to all domain roadmaps
4. References consolidated in `docs/roadmaps/research.md`
