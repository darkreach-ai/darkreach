-- Migration: API key expiration tracking
--
-- Adds rotation timestamps and expiration enforcement to operator API keys.
-- Keys older than the configured rotation period (default 90 days) will be
-- rejected at authentication time.

-- Track when key was last rotated
ALTER TABLE operators ADD COLUMN IF NOT EXISTS key_rotated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Backfill: set to registration date for existing operators
UPDATE operators SET key_rotated_at = joined_at
WHERE key_rotated_at = NOW() AND joined_at IS NOT NULL;

-- Index for expiration queries (periodic cleanup)
CREATE INDEX IF NOT EXISTS idx_operators_key_rotated_at ON operators (key_rotated_at);
