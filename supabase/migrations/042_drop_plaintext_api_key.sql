-- Migration: Drop plaintext API key column
--
-- Now that all authentication uses the SHA-256 hash (api_key_hash),
-- the plaintext api_key column is no longer needed. The registration
-- flow returns the key to the user once, then only the hash is stored.
--
-- IMPORTANT: This is irreversible. Ensure all operators have api_key_hash
-- populated before running this migration.

-- Safety check: ensure no operators are missing their hash
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM operators WHERE api_key_hash IS NULL) THEN
        RAISE EXCEPTION 'Cannot drop api_key column: some operators are missing api_key_hash';
    END IF;
END $$;

-- Drop the plaintext column
ALTER TABLE operators DROP COLUMN IF EXISTS api_key;

-- Make api_key_hash NOT NULL now that it's the primary auth column
ALTER TABLE operators ALTER COLUMN api_key_hash SET NOT NULL;
