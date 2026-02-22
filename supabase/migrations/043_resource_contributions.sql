-- Resource contribution tracking and credit conversion rates.
--
-- Phase 6 of the Resources roadmap: operators earn credits for all resource
-- contributions (CPU, GPU, storage, bandwidth), each with its own conversion rate.

-- Per-block resource contribution record
CREATE TABLE IF NOT EXISTS resource_contributions (
    id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    operator_id      UUID NOT NULL REFERENCES operators(id),
    block_id         BIGINT NOT NULL,
    cpu_core_hours   NUMERIC NOT NULL DEFAULT 0,
    gpu_hours        NUMERIC NOT NULL DEFAULT 0,
    storage_gb_hours NUMERIC NOT NULL DEFAULT 0,
    bandwidth_gb     NUMERIC NOT NULL DEFAULT 0,
    credits_earned   NUMERIC NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_resource_contributions_operator ON resource_contributions (operator_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_resource_contributions_block ON resource_contributions (block_id);

-- Credit conversion rates per resource type
CREATE TABLE IF NOT EXISTS resource_credit_rates (
    resource_type    TEXT PRIMARY KEY CHECK (resource_type IN ('cpu', 'gpu', 'storage', 'bandwidth')),
    credits_per_unit NUMERIC NOT NULL DEFAULT 1.0,
    unit_label       TEXT NOT NULL DEFAULT '',
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default rates
INSERT INTO resource_credit_rates (resource_type, credits_per_unit, unit_label) VALUES
    ('cpu', 1.0, 'core-hour'),
    ('gpu', 5.0, 'gpu-hour'),
    ('storage', 0.1, 'gb-hour'),
    ('bandwidth', 0.5, 'gb')
ON CONFLICT DO NOTHING;

-- RLS: read-open for both tables, write-open for resource_contributions
ALTER TABLE resource_contributions ENABLE ROW LEVEL SECURITY;
ALTER TABLE resource_credit_rates ENABLE ROW LEVEL SECURITY;

CREATE POLICY resource_contributions_read ON resource_contributions FOR SELECT USING (true);
CREATE POLICY resource_contributions_write ON resource_contributions FOR INSERT WITH CHECK (true);
CREATE POLICY resource_credit_rates_read ON resource_credit_rates FOR SELECT USING (true);
