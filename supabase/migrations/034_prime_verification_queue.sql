-- Phase 4: Distributed Prime Verification Queue
--
-- Adds a prime-level re-verification system distinct from the work-block
-- verification_queue (migration 028). When a prime is discovered, it is
-- enqueued here for independent re-verification by network nodes running
-- the 3-tier verify_prime() pipeline. Quorum-based consensus tags primes
-- as 'verified-distributed' once enough independent confirmations arrive.

-- ── Prime Verification Queue ─────────────────────────────────────
-- Individual verification tasks. Multiple rows per prime (one per verifier).

CREATE TABLE IF NOT EXISTS prime_verification_queue (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    prime_id        BIGINT NOT NULL REFERENCES primes(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'claimed', 'verified', 'failed')),
    claimed_by      TEXT,
    claimed_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    verification_tier   SMALLINT,
    verification_method TEXT,
    result_detail   JSONB,
    error_reason    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast lookup for pending tasks (claim query hot path)
CREATE INDEX IF NOT EXISTS idx_pvq_pending
    ON prime_verification_queue (id)
    WHERE status = 'pending';

-- Stale recovery: find claimed entries older than threshold
CREATE INDEX IF NOT EXISTS idx_pvq_claimed_at
    ON prime_verification_queue (claimed_at)
    WHERE status = 'claimed';

-- Quorum counting: how many verifications per prime
CREATE INDEX IF NOT EXISTS idx_pvq_prime_id
    ON prime_verification_queue (prime_id);

-- Prevent same worker from verifying the same prime twice
CREATE UNIQUE INDEX IF NOT EXISTS idx_pvq_prime_worker
    ON prime_verification_queue (prime_id, claimed_by)
    WHERE claimed_by IS NOT NULL;

-- ── Prime Verification Summary ───────────────────────────────────
-- One row per prime, tracks quorum progress.

CREATE TABLE IF NOT EXISTS prime_verification_summary (
    prime_id            BIGINT PRIMARY KEY REFERENCES primes(id) ON DELETE CASCADE,
    required_quorum     SMALLINT NOT NULL DEFAULT 2,
    verified_count      SMALLINT NOT NULL DEFAULT 0,
    failed_count        SMALLINT NOT NULL DEFAULT 0,
    highest_tier        SMALLINT NOT NULL DEFAULT 0,
    quorum_met          BOOLEAN NOT NULL DEFAULT FALSE,
    quorum_met_at       TIMESTAMPTZ,
    discoverer_worker   TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Stale Recovery Function ──────────────────────────────────────
-- Resets claimed entries older than p_stale_seconds back to pending.
-- Checks worker liveness via the workers table heartbeat.

CREATE OR REPLACE FUNCTION reclaim_stale_prime_verifications(p_stale_seconds INT)
RETURNS INT
LANGUAGE plpgsql
AS $$
DECLARE
    reclaimed INT;
BEGIN
    UPDATE prime_verification_queue pvq
    SET status = 'pending',
        claimed_by = NULL,
        claimed_at = NULL
    WHERE pvq.status = 'claimed'
      AND pvq.claimed_at < NOW() - (p_stale_seconds || ' seconds')::INTERVAL
      AND NOT EXISTS (
          SELECT 1 FROM workers w
          WHERE w.worker_id = pvq.claimed_by
            AND w.last_heartbeat > NOW() - INTERVAL '120 seconds'
      );

    GET DIAGNOSTICS reclaimed = ROW_COUNT;
    RETURN reclaimed;
END;
$$;
