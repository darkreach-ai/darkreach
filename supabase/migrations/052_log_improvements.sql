-- Log improvements: indexes for time-bucketed stats, request correlation, and context queries.
--
-- The system_logs table already exists (created in an earlier migration). These
-- additions enable the new /api/observability/logs/stats, /search, and /stream
-- endpoints without changing existing insert paths.

-- Composite index for time-bucketed stats queries (GROUP BY date_trunc + level)
CREATE INDEX IF NOT EXISTS idx_system_logs_ts_level ON system_logs (ts, level);

-- Request ID column for HTTP request correlation
ALTER TABLE system_logs ADD COLUMN IF NOT EXISTS request_id TEXT;
CREATE INDEX IF NOT EXISTS idx_system_logs_request_id ON system_logs (request_id) WHERE request_id IS NOT NULL;

-- GIN index on context JSONB for field-level queries (e.g. context->>'form' = 'kbn')
CREATE INDEX IF NOT EXISTS idx_system_logs_context ON system_logs USING gin (context jsonb_path_ops);
