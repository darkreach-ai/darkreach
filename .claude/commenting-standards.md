# Academic Commenting Standards

This codebase serves as both a production prime-hunting tool and a teaching resource for computational number theory. Every file should be understandable by a CS/math graduate student encountering the code for the first time.

---

## Principles

1. **Explain "why" over "what"** — The code shows *what*; comments explain *why* this approach was chosen, what mathematical property it exploits, and what alternatives were considered.
2. **Reference the mathematics** — Cite theorems by name, link to OEIS sequences, reference papers. A reader should be able to trace every algorithm to its mathematical foundation.
3. **Document invariants and preconditions** — State what must be true for the code to be correct. These are the most valuable comments in the codebase.
4. **Assume a smart reader who lacks context** — Don't explain what a for-loop does. Do explain why the loop bound is `sqrt(order)` instead of `order`.
5. **Keep comments current** — A wrong comment is worse than no comment. When changing code, update its comments in the same edit.

---

## Rust Files

### Module-level doc comment (required on every file)

Every `.rs` file must begin with a `//!` doc comment block that includes:

```rust
//! # Module Name — One-Line Purpose
//!
//! Extended description (2-5 sentences) explaining what this module does,
//! what mathematical concepts it implements, and how it fits into the
//! larger system.
//!
//! ## Algorithm Overview
//!
//! Brief description of the core algorithm(s) with complexity.
//!
//! ## References
//!
//! - [Theorem/Paper Name](URL or citation)
//! - OEIS: [A002981](https://oeis.org/A002981) — Factorial primes n!+1
//! - Software: PrimeGrid, GIMPS, etc.
```

### Public items (functions, structs, enums, traits, constants)

Every `pub` item gets a `///` doc comment:

```rust
/// Proth's primality test for numbers of the form k·2^n + 1 where k < 2^n.
///
/// # Algorithm
///
/// 1. Find a quadratic non-residue `a` modulo p (Jacobi symbol (a/p) = -1).
/// 2. Compute a^((p-1)/2) mod p.
/// 3. If the result is p-1 (i.e., -1 mod p), then p is prime (deterministic).
///
/// # Mathematical Basis
///
/// Proth's theorem (1878): If p = k·2^n + 1 with k < 2^n, and there exists
/// an integer a such that a^((p-1)/2) ≡ -1 (mod p), then p is prime.
///
/// This is a strengthening of Fermat's little theorem that yields a
/// deterministic (not probabilistic) result.
///
/// # Complexity
///
/// O(n) squarings of n-bit numbers, i.e., O(n · M(n)) where M(n) is the
/// cost of multiplying two n-bit integers.
///
/// # Panics
///
/// Debug-asserts that k < 2^n (Proth form requirement).
///
/// # Returns
///
/// - `Some(true)` — p is prime (deterministic proof)
/// - `Some(false)` — p is composite
/// - `None` — test not applicable (k >= 2^n, or no non-residue found)
pub fn proth_test(k: u64, n: u64, candidate: &Integer) -> Option<bool> {
```

### Struct and enum fields

Document non-obvious fields:

```rust
/// Montgomery multiplication context for a fixed odd modulus.
///
/// All arithmetic is performed in Montgomery form: ā = a·R mod n,
/// where R = 2^64. This replaces division (35-90 cycles) with
/// multiply-and-shift (4-6 cycles) in modular arithmetic hot paths.
///
/// # Reference
///
/// Peter L. Montgomery, "Modular Multiplication Without Trial Division",
/// Mathematics of Computation, 44(170):519-521, 1985.
#[derive(Clone, Copy, Debug)]
pub struct MontgomeryCtx {
    /// The modulus. Must be odd and > 1.
    pub n: u64,
    /// -n⁻¹ mod 2^64, precomputed via Hensel lifting (6 Newton iterations).
    n_prime: u64,
    /// R mod n = 2^64 mod n. This is 1 in Montgomery form.
    r_mod_n: u64,
    /// R² mod n. Used to convert normal integers into Montgomery form
    /// via a single Montgomery multiplication: to_mont(a) = REDC(a · R²).
    r2_mod_n: u64,
}
```

### Inline comments

Use for:
- **Mathematical identities**: `// By Fermat's little theorem: a^(p-1) ≡ 1 (mod p)`
- **Non-obvious guards**: `// Skip: a % p == 0 would make Jacobi symbol undefined`
- **Performance rationale**: `// BSGS: O(√ord) instead of O(ord) for discrete log`
- **Correctness invariants**: `// Invariant: residues[i] ≡ n! mod primes[i] after this loop`

Do **not** use for:
- Restating the code: `// increment counter` before `counter += 1`
- Closing-brace labels: `} // end for`
- Commented-out code (delete it instead)

### Test functions

