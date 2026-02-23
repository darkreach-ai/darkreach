-- Add content-addressed result hash to work_blocks for tamper detection.
-- The WASM browser engine computes SHA-256("{type}:{start}:{end}:{tested}:{primes_json}")
-- and submits it alongside results. Enables cross-worker verification by comparing
-- hashes from independent computations of the same block.

ALTER TABLE work_blocks ADD COLUMN result_hash TEXT;

CREATE INDEX idx_work_blocks_result_hash ON work_blocks (result_hash)
    WHERE result_hash IS NOT NULL;
