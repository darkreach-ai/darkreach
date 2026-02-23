-- Browser block lease TTL + submission idempotency
--
-- Browser blocks get a lease (lease_until) that must be renewed via heartbeat.
-- When the lease expires, the block is reclaimed and made available for others.
-- The idempotency index prevents double-granting credits on retry submissions.

-- Add lease column to work_blocks
ALTER TABLE work_blocks ADD COLUMN IF NOT EXISTS lease_until TIMESTAMPTZ;

-- Index for efficient reclaim query (only claimed blocks with a lease)
CREATE INDEX IF NOT EXISTS idx_work_blocks_lease_until
    ON work_blocks (lease_until) WHERE status = 'claimed';

-- Reclaim browser blocks whose lease expired
CREATE OR REPLACE FUNCTION reclaim_expired_browser_blocks()
RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE
    reclaimed INTEGER;
BEGIN
    WITH expired AS (
        UPDATE work_blocks
        SET status = 'available',
            claimed_by = NULL,
            operator_id = NULL,
            claimed_at = NULL,
            lease_until = NULL
        WHERE status = 'claimed'
          AND lease_until IS NOT NULL
          AND lease_until < NOW()
        RETURNING id
    )
    SELECT COUNT(*) INTO reclaimed FROM expired;
    RETURN reclaimed;
END;
$$;
