-- Migration 062: Add total_primes and total_tested to get_stats()
--
-- The website's proof-bar.tsx expects {total_primes, total_tested} in the
-- stats response. This updates get_stats() to include:
--   - total_primes: alias for total (backward-compatible)
--   - total_tested: SUM of total_tested from search_jobs
--
-- All existing fields are preserved — fully backward-compatible.

CREATE OR REPLACE FUNCTION get_stats()
RETURNS JSON
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    result JSON;
    tested BIGINT;
BEGIN
    -- Aggregate total_tested from search_jobs (not in materialized view)
    SELECT COALESCE(SUM(sj.total_tested), 0)::BIGINT
    INTO tested
    FROM search_jobs sj;

    -- Try materialized view first (instant, no table scan)
    SELECT json_build_object(
        'total', total,
        'total_primes', total,
        'total_tested', tested,
        'by_form', by_form,
        'largest_digits', largest_digits,
        'largest_expression', largest_expression
    ) INTO result
    FROM mv_dashboard_stats
    LIMIT 1;

    -- Fallback to live query if materialized view is empty
    IF result IS NULL THEN
        SELECT json_build_object(
            'total', (SELECT COUNT(*) FROM primes),
            'total_primes', (SELECT COUNT(*) FROM primes),
            'total_tested', tested,
            'by_form', COALESCE(
                (SELECT json_agg(row_to_json(t))
                 FROM (
                     SELECT form, COUNT(*) AS count
                     FROM primes
                     GROUP BY form
                     ORDER BY count DESC
                 ) t),
                '[]'::json
            ),
            'largest_digits', COALESCE(
                (SELECT digits FROM primes ORDER BY digits DESC LIMIT 1),
                0
            ),
            'largest_expression', (
                SELECT expression FROM primes ORDER BY digits DESC LIMIT 1
            )
        ) INTO result;
    END IF;

    RETURN result;
END;
$$;
