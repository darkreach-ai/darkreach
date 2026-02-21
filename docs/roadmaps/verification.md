# Verification Engine Roadmap

Extending the 3-tier verification pipeline into a full classification, distributed verification, and cross-form discovery system.

**Key files:** `src/verify.rs`, `src/classify.rs` (new), `src/db/primes.rs`, `src/dashboard/routes_verify.rs`

**See also:** [database.md](database.md) for schema scaling, [engine.md](engine.md) for primality testing primitives, [network.md](network.md) for distributed work claiming, [public-compute.md](public-compute.md) for volunteer quorum logic.

---

## Current State

**What works:**
- 3-tier verification pipeline in `src/verify.rs`: deterministic re-proof → BPSW+MR → PFGW cross-check
- Expression parsers for all 12 forms (reconstruct `rug::Integer` from stored expression strings)
- Tier-1 deterministic re-proofs for kbn, factorial, primorial (Proth, LLR, Pocklington, Morrison)
- Tier-2 independent BPSW + 10-round fixed-base MR (different code path from discovery)
- Tier-3 PFGW subprocess cross-verification (independent GWNUM implementation)
- Volunteer quorum logic: trust-based adaptive replication (BOINC model) with 5 trust levels
- `is_provable_form()` classification (9 provable, 3 PRP-only)
- `verify` CLI subcommand + `/api/verify` dashboard endpoint
- 48 unit tests covering parsers, all 3 tiers, quorum logic, and edge cases

**What's missing:**
- **No multi-classification**: each prime has a single `form` label assigned at discovery time. A factorial prime that's also palindromic is only tagged as "factorial".
- **No form-specific invariant checks**: twin primes aren't verified to actually be a twin pair (p and p+2 both prime), Sophie Germain primes aren't checked for 2p+1 primality, palindromes aren't verified for structural symmetry.
- **No distributed verification**: all verification runs locally via `darkreach verify` or `/api/verify`. No way to distribute verification work across the network.
- **No cross-form discovery**: after verifying a prime, no scan for membership in other form families (e.g., is this kbn prime also a twin? Is this factorial prime also palindromic?).
- **No tag-based querying**: the browse page and API can only filter by `form`, not by properties like "twin", "safe-prime", "verified-tier-1".

---

## Architecture: Form + Tags Model

### Design

The existing `form TEXT` column stays as the **singular discovery-time label** — it records how the prime was found and is part of the unique constraint `(form, expression)`. This is backward-compatible and semantically correct: a prime's discovery form is immutable.

A new `tags TEXT[]` column provides **multi-classification**. Tags are additive labels assigned at two points:

1. **Discovery time**: basic structural tags derived from the form (e.g., a `twin` discovery gets tags `["twin", "kbn"]`)
2. **Verification time**: full cross-form scan adds property tags (e.g., `["palindromic", "safe-prime", "verified-tier-1"]`)

### Tag Taxonomy

| Category | Examples | Assigned When |
|----------|----------|---------------|
| **Structural** (form names) | `factorial`, `kbn`, `palindromic`, `twin`, `repunit` | Discovery time |
| **Proof** | `deterministic`, `probabilistic`, `prp-only` | Discovery time |
| **Property** | `twin`, `safe-prime`, `sophie-germain`, `palindromic`, `near-repdigit` | Verification time |
| **Verification** | `verified-tier-1`, `verified-tier-2`, `verified-tier-3`, `verified-distributed` | Verification time |
| **Record** | `world-record`, `project-record`, `personal-best` | Post-verification |

### Query Patterns

```sql
-- Find all primes that are both palindromic AND twin
SELECT * FROM primes WHERE tags @> ARRAY['palindromic', 'twin'];

-- Find all verified primes
SELECT * FROM primes WHERE tags @> ARRAY['verified-tier-1'];

-- Count primes by tag
SELECT unnest(tags) AS tag, COUNT(*) FROM primes GROUP BY tag ORDER BY count DESC;
```

The `@>` containment operator uses the GIN index for efficient lookup.

---

## Phase 1: Tags Column + DB Types

Add the `tags TEXT[]` column to the `primes` table, update Rust types, and backfill existing primes with their basic form tag.

Estimated effort: 2-3 days.

### 1.1 Migration: Add Tags Column

