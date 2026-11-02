# Campaign Configurations

Pre-built project configurations targeting specific open problems in
computational number theory. Each TOML file defines a complete campaign that
can be imported into the darkreach coordinator.

## Usage

```bash
cargo run -- project import --file projects/campaigns/crus-sierp-base3.toml
```

After import, the orchestration engine will automatically manage phase
activation, work distribution, and progress tracking.

## Campaigns

| File | Problem | Form |
|------|---------|------|
| `crus-sierp-base3.toml` | Sierpinski conjecture base 3 | k*3^n+1 |
| `crus-sierp-base5.toml` | Sierpinski conjecture base 5 | k*5^n+1 |
| `crus-riesel-base3.toml` | Riesel conjecture base 3 | k*3^n-1 |

## Background

These campaigns target the **Conjectures 'R Us** (CRUS) project, which aims
to prove the Sierpinski and Riesel conjectures for all bases up to 1030.
For each base b, the Sierpinski conjecture identifies the smallest k such
that k*b^n+1 is composite for all n >= 1; the Riesel conjecture does the
same for k*b^n-1.

The base-2 Sierpinski/Riesel problems are well-covered by PrimeGrid and
Seventeen or Bust. These campaigns focus on **non-base-2** problems where
there is less competition and more opportunity for new discoveries.

## References

- CRUS project: http://www.noprimeleftbehind.net/crus/
- Condensed status table: https://www.rieselprime.de/Others/CRUS_tab.htm
- Mersenne Forum discussions: https://www.mersenneforum.org/
- PrimeGrid Sierpinski/Riesel Base 5: https://primegrid.fandom.com/wiki/Sierpinski/Riesel_Base_5_Problem
