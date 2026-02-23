-- Adaptive verification methods for the work-block verification queue.
--
-- Phase 8 of the verification roadmap: BOINC-style adaptive replication.
-- Instead of always doing full re-computation, route trusted operators to
-- cheaper verification methods (hash comparison, spot-checking) while
-- maintaining full replication for untrusted operators.
--
-- Verification method hierarchy:
--   hash_compare       (~0% cost) — compare SHA-256 result hashes
--   spot_check         (~5% cost) — re-test deterministic random subset
--   full_recomputation (100%)     — complete re-run of the block

-- ── New columns on verification_queue ────────────────────────────────

ALTER TABLE verification_queue
  ADD COLUMN verification_method TEXT NOT NULL DEFAULT 'full_recomputation'
    CHECK (verification_method IN ('hash_compare', 'spot_check', 'full_recomputation')),
  ADD COLUMN original_result_hash TEXT,
  ADD COLUMN verification_result_hash TEXT,
  ADD COLUMN spot_check_pct SMALLINT,
  ADD COLUMN spot_check_seed BIGINT,
  ADD COLUMN verification_cost_pct SMALLINT NOT NULL DEFAULT 100,
  ADD COLUMN escalated_from BIGINT REFERENCES verification_queue(id);

-- Index for cost analytics queries
CREATE INDEX idx_verification_queue_method
    ON verification_queue (verification_method);

-- ── Verification cost analytics view ─────────────────────────────────

CREATE VIEW verification_cost_summary AS
SELECT
  date_trunc('day', completed_at) AS day,
  verification_method,
  COUNT(*) AS blocks,
  AVG(verification_cost_pct)::NUMERIC(5,1) AS avg_cost_pct,
  SUM(CASE WHEN status = 'matched' THEN 1 ELSE 0 END) AS matched,
  SUM(CASE WHEN status = 'conflict' THEN 1 ELSE 0 END) AS conflicts
FROM verification_queue
WHERE completed_at IS NOT NULL
GROUP BY 1, 2;