**Current:** `primes` table has `form TEXT NOT NULL` as the only classification field.

**Target:** Add `tags TEXT[] NOT NULL DEFAULT '{}'` with a GIN index for containment queries.

**Migration:**
```sql
-- Add tags column with GIN index
ALTER TABLE primes ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}';
CREATE INDEX idx_primes_tags ON primes USING GIN (tags);

-- Backfill: every existing prime gets its form as a tag
UPDATE primes SET tags = ARRAY[form];

-- Add proof-type tags based on proof_method
UPDATE primes SET tags = tags || ARRAY['deterministic']
WHERE proof_method = 'deterministic';

UPDATE primes SET tags = tags || ARRAY['probabilistic']
WHERE proof_method = 'probabilistic';
```

**Rationale:** `TEXT[]` with GIN is PostgreSQL's standard approach for tag/label systems. The `@>` containment operator is O(1) per tag lookup via the inverted index. Array storage is more efficient than a junction table for a fixed, small tag set (typically 2-5 tags per prime).

**Files:** `supabase/migrations/NNN_tags_column.sql`, `src/db/mod.rs` (PrimeRecord struct), `src/db/primes.rs`

### 1.2 Update Rust Types

**Current:** `PrimeRecord` and `PrimeDetail` structs have `form: String` but no tags field.

**Target:** Add `tags: Vec<String>` to both structs. Update all queries that read from `primes` to include `tags`.

**Files:** `src/db/mod.rs`, `src/db/primes.rs`

### 1.3 Update Insert Path

**Current:** `insert_prime()` and `insert_prime_sync()` write `form` but no tags.

**Target:** Accept `tags: &[&str]` parameter. At minimum, the discovery form is always included as a tag.

```rust
pub async fn insert_prime(
    &self,
    form: &str,
    expression: &str,
    digits: u64,
    search_params: &str,
    proof_method: &str,
    certificate: Option<&str>,
    tags: &[&str],          // NEW
) -> Result<()> { ... }
```

**Rationale:** Discovery-time tagging ensures every prime has at least one tag from birth. The engine modules pass basic tags; the verification pipeline adds more later.

**Files:** `src/db/primes.rs`, all 12 search form modules (update `insert_prime_sync` calls)

### 1.4 Tag Update API

**Current:** No way to modify tags on an existing prime.

**Target:** Add `update_prime_tags()` and `add_prime_tags()` DB methods.

```rust
/// Replace all tags on a prime.
pub async fn update_prime_tags(&self, prime_id: i64, tags: &[&str]) -> Result<()>;

/// Append tags without removing existing ones (idempotent via array_cat + DISTINCT).
pub async fn add_prime_tags(&self, prime_id: i64, new_tags: &[&str]) -> Result<()>;
```

**Files:** `src/db/primes.rs`

---

## Phase 2: Classification Engine

Build `src/classify.rs` — a module that determines which tags a prime should have based on its form, expression, value, and properties.

Estimated effort: 3-5 days.

### 2.1 Discovery-Time Classification

**Current:** Primes are tagged only with their discovery `form`.

**Target:** At discovery time, assign all applicable structural and proof tags.

**Algorithm:**
```
fn classify_at_discovery(form, expression, proof_method, candidate) -> Vec<String>:
    tags = [form]

    // Proof tags
    if proof_method == "deterministic":
        tags.push("deterministic")
    else:
        tags.push("probabilistic")
    if !is_provable_form(form):
        tags.push("prp-only")

    // Cross-form structural tags assigned at discovery
    // (cheap checks only — full scan deferred to verification)
    match form:
        "twin" => tags.extend(["twin", "kbn"])
        "sophie_germain" => tags.extend(["sophie-germain", "kbn"])
        "cullen_woodall" => tags.extend(["kbn"])
        "carol_kynea" => tags.extend(["kbn"])
        "gen_fermat" => tags.extend(["kbn"])
        "near_repdigit" => tags.extend(["near-repdigit", "palindromic"])
        _ => ()

    return deduplicate(tags)
```

**Rationale:** Discovery-time classification is cheap (no primality tests needed) and ensures every prime enters the database with meaningful tags. Forms that reuse `kbn::test_prime` are tagged as `kbn` too.

