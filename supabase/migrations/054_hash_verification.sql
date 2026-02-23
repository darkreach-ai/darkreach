-- Server-side hash verification and block performance tracking.
--
-- Adds hash_verified flag to work_blocks so the server can mark blocks whose
-- result_hash has been independently verified by recomputing the canonical
-- SHA-256 digest. Also adds duration_ms for per-block performance tracking
-- to enable future adaptive block sizing and calibration.

ALTER TABLE work_blocks ADD COLUMN hash_verified BOOLEAN DEFAULT FALSE;
ALTER TABLE work_blocks ADD COLUMN duration_ms BIGINT;
