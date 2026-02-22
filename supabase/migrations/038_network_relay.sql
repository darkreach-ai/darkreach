-- Phase 5: Network relay — relay sieve cache and audit log.
--
-- Relay nodes are high-bandwidth, public-IP operator nodes promoted by the
-- AI engine to cache sieve data and serve it to nearby compute workers,
-- reducing coordinator bandwidth bottleneck.

-- relay_sieve_cache: tracks which sieve hashes are cached on which relay nodes.
CREATE TABLE IF NOT EXISTS relay_sieve_cache (
    relay_worker_id TEXT NOT NULL REFERENCES operator_nodes(worker_id),
    sieve_hash      TEXT NOT NULL REFERENCES shared_sieves(hash),
    cached_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (relay_worker_id, sieve_hash)
);

CREATE INDEX IF NOT EXISTS idx_relay_sieve_cache_hash
    ON relay_sieve_cache (sieve_hash);

-- relay_events: audit log for relay promotions/demotions and cache activity.
CREATE TABLE IF NOT EXISTS relay_events (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    worker_id  TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('promoted', 'demoted', 'cache_hit', 'cache_miss')),
    detail     JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_relay_events_worker
    ON relay_events (worker_id, created_at DESC);

-- Add relay_hint column to work_blocks for relay-aware work assignment.
ALTER TABLE work_blocks ADD COLUMN IF NOT EXISTS relay_hint TEXT;

-- RLS policies (same open pattern as shared_sieves)
ALTER TABLE relay_sieve_cache ENABLE ROW LEVEL SECURITY;
CREATE POLICY "read_relay_sieve_cache" ON relay_sieve_cache FOR SELECT USING (true);
CREATE POLICY "write_relay_sieve_cache" ON relay_sieve_cache FOR INSERT WITH CHECK (true);
CREATE POLICY "delete_relay_sieve_cache" ON relay_sieve_cache FOR DELETE USING (true);

ALTER TABLE relay_events ENABLE ROW LEVEL SECURITY;
CREATE POLICY "read_relay_events" ON relay_events FOR SELECT USING (true);
CREATE POLICY "write_relay_events" ON relay_events FOR INSERT WITH CHECK (true);