**Files:** `src/classify.rs` (new), all 12 search form modules

### 2.2 Verification-Time Classification

**Current:** Verification only confirms primality — it doesn't add tags.

**Target:** After successful verification, run classification checks and add tags.

**Algorithm:**
```
fn classify_at_verification(prime, candidate, verify_result) -> Vec<String>:
    tags = []

    // Verification tier tags
    match verify_result:
        Verified { tier: 1, .. } => tags.push("verified-tier-1")
        Verified { tier: 2, .. } => tags.push("verified-tier-2")
        Verified { tier: 3, .. } => tags.push("verified-tier-3")

    // Cheap property checks (milliseconds)
    if is_palindrome(candidate, 10):
        tags.push("palindromic")
    if is_twin_prime(candidate):        // test candidate ± 2
        tags.push("twin")
    if is_safe_prime(candidate):        // test (candidate - 1) / 2
        tags.push("safe-prime")
    if is_sophie_germain(candidate):    // test 2*candidate + 1
        tags.push("sophie-germain")

    return tags
```

**Rationale:** Some property checks require an additional primality test (twin: test p±2, Sophie Germain: test 2p+1, safe prime: test (p-1)/2). These are cheap for small primes but expensive for large ones, so they're done at verification time rather than discovery time.

**Files:** `src/classify.rs`, `src/verify.rs`

### 2.3 Digit-Size-Gated Classification

**Current:** N/A.

**Target:** Gate expensive cross-form checks by digit count to avoid wasting compute on impractical tests.

| Check | Cost | Gate |
|-------|------|------|
| Palindrome check | O(digits) string comparison | Always |
| Twin check (p±2 prime?) | One MR test | < 50,000 digits |
| Sophie Germain check (2p+1 prime?) | One MR test | < 50,000 digits |
| Safe prime check ((p-1)/2 prime?) | One MR test | < 50,000 digits |
| Repunit membership | Expression parsing | Always |
| Near-repdigit structure | Digit analysis | < 100,000 digits |

**Rationale:** For a 1M-digit prime, testing whether p+2 is also prime takes as long as the original primality test. Classification should be practical, not aspirational.

**Files:** `src/classify.rs`

---

## Phase 3: Form-Specific Invariant Validation

Extend verification to check form-specific mathematical invariants beyond "is it prime?".

Estimated effort: 1-2 weeks.

### 3.1 Twin Prime Pair Verification

**Current:** Twin primes are stored with expression "k*b^n +/- 1" and verified as a single number (the smaller twin, k*b^n - 1).

**Target:** Verify both twins: confirm that both k*b^n - 1 AND k*b^n + 1 are prime.

**Algorithm:**
```
fn verify_twin_invariant(expression, candidate) -> InvariantResult:
    p_minus = candidate               // k*b^n - 1 (already verified)
    p_plus = candidate + 2            // k*b^n + 1
    if !is_probably_prime(p_plus, 25):
        return InvariantFailed("p+2 is composite")
    return InvariantVerified("twin pair confirmed")
```

**Rationale:** A twin prime discovery claims TWO primes are prime. Current verification only checks one. This closes a logical gap — a corrupted result could have a prime p but composite p+2.

**Files:** `src/verify.rs`

### 3.2 Sophie Germain Relation Verification

**Current:** Sophie Germain primes are stored as the base prime p. The safe prime 2p+1 is not verified.

**Target:** Verify that 2p+1 is also prime.

**Algorithm:**
```
fn verify_sophie_germain_invariant(candidate) -> InvariantResult:
    safe = 2 * candidate + 1
    if !is_probably_prime(safe, 25):
        return InvariantFailed("2p+1 is composite")
    return InvariantVerified("Sophie Germain relation confirmed")
```

**Files:** `src/verify.rs`

### 3.3 Palindrome Structure Verification

**Current:** Palindromic primes are stored as raw decimal strings and verified for primality.

**Target:** Also verify palindromic structure — the decimal representation reads the same forwards and backwards.

**Algorithm:**
```
fn verify_palindrome_invariant(candidate) -> InvariantResult:
    s = candidate.to_string_radix(10)
    if s != s.chars().rev().collect::<String>():
        return InvariantFailed("not a palindrome")
    if s.len() % 2 == 0:
        return InvariantFailed("even-digit palindrome (should have been skipped)")
    return InvariantVerified("palindrome structure confirmed")
```

