Work on prime-hunting algorithms in the darkreach engine.

## Context

Read `src/CLAUDE.md` for the full module map and patterns. The engine domain covers 12 prime search forms + core primitives in `src/`.

### Search Forms (12 total)
- `factorial.rs` — n! ± 1 (GMP factorial, modular sieve, PFGW -tp/-tm proofs)
- `palindromic.rs` — Palindromic primes (digit array generation, deep sieve)
- `kbn.rs` — k·b^n ± 1 (Proth/LLR/Pocklington, BSGS sieve) — **reused by 5 forms**
- `near_repdigit.rs` — Near-repdigit palindromic (BLS N+1 proofs, PFGW)
- `primorial.rs` — p# ± 1 (Pocklington/Morrison, PFGW)
- `cullen_woodall.rs` — n·2^n ± 1 (Proth/LLR, PFGW)
- `wagstaff.rs` — (2^p+1)/3 (multiplicative order sieve, PFGW/GWNUM)
- `carol_kynea.rs` — (2^n±1)²−2 (LLR, PFGW)
- `twin.rs` — Twin primes (quad sieve, Proth+LLR intersection)
- `sophie_germain.rs` — Sophie Germain (Proth+LLR intersection sieve)
- `repunit.rs` — R(b,n) = (b^n−1)/(b−1) (PFGW)
- `gen_fermat.rs` — b^(2^n)+1 (Pépin/Proth, PFGW)

### Core Primitives
- `sieve.rs` — Sieve of Eratosthenes, Montgomery multiplication, wheel factorization, BitSieve
- `lib.rs` — Module re-exports, small primes table, trial division, MR pre-screening, Frobenius test
- `proof.rs` — Pocklington (N−1), Morrison (N+1), BLS deterministic proofs
- `verify.rs` — 3-tier verification pipeline (deterministic → BPSW+MR → PFGW)
- `certificate.rs` — PrimalityCertificate enum
- `p1.rs` — Pollard P−1 factoring for deep composite elimination

### External Tools
- `pfgw.rs` — PFGW subprocess (50-100x speedup)
- `gwnum.rs` — GWNUM FFI wrapper (feature-gated)
- `prst.rs` — PRST subprocess for k·b^n±1
- `flint.rs` — FLINT integration (feature-gated)

## Key Patterns

- Every search form follows: sieve → parallel test (rayon) → proof → log to PostgreSQL
- `kbn::test_prime` is `pub(crate)`, returns `(IsPrime, &str, Option<PrimalityCertificate>)`, reused by twin, sophie_germain, cullen_woodall, carol_kynea, gen_fermat
- `insert_prime_sync(rt, expression, form, digits, proof_method, params_json, certificate)` — 7 args
- All 12 forms must check `worker_client.is_stop_requested()` in their block loop
- Checkpoint variant per form in `checkpoint.rs`

## Conventions

- No `unsafe` code (except gwnum-sys FFI, feature-gated)
- Pre-allocate `rug::Integer` with `Integer::with_capacity(bits)` before hot loops
- `checked_u32()` instead of `n as u32` for `.pow()` / `<<`
- `has_small_factor()`: compare via `*n != p` (avoids heap alloc)
- Even-digit palindromes always divisible by base+1 — skip
- Results: "deterministic" (proven) or "probabilistic" (PRP)
- Wagstaff: no deterministic proof exists — always PRP

## Commenting Standards

**Teaching tool.** Follow `.claude/commenting-standards.md`:
- `//!` module doc comment with purpose, algorithm, complexity, references (OEIS, papers)
- `///` doc comment on every `pub` item with algorithm steps, math basis, complexity
- Cite theorems by name (e.g., "Proth's theorem (1878)")
- Target ~30-40% comment lines

## Testing

```bash
cargo test                                    # All unit tests
cargo run -- factorial --start 1 --end 100    # Quick verification
cargo run -- kbn --k 3 --base 2 --min-n 1 --max-n 1000
cargo run -- palindromic --base 10 --min-digits 1 --max-digits 9
cargo run -- cullen-woodall --min-n 1 --max-n 200
cargo run -- repunit --base 10 --min-n 2 --max-n 100
cargo run -- twin --k 3 --base 2 --min-n 1 --max-n 500
cargo bench                                   # Performance benchmarks
```

## Adding a New Search Form

1. Create `src/<form>.rs` with the standard pipeline (copy closest existing form)
2. Add checkpoint variant to `CheckpointData` enum in `checkpoint.rs`
3. Add CLI subcommand in `main.rs` (`Commands` enum) + dispatch in `cli.rs`
4. Add `pub mod <form>;` in `lib.rs`
5. Update `search_manager.rs` with block generation
6. Update `deploy.rs` with deployment support
7. Add unit tests with known values from OEIS

## Roadmap

See `docs/roadmaps/engine.md` for planned improvements and `docs/roadmaps/gwnum-flint.md` for external library integration.
