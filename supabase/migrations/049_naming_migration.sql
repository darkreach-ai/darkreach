-- 048_naming_migration.sql
--
-- Complete the naming migration started in 025_operator_rename.sql:
--   volunteer_id → operator_id   (FK columns left unchanged in Phase 0)
--   worker_id    → node_id       (operator_nodes table, operator-facing)
--   worker_count → node_count    (leaderboard view)
--   fleet.*      → network.*     (metric_samples keys)
--   fleet_fit    → network_fit   (ai_engine_state JSON)
--
-- Backward-compatibility views from 025 are refreshed to reflect
-- the renamed columns.

BEGIN;

-- ============================================================
-- 1. Rename FK columns: volunteer_id → operator_id
-- ============================================================

ALTER TABLE operator_trust
    RENAME COLUMN volunteer_id TO operator_id;

ALTER TABLE operator_nodes
    RENAME COLUMN volunteer_id TO operator_id;

ALTER TABLE operator_credits
    RENAME COLUMN volunteer_id TO operator_id;

ALTER TABLE work_blocks
    RENAME COLUMN volunteer_id TO operator_id;

ALTER TABLE primes
    RENAME COLUMN volunteer_id TO operator_id;

ALTER TABLE verification_queue
    RENAME COLUMN original_volunteer_id TO original_operator_id;

-- ============================================================
-- 2. Rename operator_nodes worker columns → node
-- ============================================================

ALTER TABLE operator_nodes
    RENAME COLUMN worker_id TO node_id;

ALTER TABLE operator_nodes
    RENAME COLUMN worker_version TO node_version;

-- ============================================================
-- 3. Rename relay_sieve_cache column
-- ============================================================

ALTER TABLE relay_sieve_cache
    RENAME COLUMN relay_worker_id TO relay_node_id;

-- ============================================================
-- 4. Rename indexes to match new column names
-- ============================================================

ALTER INDEX idx_operator_nodes_volunteer
    RENAME TO idx_operator_nodes_operator;

ALTER INDEX idx_operator_credits_volunteer
    RENAME TO idx_operator_credits_operator;

-- operator_nodes unique index on worker_id
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'operator_nodes_worker_id_key') THEN
        ALTER INDEX operator_nodes_worker_id_key RENAME TO operator_nodes_node_id_key;
    END IF;
END $$;

-- ============================================================
-- 5. Recreate operator_leaderboard view with new column names
-- ============================================================

DROP VIEW IF EXISTS volunteer_leaderboard;
DROP VIEW IF EXISTS operator_leaderboard;

CREATE VIEW operator_leaderboard AS
SELECT
  o.id,
  o.username,
  o.team,
  o.credit,
  o.primes_found,
  o.joined_at,
  o.last_seen,
  COALESCE(ot.trust_level, 1) AS trust_level,
  COUNT(DISTINCT on_node.id) AS node_count
FROM operators o
LEFT JOIN operator_trust ot ON ot.operator_id = o.id
LEFT JOIN operator_nodes on_node ON on_node.operator_id = o.id
GROUP BY o.id, o.username, o.team, o.credit, o.primes_found, o.joined_at, o.last_seen, ot.trust_level
ORDER BY o.credit DESC;

-- Backward-compat alias
CREATE VIEW volunteer_leaderboard AS SELECT * FROM operator_leaderboard;

-- ============================================================
-- 6. Refresh backward-compatibility views (drop + recreate)
--    Views reference the underlying columns, which have changed.
-- ============================================================

DROP VIEW IF EXISTS volunteer_trust;
CREATE VIEW volunteer_trust AS
SELECT operator_id AS volunteer_id, consecutive_valid, total_valid, total_invalid, trust_level
FROM operator_trust;

DROP VIEW IF EXISTS volunteer_workers;
CREATE VIEW volunteer_workers AS
SELECT id, operator_id AS volunteer_id, node_id AS worker_id, hostname, cores,
       cpu_model, os, arch, ram_gb, has_gpu, gpu_model, gpu_vram_gb,
       node_version AS worker_version, update_channel, registered_at, last_heartbeat
FROM operator_nodes;

DROP VIEW IF EXISTS credit_log;
CREATE VIEW credit_log AS
SELECT id, operator_id AS volunteer_id, block_id, credit, reason, granted_at
FROM operator_credits;

-- ============================================================
-- 7. Recreate check_and_grant_badges() with new column names
-- ============================================================

CREATE OR REPLACE FUNCTION check_and_grant_badges(p_operator_id UUID)
RETURNS INTEGER AS $$
DECLARE
    v_primes_found      INTEGER;
    v_blocks_completed  BIGINT;
    v_core_hours        FLOAT8;
    v_trust_level       SMALLINT;
    v_consecutive_valid INTEGER;
    v_node_count        BIGINT;
    v_granted           INTEGER := 0;
    v_badge             RECORD;
    v_metric_value      BIGINT;
BEGIN
    -- Gather metrics for this operator
    SELECT COALESCE(primes_found, 0) INTO v_primes_found
    FROM operators WHERE id = p_operator_id;

    SELECT COUNT(*) INTO v_blocks_completed
    FROM work_blocks WHERE operator_id = p_operator_id AND status = 'completed';

    SELECT COALESCE(SUM(cpu_core_hours::float8), 0) INTO v_core_hours
    FROM resource_contributions WHERE operator_id = p_operator_id;

    SELECT COALESCE(trust_level, 1) INTO v_trust_level
    FROM operator_trust WHERE operator_id = p_operator_id;

    SELECT COALESCE(consecutive_valid, 0) INTO v_consecutive_valid
    FROM operator_trust WHERE operator_id = p_operator_id;

    SELECT COUNT(*) INTO v_node_count
    FROM operator_nodes WHERE operator_id = p_operator_id
    AND last_heartbeat > NOW() - INTERVAL '5 minutes';

    -- Check each badge definition
    FOR v_badge IN SELECT * FROM badge_definitions LOOP
        -- Map metric name to value
        CASE v_badge.metric
            WHEN 'primes_found'     THEN v_metric_value := v_primes_found;
            WHEN 'blocks_completed' THEN v_metric_value := v_blocks_completed;
            WHEN 'core_hours'       THEN v_metric_value := v_core_hours::bigint;
            WHEN 'trust_level'      THEN v_metric_value := v_trust_level;
            WHEN 'consecutive_valid' THEN v_metric_value := v_consecutive_valid;
            WHEN 'worker_count'     THEN v_metric_value := v_node_count;
            ELSE v_metric_value := 0;
        END CASE;

        -- Grant badge if threshold met and not already granted
        IF v_metric_value >= v_badge.threshold THEN
            INSERT INTO operator_badges (operator_id, badge_id)
            VALUES (p_operator_id, v_badge.id)
            ON CONFLICT DO NOTHING;

            IF FOUND THEN
                v_granted := v_granted + 1;
            END IF;
        END IF;
    END LOOP;

    RETURN v_granted;
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- 8. Data migration: metric_samples fleet.* → network.*
-- ============================================================

UPDATE metric_samples
SET metric = REPLACE(metric, 'fleet.', 'network.')
WHERE metric LIKE 'fleet.%';

-- ============================================================
-- 9. Data migration: ai_engine_state fleet_fit → network_fit
-- ============================================================

UPDATE ai_engine_state
SET scoring_weights = scoring_weights || jsonb_build_object(
    'network_fit', scoring_weights->'fleet_fit'
)
WHERE scoring_weights ? 'fleet_fit';

UPDATE ai_engine_state
SET scoring_weights = scoring_weights - 'fleet_fit'
WHERE scoring_weights ? 'fleet_fit';

COMMIT;