```rust
#[test]
fn proth_test_known_primes() {
    // Verify against known Proth primes from OEIS A080076.
    // 3·2^1+1=7, 3·2^2+1=13, 3·2^4+1=49 (composite), 3·2^5+1=97
    assert_eq!(proth_test(3, 1, &Integer::from(7)), Some(true));
    // ...
}
```

---

## TypeScript / React Files

### Module-level JSDoc (required on every file)

```typescript
/**
 * # WebSocket Hook — Real-time Coordination Data
 *
 * Manages a persistent WebSocket connection to the Rust backend for
 * receiving fleet status, search progress, and deployment updates.
 *
 * ## Data Flow
 *
 * Browser ←WebSocket→ Rust dashboard (port 8080)
 *   - Receives: fleet workers, managed searches, coordinator metrics
 *   - Sends: (nothing currently; read-only connection)
 *
 * Note: Prime data comes from Supabase, not this WebSocket.
 * The WebSocket is used only for coordination/operational data.
 *
 * ## Reconnection
 *
 * Uses exponential backoff: 1s → 2s → 4s → ... → 30s max, with
 * random jitter. Resets to 1s on successful connection.
 */
```

### Interfaces and types

```typescript
/**
 * Real-time status of a connected worker in the search fleet.
 *
 * Updated every 10 seconds via WebSocket from the coordinator.
 * Workers that miss 6 consecutive heartbeats (60s) are considered stale.
 */
export interface WorkerStatus {
  /** Unique identifier, typically the hostname. */
  worker_id: string;
  /** Number of candidates tested since this worker started. */
  tested: number;
  /** Human-readable description of the current candidate being tested. */
  current: string;
  // ...
}
```

### React components

```typescript
/**
 * # Prime Detail Dialog
 *
 * Modal dialog showing full details for a selected prime: expression,
 * digit count, proof method, verification status, and search parameters.
 *
 * ## User Actions
 *
 * - **Re-verify**: Triggers POST /api/primes/{id}/verify on the backend,
 *   which reconstructs and re-tests the candidate using GMP.
 * - **Permalink**: Navigates to /prime/?id=N for a standalone shareable page.
 *
 * ## Data Source
 *
 * Fetches full prime detail from Supabase (not WebSocket) via the
 * `usePrimes` hook's `fetchPrimeDetail()` method.
 */
```

### Hooks

```typescript
/**
 * Fetches and caches prime records from Supabase with filtering,
 * pagination, and sorting.
 *
 * ## State Management
 *
 * Maintains three pieces of state:
 * - `primes`: Paginated list (PrimeRecord[]) with offset/total
 * - `selectedPrime`: Full detail (PrimeDetail | null) for the detail dialog
 * - Loading/error state (implicit via React rendering)
 *
 * ## Caching Strategy
 *
 * No client-side cache — every filter/sort/page change triggers a fresh
 * Supabase query. This is acceptable because queries are fast (<100ms)
 * and the data changes infrequently.
 */
export function usePrimes() {
```

### Non-obvious logic in effects/callbacks

```typescript
useEffect(() => {
  // Debounce search input: wait 300ms after last keystroke before
  // triggering a Supabase query. This prevents hammering the database
  // while the user is still typing.
  const timer = setTimeout(() => {
    setDebouncedSearch(searchInput);
  }, 300);
  return () => clearTimeout(timer);
}, [searchInput]);
```

---

## What NOT to Comment

- **Self-evident code**: `const [loading, setLoading] = useState(true);` needs no comment.
- **Framework boilerplate**: Don't explain what `useEffect` does.
- **Type annotations that speak for themselves**: `digits: number` is clear.
- **Obvious function names**: `formatTime(timestamp)` doesn't need a comment saying "formats a timestamp."

---

## Density Guidelines

| File Type | Target Comment Ratio | Focus Areas |
|-----------|---------------------|-------------|
| Engine (algorithms) | ~30-40% comment lines | Math proofs, complexity, invariants, references |
| Server (infrastructure) | ~20-30% comment lines | Data flow, API contracts, error handling rationale |
| Frontend (UI) | ~15-25% comment lines | Data sources, user interactions, state management |

These are rough targets, not rigid rules. A 10-line function with a 15-line doc comment is fine if the algorithm warrants it. A 200-line React component might need only 20 lines of comments if the JSX is self-documenting.

---

## Checklist for Code Review

When reviewing or writing code, verify:

- [ ] File has a module-level doc comment with purpose, algorithm overview, and references
- [ ] Every `pub` item (Rust) or `export` (TypeScript) has a doc comment
- [ ] Mathematical algorithms cite their theorem/paper by name
- [ ] Non-obvious loop bounds, guards, and edge cases are explained
- [ ] Complexity (Big-O) is stated for any algorithm beyond O(n)
- [ ] OEIS sequences are linked where applicable
- [ ] Invariants and preconditions are documented
- [ ] Data flow between modules/services is explained at connection points
