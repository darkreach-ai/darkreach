-- Migration 045: Pipeline Stage Tracking for Work Blocks
--
-- Adds multi-stage pipeline support to work_blocks, enabling different workers
-- to handle different stages of the search pipeline. Cheap stages (sieve, screen)
-- eliminate candidates before expensive ones (test, proof), improving throughput.
--
-- Pipeline stages:
--   sieve  -> Run form-specific sieve to eliminate composites with small factors
--   screen -> Quick 2-round Miller-Rabin pre-screen on sieve survivors
--   test   -> Full 25-round MR or form-specific primality test
--   proof  -> Deterministic proof attempt (Pocklington/Morrison/BLS/Proth/LLR)
--
-- Backward compatible: DEFAULT 'sieve' means existing blocks continue to work
-- as before. The stage_data column stores intermediate results (e.g., sieve
-- survivor indices) as JSONB for hand-off between stages.

-- Add pipeline stage tracking to work_blocks
ALTER TABLE work_blocks ADD COLUMN IF NOT EXISTS pipeline_stage TEXT
    NOT NULL DEFAULT 'sieve'
    CHECK (pipeline_stage IN ('sieve', 'screen', 'test', 'proof'));

-- Stage checkpoint stores intermediate results (e.g., sieve survivor indices)
ALTER TABLE work_blocks ADD COLUMN IF NOT EXISTS stage_data JSONB;

-- Index for stage-aware work claiming: workers can request blocks at a specific
-- pipeline stage, avoiding contention between sieve workers and test workers.
CREATE INDEX IF NOT EXISTS idx_work_blocks_pipeline_stage
    ON work_blocks(pipeline_stage, status)
    WHERE status = 'available';

-- Document the new columns
COMMENT ON COLUMN work_blocks.pipeline_stage IS
    'Current pipeline stage: sieve -> screen (quick MR) -> test (full PRP) -> proof';
COMMENT ON COLUMN work_blocks.stage_data IS
    'Intermediate results from previous stage (e.g., sieve survivor indices as JSON array)';