**Rationale:** A data corruption or expression parsing bug could store a non-palindromic number as palindromic. This structural check is O(digits) — essentially free.

**Files:** `src/verify.rs`

### 3.4 Factorial/Primorial Identity Verification

**Current:** Factorial primes are verified by reconstructing n!±1 and testing primality. But the reconstruction itself isn't verified against the stored expression.

**Target:** Verify that the reconstructed value matches the expression AND that the relationship holds (e.g., candidate = n! + 1, and candidate - 1 = n!).

**Algorithm:**
```
fn verify_factorial_invariant(n, candidate, is_plus) -> InvariantResult:
    factorial_n = Integer::factorial(n)
    expected = if is_plus { factorial_n + 1 } else { factorial_n - 1 }
    if candidate != expected:
        return InvariantFailed("candidate != n!±1")
    return InvariantVerified("factorial identity confirmed")
```

**Files:** `src/verify.rs`

### 3.5 Wagstaff Form Verification

**Current:** Wagstaff primes (2^p+1)/3 are verified for primality only.

**Target:** Also verify the structural identity: 3 * candidate = 2^p + 1, and p is prime.

**Files:** `src/verify.rs`

### 3.6 Cullen/Woodall Parameter Verification

**Current:** Cullen (n*2^n+1) and Woodall (n*2^n-1) primes are verified via kbn path.

**Target:** Verify the Cullen/Woodall constraint: k must equal n (i.e., k=n in k*2^n±1).

**Files:** `src/verify.rs`

---

## Phase 4: Distributed Verification Queue

Move verification from a local single-machine operation to a distributed queue that network nodes can claim work from.

Estimated effort: 1-2 weeks.

### 4.1 Verification Queue Table

**Current:** Verification is triggered manually via CLI or API. No queue, no tracking.

**Target:** A `verification_queue` table that tracks pending, in-progress, and completed verifications.

**Migration:**
```sql
CREATE TABLE verification_queue (
    id            BIGSERIAL PRIMARY KEY,
    prime_id      BIGINT NOT NULL REFERENCES primes(id),
    tier          SMALLINT NOT NULL,                     -- 1, 2, or 3
    status        TEXT NOT NULL DEFAULT 'pending',        -- pending, claimed, verified, failed
    claimed_by    TEXT,                                   -- worker_id
    claimed_at    TIMESTAMPTZ,
    completed_at  TIMESTAMPTZ,
    result        JSONB,                                  -- VerifyResult serialized
    attempt       SMALLINT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_vq_status ON verification_queue (status) WHERE status = 'pending';
CREATE INDEX idx_vq_prime_tier ON verification_queue (prime_id, tier);
```

**Rationale:** Separating verification state from the `primes` table avoids write contention and allows multiple verification tiers to run concurrently on the same prime.

**Files:** `supabase/migrations/NNN_verification_queue.sql`, `src/db/verification.rs` (new)

### 4.2 Work Claiming (FOR UPDATE SKIP LOCKED)

**Current:** N/A (no distributed verification).

**Target:** Reuse the proven `FOR UPDATE SKIP LOCKED` pattern from `src/pg_worker.rs` for verification work.

**Algorithm:**
```sql
-- Claim a pending verification job
UPDATE verification_queue
SET status = 'claimed', claimed_by = $1, claimed_at = NOW(), attempt = attempt + 1
WHERE id = (
    SELECT id FROM verification_queue
    WHERE status = 'pending'
    ORDER BY tier ASC, created_at ASC  -- prioritize tier-1 (cheapest)
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
RETURNING *;
```

**Rationale:** `FOR UPDATE SKIP LOCKED` is the battle-tested pattern used by the search work distribution system. It provides exactly-once claiming without contention.

**Files:** `src/db/verification.rs`, `src/pg_worker.rs` (refactor shared claiming logic)

### 4.3 Automatic Queue Population

**Current:** N/A.

**Target:** Automatically enqueue verification jobs when new primes are inserted.

**Options:**
1. **Trigger-based**: PostgreSQL trigger on `INSERT INTO primes` creates verification jobs
2. **Application-side**: `insert_prime()` also enqueues verification jobs in the same transaction

