-- Batch work block claiming for operator nodes.
--
-- Extends the operator claim pattern from single-block (LIMIT 1) to batch
-- (LIMIT p_count) with atomic lease setting. Reduces DB round-trips by
-- 5-10x as the network scales beyond 20 nodes.
--
-- Pattern mirrors the internal `claim_work_blocks()` (migration 028) but
-- adds hardware matching, form preference filters, and optional lease TTL
-- for browser contribute batches.

CREATE OR REPLACE FUNCTION claim_operator_blocks(
    p_operator_id UUID,
    p_worker_id TEXT,
    p_count INTEGER,
    p_cores INTEGER,
    p_ram_gb INTEGER,
    p_has_gpu BOOLEAN,
    p_os TEXT,
    p_arch TEXT,
    p_preferred_forms TEXT[],
    p_excluded_forms TEXT[],
    p_gpu_runtime TEXT,
    p_gpu_vram_gb INTEGER,
    p_lease_minutes INTEGER DEFAULT NULL
) RETURNS TABLE(block_id BIGINT, search_job_id BIGINT, block_start BIGINT, block_end BIGINT)
LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT wb.id
        FROM work_blocks wb
        JOIN search_jobs sj ON sj.id = wb.search_job_id
        WHERE wb.status = 'available'
          -- Hardware capability filters (identical to claim_operator_block_with_prefs)
          AND (
            NOT (sj.params ? 'min_cores')
            OR (
              jsonb_typeof(sj.params->'min_cores') = 'number'
              AND (sj.params->>'min_cores')::int <= p_cores
            )
          )
          AND (
            NOT (sj.params ? 'min_ram_gb')
            OR (
              jsonb_typeof(sj.params->'min_ram_gb') = 'number'
              AND (sj.params->>'min_ram_gb')::int <= p_ram_gb
            )
          )
          AND (
            NOT (sj.params ? 'requires_gpu')
            OR lower(sj.params->>'requires_gpu') <> 'true'
            OR p_has_gpu = TRUE
          )
          AND (
            NOT (sj.params ? 'required_os')
            OR (p_os IS NOT NULL AND lower(sj.params->>'required_os') = lower(p_os))
          )
          AND (
            NOT (sj.params ? 'required_arch')
            OR (p_arch IS NOT NULL AND lower(sj.params->>'required_arch') = lower(p_arch))
          )
          -- Form preference filters
          AND (p_preferred_forms = '{}' OR sj.search_type = ANY(p_preferred_forms))
          AND (p_excluded_forms = '{}' OR sj.search_type <> ALL(p_excluded_forms))
          -- GPU filters
          AND (
            NOT (sj.params ? 'gpu_runtime')
            OR (sj.params->>'gpu_runtime') IS NULL
            OR (p_gpu_runtime IS NOT NULL AND lower(sj.params->>'gpu_runtime') = lower(p_gpu_runtime))
          )
          AND (
            NOT (sj.params ? 'min_gpu_vram_gb')
            OR (
              jsonb_typeof(sj.params->'min_gpu_vram_gb') = 'number'
              AND (sj.params->>'min_gpu_vram_gb')::int <= COALESCE(p_gpu_vram_gb, 0)
            )
          )
        ORDER BY wb.id
        FOR UPDATE SKIP LOCKED
        LIMIT p_count
    )
    UPDATE work_blocks wb
    SET status = 'claimed',
        claimed_by = p_worker_id,
        operator_id = p_operator_id,
        claimed_at = NOW(),
        lease_until = CASE
            WHEN p_lease_minutes IS NOT NULL
            THEN NOW() + (p_lease_minutes || ' minutes')::interval
            ELSE wb.lease_until
        END
    FROM claimed
    WHERE wb.id = claimed.id
    RETURNING wb.id AS block_id, wb.search_job_id, wb.block_start, wb.block_end;
END;
$$;
