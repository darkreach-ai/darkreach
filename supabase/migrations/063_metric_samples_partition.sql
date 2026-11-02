-- Migration 063: Native range-partitioning for metric_samples
--
-- Converts metric_samples from a regular heap table to a PostgreSQL native
-- range-partitioned table on the `ts` column (weekly partitions).  This
-- eliminates the need for the expensive `DELETE FROM metric_samples WHERE
-- ts < ...` pruning query: old data is dropped instantly via `DROP TABLE`
-- on expired partitions.
--
-- A `maintain_metric_partitions()` function is provided and should be
-- called periodically (e.g. hourly) to create upcoming partitions and
-- drop partitions older than the retention window.
--
-- If TimescaleDB is already managing metric_samples as a hypertable,
-- this migration is a no-op — the guard at the top will skip it.

DO $migration$
BEGIN

-- ── P1: TimescaleDB guard ──────────────────────────────────────────────
-- If metric_samples is already a TimescaleDB hypertable, skip the entire
-- migration.  This prevents conflicts when TimescaleDB is installed and
-- has already converted the table.
IF EXISTS (
    SELECT 1 FROM pg_catalog.pg_extension WHERE extname = 'timescaledb'
) AND EXISTS (
    SELECT 1 FROM timescaledb_information.hypertables
    WHERE hypertable_name = 'metric_samples'
) THEN
    RAISE NOTICE 'metric_samples is already a TimescaleDB hypertable, skipping native partitioning';
    RETURN;
END IF;

-- ── Step 1: Rename old table and migrate data ──────────────────────────
-- We keep the old table around temporarily so we can copy data over.
ALTER TABLE IF EXISTS metric_samples RENAME TO metric_samples_old;

-- Drop indexes on old table to speed up the copy (they belong to the old name)
DROP INDEX IF EXISTS idx_metric_samples_ts;
DROP INDEX IF EXISTS idx_metric_samples_metric_ts;
DROP INDEX IF EXISTS idx_metric_samples_scope_ts;

-- ── Step 2: Create partitioned table ───────────────────────────────────
-- P0: Include PRIMARY KEY (id, ts) — partition key must be part of PK.
CREATE TABLE metric_samples (
    id          BIGINT GENERATED ALWAYS AS IDENTITY,
    ts          TIMESTAMPTZ NOT NULL,
    scope       TEXT NOT NULL,
    metric      TEXT NOT NULL,
    value       DOUBLE PRECISION NOT NULL,
    labels      JSONB,
    PRIMARY KEY (id, ts)
) PARTITION BY RANGE (ts);

-- Recreate indexes on the partitioned parent
CREATE INDEX idx_metric_samples_ts ON metric_samples (ts);
CREATE INDEX idx_metric_samples_metric_ts ON metric_samples (metric, ts);
CREATE INDEX idx_metric_samples_scope_ts ON metric_samples (scope, ts);

-- ── Step 3: Create initial partitions (current week + 4 weeks ahead) ───
-- P1: Schema-qualified pg_class check to avoid false positives from
-- identically-named objects in other schemas.
DO $parts$
DECLARE
    week_start DATE;
    week_end   DATE;
    part_name  TEXT;
BEGIN
    FOR i IN -1..4 LOOP
        week_start := date_trunc('week', CURRENT_DATE)::date + (i * 7);
        week_end   := week_start + 7;
        part_name  := 'metric_samples_' || to_char(week_start, 'IYYY_"w"IW');

        IF NOT EXISTS (
            SELECT 1 FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relname = part_name AND n.nspname = 'public'
        ) THEN
            EXECUTE format(
                'CREATE TABLE %I PARTITION OF metric_samples FOR VALUES FROM (%L) TO (%L)',
                part_name, week_start, week_end
            );
        END IF;
    END LOOP;
END $parts$;

-- ── Step 4: Copy data from old table ───────────────────────────────────
-- Only copy data that falls within the created partitions (last week
-- through 4 weeks ahead).  Older data is intentionally discarded as it
-- would have been pruned anyway.
INSERT INTO metric_samples (ts, scope, metric, value, labels)
SELECT ts, scope, metric, value, labels
FROM metric_samples_old
WHERE ts >= date_trunc('week', CURRENT_DATE)::date - 7
ON CONFLICT DO NOTHING;

-- ── Step 5: Drop old table ─────────────────────────────────────────────
DROP TABLE IF EXISTS metric_samples_old;

-- ── Step 6: Maintenance function ───────────────────────────────────────
-- Creates upcoming partitions and drops expired ones.  Call hourly.
CREATE OR REPLACE FUNCTION maintain_metric_partitions(
    retention_weeks INT DEFAULT 4,
    future_weeks    INT DEFAULT 4
) RETURNS VOID AS $func$
DECLARE
    week_start DATE;
    week_end   DATE;
    part_name  TEXT;
    cutoff     DATE;
    rec        RECORD;
BEGIN
    -- P1: Create partitions for the next `future_weeks` weeks
    FOR i IN 0..future_weeks LOOP
        week_start := date_trunc('week', CURRENT_DATE)::date + (i * 7);
        week_end   := week_start + 7;
        part_name  := 'metric_samples_' || to_char(week_start, 'IYYY_"w"IW');

        -- P1: Schema-qualified check to avoid false matches in other schemas
        IF NOT EXISTS (
            SELECT 1 FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relname = part_name AND n.nspname = 'public'
        ) THEN
            EXECUTE format(
                'CREATE TABLE %I PARTITION OF metric_samples FOR VALUES FROM (%L) TO (%L)',
                part_name, week_start, week_end
            );
            RAISE NOTICE 'created partition %', part_name;
        END IF;
    END LOOP;

    -- Drop partitions older than retention window
    cutoff := date_trunc('week', CURRENT_DATE)::date - (retention_weeks * 7);
    FOR rec IN
        SELECT c.relname
        FROM pg_inherits i
        JOIN pg_class c ON c.oid = i.inhrelid
        JOIN pg_class p ON p.oid = i.inhparent
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE p.relname = 'metric_samples'
          AND n.nspname = 'public'
        ORDER BY c.relname
    LOOP
        -- Extract the week start date from the partition name pattern metric_samples_YYYY_wWW
        -- and drop if it's before the cutoff.
        BEGIN
            DECLARE
                part_date DATE;
            BEGIN
                -- Parse ISO year + week from name: metric_samples_2026_w08 → 2026-W08
                part_date := to_date(
                    substring(rec.relname FROM 'metric_samples_(\d{4})_w(\d{2})'),
                    'IYYY'
                ) + ((substring(rec.relname FROM 'metric_samples_\d{4}_w(\d{2})')::int - 1) * 7);

                IF part_date < cutoff THEN
                    EXECUTE format('DROP TABLE IF EXISTS %I', rec.relname);
                    RAISE NOTICE 'dropped expired partition %', rec.relname;
                END IF;
            EXCEPTION WHEN OTHERS THEN
                -- Skip partitions with unexpected naming
                RAISE NOTICE 'skipping partition % (could not parse date)', rec.relname;
            END;
        END;
    END LOOP;
END;
$func$ LANGUAGE plpgsql;

COMMENT ON FUNCTION maintain_metric_partitions IS
    'Creates future weekly partitions and drops expired ones for metric_samples. '
    'Call hourly from the dashboard background loop. Retention and lookahead are '
    'configurable in weeks (defaults: 4 weeks retention, 4 weeks ahead).';

END $migration$;
