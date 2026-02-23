-- Operator badge system: achievement badges for gamification and retention.
-- Badges are defined in badge_definitions and granted to operators via
-- operator_badges. The check_and_grant_badges() function evaluates all
-- badge criteria and inserts any newly earned badges.

CREATE TABLE IF NOT EXISTS badge_definitions (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tier        TEXT NOT NULL CHECK (tier IN ('bronze', 'silver', 'gold')),
    threshold   BIGINT NOT NULL,
    metric      TEXT NOT NULL,
    icon        TEXT NOT NULL DEFAULT 'award',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE badge_definitions IS 'Achievement badge definitions with tier, threshold, and metric';

-- Seed 14 badges across primes, blocks, hours, trust, streak, and fleet metrics.
INSERT INTO badge_definitions (id, name, description, tier, threshold, metric, icon) VALUES
    ('first_prime',  'First Blood',     'Discover your first prime',              'bronze', 1,     'primes_found',     'zap'),
    ('primes_10',    'Prime Finder',    'Discover 10 primes',                     'silver', 10,    'primes_found',     'search'),
    ('primes_100',   'Prime Hunter',    'Discover 100 primes',                    'gold',   100,   'primes_found',     'target'),
    ('blocks_100',   'Centurion',       'Complete 100 work blocks',               'bronze', 100,   'blocks_completed', 'box'),
    ('blocks_1000',  'Workhorse',       'Complete 1,000 work blocks',             'silver', 1000,  'blocks_completed', 'cpu'),
    ('blocks_10000', 'Iron Node',       'Complete 10,000 work blocks',            'gold',   10000, 'blocks_completed', 'server'),
    ('hours_100',    'Contributor',     'Contribute 100 core-hours',              'bronze', 100,   'core_hours',       'clock'),
    ('hours_1000',   'Powerhouse',      'Contribute 1,000 core-hours',            'silver', 1000,  'core_hours',       'battery-charging'),
    ('hours_10000',  'Compute Legend',   'Contribute 10,000 core-hours',           'gold',   10000, 'core_hours',       'flame'),
    ('trust_2',      'Reliable',        'Reach trust level 2 (proven)',            'bronze', 2,     'trust_level',      'shield'),
    ('trust_3',      'Trusted',         'Reach trust level 3 (trusted)',           'silver', 3,     'trust_level',      'shield-check'),
    ('trust_4',      'Core Member',     'Reach trust level 4 (core)',              'gold',   4,     'trust_level',      'crown'),
    ('streak_50',    'Streak Master',   '50 consecutive valid results',            'silver', 50,    'consecutive_valid','trending-up'),
    ('multi_node',   'Fleet Commander', 'Run 5 or more nodes simultaneously',      'silver', 5,     'worker_count',     'network')
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS operator_badges (
    operator_id UUID NOT NULL REFERENCES operators(id) ON DELETE CASCADE,
    badge_id    TEXT NOT NULL REFERENCES badge_definitions(id) ON DELETE CASCADE,
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (operator_id, badge_id)
);

COMMENT ON TABLE operator_badges IS 'Badges earned by operators (grant ledger)';

CREATE INDEX IF NOT EXISTS idx_operator_badges_operator ON operator_badges(operator_id);

-- PL/pgSQL function to check all badge criteria and grant any newly earned badges.
-- Returns the number of newly granted badges.
CREATE OR REPLACE FUNCTION check_and_grant_badges(p_operator_id UUID)
RETURNS INTEGER AS $$
DECLARE
    v_primes_found      INTEGER;
    v_blocks_completed  BIGINT;
    v_core_hours        FLOAT8;
    v_trust_level       SMALLINT;
    v_consecutive_valid INTEGER;
    v_worker_count      BIGINT;
    v_granted           INTEGER := 0;
    v_badge             RECORD;
    v_metric_value      BIGINT;
BEGIN
    -- Gather metrics for this operator
    SELECT COALESCE(primes_found, 0) INTO v_primes_found
    FROM operators WHERE id = p_operator_id;

    SELECT COUNT(*) INTO v_blocks_completed
    FROM work_blocks WHERE volunteer_id = p_operator_id AND status = 'completed';

    SELECT COALESCE(SUM(cpu_core_hours::float8), 0) INTO v_core_hours
    FROM resource_contributions WHERE operator_id = p_operator_id;

    SELECT COALESCE(trust_level, 1) INTO v_trust_level
    FROM operator_trust WHERE volunteer_id = p_operator_id;

    SELECT COALESCE(consecutive_valid, 0) INTO v_consecutive_valid
    FROM operator_trust WHERE volunteer_id = p_operator_id;

    SELECT COUNT(*) INTO v_worker_count
    FROM operator_nodes WHERE volunteer_id = p_operator_id
    AND last_heartbeat > NOW() - INTERVAL '5 minutes';

    -- Check each badge definition
    FOR v_badge IN SELECT * FROM badge_definitions LOOP
        -- Map metric name to value
        CASE v_badge.metric
            WHEN 'primes_found'     THEN v_metric_value := v_primes_found;
            WHEN 'blocks_completed' THEN v_metric_value := v_blocks_completed;
            WHEN 'core_hours'       THEN v_metric_value := v_core_hours::bigint;
            WHEN 'trust_level'      THEN v_metric_value := v_trust_level;
            WHEN 'consecutive_valid' THEN v_metric_value := v_consecutive_valid;
            WHEN 'worker_count'     THEN v_metric_value := v_worker_count;
            ELSE v_metric_value := 0;
        END CASE;

        -- Grant badge if threshold met and not already granted
        IF v_metric_value >= v_badge.threshold THEN
            INSERT INTO operator_badges (operator_id, badge_id)
            VALUES (p_operator_id, v_badge.id)
            ON CONFLICT DO NOTHING;

            IF FOUND THEN
                v_granted := v_granted + 1;
            END IF;
        END IF;
    END LOOP;

    RETURN v_granted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION check_and_grant_badges IS 'Evaluate all badge criteria for an operator and grant any newly earned badges';
