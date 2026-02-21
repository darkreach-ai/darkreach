-- AI Engine Phases 6-8: weight learning, agent integration, fleet rebalancing
--
-- Phase 6: Adds component_scores to decisions for outcome correlation,
--          index for unmeasured outcomes.
-- Phase 8: Materialized view for worker speed tracking from work blocks.

-- Store the 7 component scores alongside each decision for reward correlation
ALTER TABLE ai_engine_decisions
    ADD COLUMN IF NOT EXISTS component_scores JSONB;

-- Index for efficiently finding decisions that need outcome measurement
CREATE INDEX IF NOT EXISTS idx_ai_engine_decisions_pending_outcome
    ON ai_engine_decisions (created_at DESC)
    WHERE outcome IS NULL AND decision_type = 'create_project';

-- Worker speed materialized view: aggregates per-worker, per-form performance
-- from completed work blocks. Used by the AI engine for fleet rebalancing.
CREATE MATERIALIZED VIEW IF NOT EXISTS worker_speed AS
SELECT
    wb.claimed_by AS worker_id,
    sj.search_type AS form,
    COUNT(*) AS blocks_completed,
    AVG(EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at)))::float8 AS avg_block_secs,
    CASE
        WHEN SUM(EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at))) > 0 THEN
            SUM(wb.tested)::float8 / SUM(EXTRACT(EPOCH FROM (wb.completed_at - wb.claimed_at)))
        ELSE 0
    END AS candidates_per_sec
FROM work_blocks wb
JOIN search_jobs sj ON sj.id = wb.search_job_id
WHERE wb.status = 'completed'
  AND wb.claimed_by IS NOT NULL
  AND wb.completed_at IS NOT NULL
  AND wb.claimed_at IS NOT NULL
  AND wb.completed_at > wb.claimed_at
GROUP BY wb.claimed_by, sj.search_type;

-- Index on the materialized view for fast lookups
CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_speed_worker_form
    ON worker_speed (worker_id, form);
