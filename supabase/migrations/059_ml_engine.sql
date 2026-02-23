-- ML Engine tables for data-driven prime hunting optimization.
-- Supports: Thompson Sampling bandits, GP cost prediction, Bayesian search
-- optimization, and node intelligence (anomaly detection + affinity routing).

-- ── Thompson Sampling Bandit Arms ──────────────────────────────

CREATE TABLE IF NOT EXISTS ml_bandit_arms (
    form TEXT PRIMARY KEY,
    alpha DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    beta DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    mean_reward DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    reward_var DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    n_obs BIGINT NOT NULL DEFAULT 0,
    window_alpha DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    window_beta DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    window_n BIGINT NOT NULL DEFAULT 0,
    context_weights JSONB NOT NULL DEFAULT '[0.2, 0.2, 0.2, 0.2, 0.2]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE ml_bandit_arms IS
    'Thompson Sampling posterior state per search form for intelligent form selection';

-- ── GP Cost Model ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ml_gp_training (
    id BIGSERIAL PRIMARY KEY,
    form TEXT NOT NULL,
    log_digits DOUBLE PRECISION NOT NULL,
    node_class TEXT NOT NULL,
    sieve_depth_log DOUBLE PRECISION NOT NULL,
    log_secs DOUBLE PRECISION NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ml_gp_training_form ON ml_gp_training(form);

COMMENT ON TABLE ml_gp_training IS
    'GP training data points for cost prediction (cached from work_blocks)';

CREATE TABLE IF NOT EXISTS ml_gp_state (
    form TEXT PRIMARY KEY,
    n_points INTEGER NOT NULL DEFAULT 0,
    last_mape DOUBLE PRECISION,
    hyperparams JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE ml_gp_state IS
    'Serialized GP model hyperparameters per search form';

-- ── Bayesian Optimization ──────────────────────────────────────

CREATE TABLE IF NOT EXISTS ml_bayesopt_observations (
    id BIGSERIAL PRIMARY KEY,
    form TEXT NOT NULL,
    sieve_depth_log DOUBLE PRECISION NOT NULL,
    block_size_log DOUBLE PRECISION NOT NULL,
    digits DOUBLE PRECISION NOT NULL,
    throughput DOUBLE PRECISION NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ml_bayesopt_form ON ml_bayesopt_observations(form);

COMMENT ON TABLE ml_bayesopt_observations IS
    'BayesOpt observations: (sieve_depth, block_size) → throughput per form';

CREATE TABLE IF NOT EXISTS ml_bayesopt_state (
    form TEXT PRIMARY KEY,
    best_sieve_depth BIGINT NOT NULL DEFAULT 100000,
    best_block_size BIGINT NOT NULL DEFAULT 1000,
    best_throughput DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    n_evals BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE ml_bayesopt_state IS
    'Current best parameters and evaluation count per form for BayesOpt';

-- ── Node Intelligence ──────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ml_node_profiles (
    node_id TEXT PRIMARY KEY,
    node_class TEXT NOT NULL DEFAULT 'cpu_medium',
    avg_throughput JSONB NOT NULL DEFAULT '{}',
    failure_rate JSONB NOT NULL DEFAULT '{}',
    blocks_completed BIGINT NOT NULL DEFAULT 0,
    anomaly_score DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE ml_node_profiles IS
    'Per-node performance profiles with anomaly scores for smart routing';

CREATE TABLE IF NOT EXISTS ml_node_form_affinity (
    node_id TEXT NOT NULL,
    form TEXT NOT NULL,
    alpha DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    beta DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    mean_throughput DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    n_obs BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (node_id, form)
);

COMMENT ON TABLE ml_node_form_affinity IS
    'Thompson Sampling affinity posteriors for (node, form) pairs';

-- ── Model Registry ─────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ml_model_registry (
    subsystem TEXT NOT NULL,
    version BIGINT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT false,
    is_shadow BOOLEAN NOT NULL DEFAULT false,
    predictions BIGINT NOT NULL DEFAULT 0,
    mape DOUBLE PRECISION,
    regret DOUBLE PRECISION,
    throughput_gain DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    promoted_at TIMESTAMPTZ,
    PRIMARY KEY (subsystem, version)
);

COMMENT ON TABLE ml_model_registry IS
    'Model versioning and shadow mode state for ML subsystems';
