Plan and execute a prime discovery campaign.

## Steps

1. **Pick a target form** — Use the ROI table below and `docs/roadmaps/research.md`
2. **Configure sieve** — Optimal depth for candidate size (see `docs/roadmaps/engine.md`)
3. **Run search** — Choose subcommand and range
4. **Verify discovery** — Cross-verify with different software (LLR/PFGW/PRST)
5. **Publish** — Follow pipeline in `docs/roadmaps/research.md`

## ROI Comparison

| Search Type | Core-years/discovery | Provable? | Competition | Best for |
|------------|---------------------|-----------|-------------|----------|
| Sierpinski/Riesel (non-base-2) | 1-10 | Yes | Low | Quick wins, conjecture solving |
| Palindromic record | 100-1,000 | Yes (BLS) | 1 team | High visibility |
| Factorial | ~2,300 | Yes | PrimeGrid | Auto-provable results |
| Wagstaff PRP | ~3,000 | No | None | Prestige, untested territory |

## All 12 Search Commands

```bash
# k·b^n ± 1 — Best ROI for Sierpinski/Riesel conjectures
cargo run -- kbn --k <k> --base <base> --min-n 1 --max-n 1000000

# Factorial — Auto-provable via Pocklington/Morrison
cargo run -- factorial --start 1 --end 10000

# Palindromic — BLS-provable record attempts
cargo run -- palindromic --base 10 --min-digits 1 --max-digits 15

# Near-repdigit — BLS-provable palindromic variants
cargo run -- near-repdigit --base 10 --min-digits 1 --max-digits 15

# Primorial — p# ± 1, auto-provable
cargo run -- primorial --start 2 --end 10000

# Cullen/Woodall — n·2^n ± 1
cargo run -- cullen-woodall --min-n 1 --max-n 1000

# Wagstaff — (2^p+1)/3, PRP only, no competition
cargo run -- wagstaff --min-p 2 --max-p 10000

# Carol/Kynea — (2^n±1)²−2
cargo run -- carol-kynea --min-n 2 --max-n 500

# Twin primes — p and p+2 both prime
cargo run -- twin --k 3 --base 2 --min-n 1 --max-n 1000

# Sophie Germain — p and 2p+1 both prime
cargo run -- sophie-germain --k 3 --base 2 --min-n 1 --max-n 1000

# Repunit — R(b,n) = (b^n−1)/(b−1)
cargo run -- repunit --base 10 --min-n 2 --max-n 1000

# Generalized Fermat — b^(2^n)+1
cargo run -- gen-fermat --base 2 --min-n 1 --max-n 30
```

## Form-Specific Strategies

### Sierpinski/Riesel (BEST ROI)
Pick bases from Conjectures 'R Us with 1-3 remaining k-values and low search limits.

### Palindromic Record
Near-repdigit construction for BLS-provable results at 3M+ digits. Use `near-repdigit` subcommand.

### Factorial
Sequential search, auto-provable. PFGW acceleration for large n (>10K digits).

### Wagstaff
(2^p+1)/3 for prime p. No active project — discovery gets immediate recognition. PRP only.

## Fleet Coordination

```bash
# Run as distributed worker claiming blocks from coordinator
cargo run -- --database-url $DATABASE_URL work --search-job-id <id>

# Or with HTTP coordinator
cargo run -- --coordinator http://coordinator:7001 kbn --k 3 --base 2 --min-n 1 --max-n 1000000
```

## Project Campaigns

Use the project system for structured campaigns:
```bash
# Import a TOML project definition
cargo run -- --database-url $DATABASE_URL project import projects/<name>.toml

# Orchestrate (30s tick loop advances phases automatically)
cargo run -- --database-url $DATABASE_URL project orchestrate <slug>
```

See `docs/roadmaps/projects.md` for project configuration format.

## Verification

| Form | Verify with |
|------|-------------|
| k·b^n+1 (Proth) | LLR, PFGW, PRST |
| k·b^n-1 (Riesel) | LLR, PFGW, PRST |
| n!±1 | PFGW, PRST |
| p#±1 | PFGW |
| Palindromic | PFGW, Primo (up to ~40K digits) |
| Wagstaff | PFGW (PRP only) |

Built-in 3-tier verification: `cargo run -- --database-url $DATABASE_URL verify`

## Publication

```
Day 0:     Discovery — DO NOT ANNOUNCE PUBLICLY
Day 0-7:   Cross-verify (different software + hardware)
Day 7-14:  Submit to t5k.org (with proof)
Month 1:   Update OEIS, arXiv preprint
Month 3-6: Journal paper
```

## References

- `docs/roadmaps/research.md` — Full strategy analysis
- `docs/roadmaps/engine.md` — Algorithm improvements
- `docs/roadmaps/ops.md` — Hardware optimization
- `docs/roadmaps/competitive-analysis.md` — Competitor landscape
