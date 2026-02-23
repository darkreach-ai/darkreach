-- AI Engine Phases 1-3: Node Profiling, Model Auto-Selection, Adaptive Block Sizing
--
-- Item 3: Add profiling columns to workers for performance-aware assignment.
-- Item 2: Add default_model to agent_roles for role-based model selection.
-- Item 5: Add unique index on worker_speed for CONCURRENTLY refresh.

-- Item 3: Node profiling columns
ALTER TABLE workers ADD COLUMN IF NOT EXISTS benchmark_score FLOAT8;
ALTER TABLE workers ADD COLUMN IF NOT EXISTS preferred_forms TEXT[];
ALTER TABLE workers ADD COLUMN IF NOT EXISTS cpu_model TEXT;
ALTER TABLE workers ADD COLUMN IF NOT EXISTS ram_gb INTEGER;

-- Item 2: Role default model (already exists as NOT NULL TEXT with default,
-- so this is a no-op if column exists; the column was added in the roles migration)
-- ALTER TABLE agent_roles ADD COLUMN IF NOT EXISTS default_model TEXT;

-- Item 5: Enable CONCURRENTLY refresh on worker_speed materialized view
-- Requires a unique index for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX IF NOT EXISTS worker_speed_unique
ON worker_speed (worker_id, form);
