-- Add tags column to primes table for multi-classification.
--
-- The existing `form` column stays as the singular discovery-time label (part of
-- the unique constraint). The new `tags` array enables multi-classification:
-- structural tags (form names), proof tags, property tags, and verification tags.
--
-- Uses GIN index for efficient containment queries: WHERE tags @> ARRAY['twin']

ALTER TABLE primes ADD COLUMN IF NOT EXISTS tags TEXT[] NOT NULL DEFAULT '{}';

CREATE INDEX IF NOT EXISTS idx_primes_tags ON primes USING GIN (tags);

-- Backfill: every existing prime gets its form as a tag
UPDATE primes SET tags = ARRAY[form] WHERE tags = '{}';

-- Add proof-type tags based on proof_method
UPDATE primes SET tags = array_cat(tags, ARRAY['deterministic'])
WHERE proof_method = 'deterministic' AND NOT ('deterministic' = ANY(tags));

UPDATE primes SET tags = array_cat(tags, ARRAY['probabilistic'])
WHERE proof_method = 'probabilistic' AND NOT ('probabilistic' = ANY(tags));
