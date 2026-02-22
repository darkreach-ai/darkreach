-- Phase 4: Storage pooling — shared sieve blob cache.
--
-- Hash-addressed immutable sieve blobs. Workers upload computed sieves
-- and subsequent workers download instead of recomputing.

CREATE TABLE IF NOT EXISTS shared_sieves (
    hash        TEXT PRIMARY KEY,
    form        TEXT NOT NULL,
    k           BIGINT NOT NULL,
    base        INTEGER NOT NULL,
    min_n       BIGINT NOT NULL,
    max_n       BIGINT NOT NULL,
    sieve_limit BIGINT NOT NULL,
    blob        BYTEA NOT NULL,
    size_bytes  BIGINT NOT NULL,
    uploaded_by TEXT,
    hit_count   BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shared_sieves_form
    ON shared_sieves (form);

CREATE INDEX IF NOT EXISTS idx_shared_sieves_lookup
    ON shared_sieves (form, k, base, min_n, max_n, sieve_limit);

ALTER TABLE shared_sieves ENABLE ROW LEVEL SECURITY;
CREATE POLICY "read_shared_sieves" ON shared_sieves FOR SELECT USING (true);
CREATE POLICY "write_shared_sieves" ON shared_sieves FOR INSERT WITH CHECK (true);
CREATE POLICY "update_shared_sieves" ON shared_sieves FOR UPDATE USING (true);
