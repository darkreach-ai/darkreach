-- Operator work preferences: form selection, CPU/RAM limits, GPU preference.
-- Allows operators to control which search forms they participate in and
-- how much of their machine's resources to dedicate.

CREATE TABLE IF NOT EXISTS operator_preferences (
    operator_id    UUID PRIMARY KEY REFERENCES operators(id) ON DELETE CASCADE,
    preferred_forms TEXT[] DEFAULT '{}',
    excluded_forms  TEXT[] DEFAULT '{}',
    max_cpu_pct     SMALLINT DEFAULT 100 CHECK (max_cpu_pct BETWEEN 10 AND 100),
    max_ram_gb      INTEGER,
    prefer_gpu      BOOLEAN DEFAULT FALSE,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE operator_preferences IS 'Per-operator work preferences for form selection and resource limits';
COMMENT ON COLUMN operator_preferences.preferred_forms IS 'If non-empty, only claim blocks for these search forms';
COMMENT ON COLUMN operator_preferences.excluded_forms IS 'Never claim blocks for these search forms';
COMMENT ON COLUMN operator_preferences.max_cpu_pct IS 'Maximum CPU percentage to use (10-100)';
COMMENT ON COLUMN operator_preferences.max_ram_gb IS 'Maximum RAM in GB to use (NULL = unlimited)';
COMMENT ON COLUMN operator_preferences.prefer_gpu IS 'Prefer GPU-accelerated work blocks when available';