**Recommendation:** Application-side (option 2) for explicitness and testability. The trigger approach hides side effects.

```rust
pub async fn insert_prime_with_verification(&self, ...) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    // Insert prime
    let prime_id = sqlx::query_scalar("INSERT INTO primes ... RETURNING id")
        .fetch_one(&mut *tx).await?;
    // Enqueue tier-1 verification (or tier-2 if no deterministic proof)
    let tier = if is_provable_form(form) { 1 } else { 2 };
    sqlx::query("INSERT INTO verification_queue (prime_id, tier) VALUES ($1, $2)")
        .bind(prime_id).bind(tier)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
```

**Files:** `src/db/primes.rs`, `src/db/verification.rs`

### 4.4 Quorum Aggregation

**Current:** `verify.rs` has `required_quorum()` and `required_quorum_high_value()` functions.

**Target:** Integrate quorum logic with the verification queue so that a prime needs N independent verifications before being tagged `verified-distributed`.

**Algorithm:**
```
fn check_quorum(prime_id, form, digits, trust_levels) -> QuorumResult:
    required = max(required_quorum_high_value(t, form, digits) for t in trust_levels)
    verified_count = count(verification_queue WHERE prime_id AND status = 'verified')
    if verified_count >= required:
        add_tag(prime_id, "verified-distributed")
        return QuorumMet
    return QuorumPending(verified_count, required)
```

**Files:** `src/verify.rs`, `src/db/verification.rs`

### 4.5 Stale Claim Recovery

**Current:** `pg_worker.rs` reclaims stale work blocks after timeout.

**Target:** Apply the same pattern to verification claims. Claims older than 10 minutes without completion are reset to `pending`.

```sql
-- Reclaim stale verification jobs
UPDATE verification_queue
SET status = 'pending', claimed_by = NULL, claimed_at = NULL
WHERE status = 'claimed'
AND claimed_at < NOW() - INTERVAL '10 minutes';
```

**Files:** `src/db/verification.rs`

---

## Phase 5: API + Frontend

Expose tags in the REST API and build UI for tag-based filtering, display, and statistics.

Estimated effort: 1 week.

### 5.1 Tag Filtering in REST API

**Current:** `/api/primes` supports filtering by `form`, `min_digits`, `max_digits`, `sort`.

**Target:** Add `tags` query parameter for containment filtering.

```
GET /api/primes?tags=twin,verified-tier-1&min_digits=1000
```

**SQL:** `WHERE tags @> $1::text[]` (uses GIN index).

**Files:** `src/dashboard/routes_verify.rs` or new `routes_primes.rs`, `src/db/primes.rs`

### 5.2 Tag Chips in Browse Page

**Current:** Browse page shows form, digits, expression, date.

**Target:** Display tags as colored chips next to each prime. Color coding by category:
- Structural: blue
- Proof: green/yellow
- Property: purple
- Verification: emerald

**Files:** `frontend/src/app/browse/page.tsx`, `frontend/src/components/tag-chip.tsx` (new)

### 5.3 Tag Filter UI

**Current:** Browse page has form dropdown and digit range filter.

**Target:** Add multi-select tag filter (checkboxes or chip-based).

**Files:** `frontend/src/app/browse/page.tsx`, `frontend/src/hooks/use-primes.ts`

### 5.4 Tag Statistics

**Current:** Dashboard stats page shows form distribution, digit distribution.

**Target:** Add tag distribution chart (bar chart showing count per tag).

```sql
SELECT unnest(tags) AS tag, COUNT(*) AS count
FROM primes
GROUP BY tag
ORDER BY count DESC;
```

**Files:** `frontend/src/app/stats/page.tsx`, `src/dashboard/routes_verify.rs`

### 5.5 Verification Status Dashboard

**Current:** No visibility into verification queue status.

**Target:** Dashboard page showing verification queue depth, in-progress jobs, completion rate, and quorum progress.

**Files:** `frontend/src/app/verification/page.tsx` (new), `src/dashboard/routes_verify.rs`

---

## Phase 6: Cross-Form Discovery

After verification, automatically scan primes for membership in other form families. This is the payoff of the tags architecture — discovering that a prime found as `factorial` is also `palindromic`, or that a `kbn` prime is also `twin`.

