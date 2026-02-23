-- Migration 049: AI Engine Phases 4-6 — Sieve tracking + Weight history
--
-- Item 2: Add sieve effectiveness columns to work_blocks so the AI engine
--         can analyze optimal sieve depth per form.
-- Item 5: Create scoring_weight_history table for auditing how EWA weights
--         evolve over time.

-- ── Item 2: Sieve effectiveness tracking ─────────────────────────
ALTER TABLE work_blocks ADD COLUMN IF NOT EXISTS sieved_out BIGINT DEFAULT 0;
ALTER TABLE work_blocks ADD COLUMN IF NOT EXISTS sieve_depth BIGINT;

-- ── Item 5: Weight learning history ──────────────────────────────
CREATE TABLE IF NOT EXISTS scoring_weight_history (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  tick_id BIGINT NOT NULL,
  weights JSONB NOT NULL,
  trigger TEXT NOT NULL,
  projects_completed BIGINT NOT NULL DEFAULT 0,
  recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_weight_history_recorded
  ON scoring_weight_history(recorded_at DESC);
