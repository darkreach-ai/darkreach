-- Migration: Hash operator API keys at rest
--
-- Adds an `api_key_hash` column to store SHA-256 hashes of operator API keys.
-- During migration period, both plaintext and hash lookup are supported.
-- After migration is complete, the plaintext `api_key` column can be dropped.

-- Add hash column
ALTER TABLE operators ADD COLUMN IF NOT EXISTS api_key_hash TEXT;

-- Populate hashes from existing plaintext keys
UPDATE operators SET api_key_hash = encode(sha256(api_key::bytea), 'hex')
WHERE api_key_hash IS NULL AND api_key IS NOT NULL;

-- Index for hash-based lookups
CREATE INDEX IF NOT EXISTS idx_operators_api_key_hash ON operators (api_key_hash);