Estimated effort: 1-2 weeks.

### 6.1 Post-Verification Classification Scan

**Current:** Verification confirms primality but doesn't check for other form memberships.

**Target:** After verification completes, run the Phase 2 classification engine to discover cross-form memberships.

**Pipeline:**
```
verify_prime(detail)
  → if Verified:
      new_tags = classify_at_verification(detail, candidate, result)
      add_prime_tags(prime_id, new_tags)
      if new_tags contains cross-form discoveries:
          emit_event(CrossFormDiscovery { prime_id, new_tags })
```

**Files:** `src/verify.rs`, `src/classify.rs`, `src/events.rs`

### 6.2 Batch Cross-Form Scanner

**Current:** N/A.

**Target:** CLI command and background job to scan existing verified primes for cross-form memberships.

```
darkreach classify --batch --min-digits 1 --max-digits 50000
```

This processes primes in batches, running the classification engine on each and updating tags. Useful for backfilling tags on primes discovered before the classification engine existed.

**Files:** `src/cli.rs`, `src/classify.rs`

### 6.3 Cross-Form Discovery Events

**Current:** `events.rs` has `PrimeDiscovered` events.

**Target:** Add `CrossFormDiscovery` event type for when a prime is found to belong to an additional form family.

```rust
pub enum Event {
    PrimeDiscovered { ... },
    PrimeVerified { prime_id: i64, tier: u8, method: String },
    CrossFormDiscovery { prime_id: i64, new_tags: Vec<String>, expression: String },
}
```

**Rationale:** Cross-form discoveries are noteworthy events — finding that a known factorial prime is also palindromic is a publishable result.

**Files:** `src/events.rs`, `src/dashboard/websocket.rs`

### 6.4 Interesting Property Detection

**Current:** N/A.

**Target:** Detect mathematically interesting properties beyond form membership:

| Property | Check | Interest Level |
|----------|-------|----------------|
| Twin prime | p±2 is prime | High — OEIS A001359 |
| Safe prime | (p-1)/2 is prime | High — cryptographic relevance |
| Sophie Germain | 2p+1 is prime | High — OEIS A005384 |
| Balanced prime | (p_prev + p_next)/2 = p | Medium |
| Chen prime | p+2 is prime or semiprime | Medium |
| Palindromic in other bases | palindrome in base 2, 8, 16 | Low (expensive) |

**Gate:** Only for primes < 50,000 digits (property checks require full primality tests on related numbers).

**Files:** `src/classify.rs`

---

## File Impact Matrix

| File | Phase | Change |
|------|-------|--------|
| `supabase/migrations/NNN_tags_column.sql` | 1 | New migration: `tags TEXT[]`, GIN index, backfill |
| `supabase/migrations/NNN_verification_queue.sql` | 4 | New migration: `verification_queue` table |
| `src/db/mod.rs` | 1 | Add `tags: Vec<String>` to PrimeRecord, PrimeDetail |
| `src/db/primes.rs` | 1, 5 | Update insert/query to include tags, add tag filtering |
| `src/db/verification.rs` | 4 | New module: verification queue CRUD, claiming, quorum |
| `src/classify.rs` | 2, 6 | New module: discovery-time and verification-time classification |
| `src/verify.rs` | 3, 6 | Invariant checks, post-verification classification hook |
| `src/events.rs` | 6 | Add CrossFormDiscovery event type |
| `src/cli.rs` | 6 | Add `classify` subcommand for batch scanning |
| `src/dashboard/routes_verify.rs` | 5 | Tag filtering API, verification queue status |
| `src/dashboard/websocket.rs` | 6 | Broadcast cross-form discovery events |
| All 12 search form modules | 1 | Update `insert_prime_sync` calls to pass tags |
| `frontend/src/app/browse/page.tsx` | 5 | Tag chips, tag filter UI |
| `frontend/src/app/stats/page.tsx` | 5 | Tag distribution chart |
| `frontend/src/app/verification/page.tsx` | 5 | New page: verification queue dashboard |
| `frontend/src/components/tag-chip.tsx` | 5 | New component: colored tag chips |
| `frontend/src/hooks/use-primes.ts` | 5 | Add tags parameter to query hook |
