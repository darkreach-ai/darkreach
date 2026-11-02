# supabase/ — Database Domain

PostgreSQL database schema managed via Supabase migrations. All tables live in self-hosted PostgreSQL and are accessed from Rust (via sqlx) and the frontend (via REST API).

## Migration Files

Migrations are numbered sequentially in `supabase/migrations/`. Apply in order.

| # | File | Tables/Functions | Purpose |
|---|------|-----------------|---------|
| 001 | `create_primes.sql` | `primes` | Core primes table (form, expression, digits, proof_method) + Realtime |
| 002 | `create_functions.sql` | RPCs | `get_stats`, `get_discovery_timeline`, `get_digit_distribution` |
| 003 | `rls_policies.sql` | — | Row-Level Security for primes |
| 004 | `coordination_tables.sql` | `workers`, `search_jobs`, `work_blocks` | Fleet coordination + RPCs (`worker_heartbeat`, `claim_work_block`, `reclaim_stale_blocks`) |
| 005 | `verification.sql` | `primes` (alter) | Add verification columns |
| 006 | `agents.sql` | `agent_tasks`, `agent_events`, `agent_budgets` | AI agent task queue + budgets + Realtime |
| 007 | `agent_cost_control.sql` | `agent_budgets` (alter) | Budget limits and tracking |
| 008 | `agent_permissions.sql` | `agent_permissions` | Agent permission controls |
| 009 | `agent_memory.sql` | `agent_memory` | Agent key-value memory store |
| 010 | `task_decomposition.sql` | `agent_tasks` (alter) | Subtask hierarchy (parent_task_id) |
| 011 | `projects.sql` | `projects`, `project_phases`, `records`, `project_events` | Campaign management + world records |
| 0121 | `project_cost_tracking.sql` | `projects` (alter) | Core-hours and cost columns |
| 012 | `form_leaderboard.sql` | RPCs | `form_leaderboard` function |
| 013 | `agent_roles.sql` | `agent_roles`, `agent_templates` | Agent role configuration + templates |
| 014 | `agent_schedules.sql` | `agent_schedules` | Scheduled agent task automation |
| 015 | `add_certificate.sql` | `primes` (alter) | Add certificate column |
| 016 | `lifecycle_management.sql` | `search_jobs` (alter), RPCs | Job lifecycle functions |
| 017 | `cost_calibration.sql` | `cost_calibrations` | Cost model coefficients per form |
| 018 | `agent_observability.sql` | `agent_tasks` (alter) | Agent observability columns |
| 019 | `volunteers.sql` | `volunteer_workers`, RPCs | Volunteer worker registration |
| 020 | `observability.sql` | `system_metrics`, `system_logs`, RPCs | System metrics + log storage |
| 021 | `volunteer_worker_capabilities.sql` | `volunteer_workers` (alter) | Worker capability columns |
| 022 | `worker_release_channels.sql` | `worker_releases`, `worker_release_channels` | Release channel management |
| 023 | `volunteer_worker_release_tracking.sql` | `volunteer_workers` (alter) | Release version tracking |
| 024 | `metric_rollups_daily.sql` | `metric_rollups_daily`, RPCs | Daily metric aggregation |
| 025 | `operator_rename.sql` | Renames | Rename volunteer tables → operator tables + backward-compat views |
| 026 | `user_profiles.sql` | `user_profiles` | User profile management |
| 027 | `strategy_engine.sql` | Strategy tables | Strategy engine for search optimization |
| 028 | `network_scaling.sql` | Network tables | Network scaling infrastructure |
| 029 | `security_hardening.sql` | — | RLS on 22 tables, SECURITY INVOKER views, function search_path, tighten write policies |
| 030 | `materialized_views.sql` | Materialized views | Pre-computed views for dashboard queries |
| 031 | `timescaledb_hypertables.sql` | Hypertables | TimescaleDB time-series optimization |
| 032 | `ai_engine.sql` | `ai_engine_state`, `ai_engine_decisions` | AI engine OODA loop state + decision audit trail |
| 033 | `ai_engine_phase6.sql` | AI engine (alter) | AI engine phase 6 enhancements |
| 033 | `prime_tags.sql` | `prime_tags` | Tagging system for primes |
| 034 | `prime_verification_queue.sql` | `prime_verification_queue` | Queued verification pipeline |
| 035 | `resource_pooling.sql` | `resource_pools`, `resource_allocations` | Compute resource pooling |
| 036 | `shared_sieves.sql` | `shared_sieves` | Cross-search sieve sharing |
| 037 | `api_key_hashing.sql` | — | Hash API keys (bcrypt), remove plaintext storage |
| 038 | `network_relay.sql` | Network tables (alter) | Network relay infrastructure |
| 039 | `audit_log.sql` | `audit_log` | Comprehensive audit logging |
| 040 | `api_key_expiration.sql` | — | API key expiration dates |
| 041 | `operator_ip_allowlist.sql` | `operator_ip_allowlist` | IP allowlisting per operator |
| 042 | `drop_plaintext_api_key.sql` | — | Drop plaintext API key column |
| 043 | `resource_contributions.sql` | `resource_contributions` | Track operator resource contributions |
| 044 | `node_commands.sql` | `node_commands` | Remote command queue for nodes |
| 045 | `pipeline_stages.sql` | `pipeline_stages` | Multi-stage work block pipeline |
| 046 | `operator_preferences.sql` | `operator_preferences` | Operator configuration preferences |
| 047 | `operator_badges.sql` | `operator_badges` | Achievement badges for operators |
| 048 | `ai_engine_phases.sql` | AI engine (alter) | AI engine phases 1-3 |
| 049 | `naming_migration.sql` | Renames | volunteer->operator, fleet->network naming migration |
| 050 | `ai_engine_phases_4_6.sql` | AI engine (alter) | AI engine phases 4-6 |
| 051 | `agent_hardening.sql` | Agent tables (alter) | Agent security hardening |
| 052 | `log_improvements.sql` | `system_logs` (alter) | Log level and metadata improvements |
| 053 | `browser_result_hash.sql` | `work_blocks` (alter) | Browser-submitted result hashing |
| 054 | `hash_verification.sql` | — | Hash verification functions |
| 055 | `adaptive_verification.sql` | — | Adaptive verification thresholds |
| 055 | `browser_block_ttl.sql` | `work_blocks` (alter) | Browser block TTL settings |
| 056 | `listen_notify.sql` | — | PostgreSQL LISTEN/NOTIFY channels |
| 057 | `waitlist.sql` | `waitlist` | Operator waitlist registration |
| 058 | `batch_claiming.sql` | — | Batch work block claiming |
| 059 | `ml_engine.sql` | `ml_models`, `ml_experiments`, `ml_predictions` | ML engine model registry + experiments |
| 060 | `fix_source_constraint.sql` | — | Fix agent_tasks source constraint |
| 061 | `strategy_advisor_role.sql` | `agent_roles` (alter) | Add strategy-advisor role |

## Schema Overview

### Core Tables

```
primes
├── id (BIGINT PK)
├── form (TEXT) — factorial, palindromic, kbn, twin, etc.
├── expression (TEXT) — e.g., "3!+1", "k*2^n+1"
├── digits (BIGINT)
├── found_at (TIMESTAMPTZ)
├── proof_method (TEXT) — deterministic, probabilistic
├── search_params (TEXT) — JSON search configuration
├── certificate (TEXT) — primality certificate
└── UNIQUE(form, expression)

workers
├── worker_id (TEXT PK) — hostname
├── search_type, search_params, tested, found, current
├── checkpoint, metrics (JSONB)
├── last_heartbeat (TIMESTAMPTZ)
└── pending_command (TEXT) — cleared on next heartbeat

search_jobs
├── id (BIGSERIAL PK)
├── search_type, params (JSONB), status
├── range_start, range_end, block_size
├── total_tested, total_found
└── project_id (FK → projects)

work_blocks
├── id (BIGSERIAL PK)
├── search_job_id (FK → search_jobs)
├── block_start, block_end, status
├── claimed_by (FK → workers)
└── UNIQUE(search_job_id, block_start)
```

### Agent Tables

```
agent_tasks — Task queue (status, priority, cost, parent hierarchy)
agent_events — Activity feed (event_type, summary, detail JSON)
agent_budgets — Spending limits (daily/weekly/monthly)
agent_memory — Key-value store per agent
agent_roles — Role configuration
agent_templates — Task templates
agent_schedules — Scheduled automation
agent_permissions — Permission controls
```

### Project Tables

```
projects — Campaign definitions (objective, form, status, budget, records)
project_phases — Ordered phases with completion conditions
project_events — Audit log of project changes
records — World records per form (from t5k.org)
```

### Observability Tables

```
system_metrics — Time-series metrics (CPU, memory, throughput)
system_logs — Structured log entries
metric_rollups_daily — Aggregated daily metrics
volunteer_workers — Volunteer registration and capabilities
  Note: Tables renamed in migration 025: volunteers → operators, volunteer_workers → operator_nodes,
  volunteer_trust → operator_trust, credit_log → operator_credits. Old names available as views.
worker_releases — Binary release metadata
worker_release_channels — Release channel config (stable, beta, nightly)
cost_calibrations — Per-form cost model coefficients
```

### Security & API Tables

```
audit_log — Comprehensive audit trail (action, actor, target, detail JSONB)
operator_ip_allowlist — IP allowlisting per operator
```

### AI Engine Tables

```
ai_engine_state — OODA loop state, scoring weights, snapshots
ai_engine_decisions — Decision audit trail (reasoning, confidence, outcome)
```

### ML Engine Tables

```
ml_models — Model registry (version, parameters, metrics, shadow mode flag)
ml_experiments — A/B experiment tracking
ml_predictions — Model prediction log with outcomes
```

### Operator & Badge Tables

```
operator_badges — Achievement badges for operators
operator_preferences — Operator configuration preferences
node_commands — Remote command queue for nodes
resource_contributions — Track operator resource contributions
resource_pools — Compute resource pooling
resource_allocations — Resource allocation tracking
waitlist — Operator waitlist registration
```

### Pipeline & Verification Tables

```
prime_verification_queue — Queued verification pipeline
pipeline_stages — Multi-stage work block pipeline
shared_sieves — Cross-search sieve sharing
prime_tags — Tagging system for primes
```

## Key RPC Functions

| Function | Purpose |
|----------|---------|
| `worker_heartbeat(...)` | Atomic UPSERT worker + read/clear pending command |
| `claim_work_block(job_id, worker_id)` | `FOR UPDATE SKIP LOCKED` block claiming |
| `reclaim_stale_blocks(stale_seconds)` | Reclaim blocks from dead workers |
| `get_stats()` | Dashboard summary stats |
| `get_discovery_timeline()` | Discoveries over time |
| `get_digit_distribution()` | Digit count histogram |
| `form_leaderboard()` | Form ranking by count/max digits |

## Conventions

- **Naming**: snake_case for tables, columns, and functions
- **IDs**: `BIGINT GENERATED ALWAYS AS IDENTITY` (not UUID)
- **Timestamps**: `TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- **Status columns**: CHECK constraints with explicit allowed values
- **JSONB**: Used for flexible schema (params, metrics, strategy, detail)
- **RLS**: All tables have Row-Level Security enabled. Read policies are permissive; write policies require auth
- **Realtime**: `primes`, `agent_tasks`, `agent_events` published for live notifications
- **Indexes**: On status columns, timestamps, and foreign keys used in queries

## Rust ↔ Database Mapping

| Supabase table | Rust db module | Dashboard route module |
|----------------|---------------|----------------------|
| `primes`, `prime_tags` | `db/primes.rs` | `routes_primes`, `routes_status`, `routes_verify` |
| `workers` | `db/workers.rs` | `routes_fleet` |
| `search_jobs`, `work_blocks` | `db/jobs.rs` | `routes_jobs`, `routes_searches` |
| `agent_*` | `db/agents.rs`, `db/memory.rs`, `db/roles.rs`, `db/schedules.rs` | `routes_agents` |
| `projects`, `project_*` | `db/projects.rs` | `routes_projects` |
| `records` | `db/records.rs` | `routes_projects` |
| `system_*`, `metric_*` | `db/observability.rs` | `routes_observability` |
| `operators`, `operator_*` | `db/operators.rs` | `routes_operator`, `routes_contribute` |
| `operator_badges` | `db/badges.rs` | `routes_operator` |
| `node_commands` | `db/commands.rs` | `routes_operator` |
| `worker_release*` | `db/releases.rs` | `routes_releases` |
| `cost_calibrations` | `db/calibrations.rs` | `routes_projects` |
| `ai_engine_*` | `db/ai_engine.rs` | `routes_strategy` |
| `audit_log` | `db/audit.rs` | `routes_audit` |
| `resource_*` | `db/resources.rs` | `routes_resources` |
| `shared_sieves` | `db/sieves.rs` | `routes_sieve` |
| `strategy_*` | `db/strategy.rs` | `routes_strategy` |
| `*_trust` | `db/trust.rs` | `routes_operator` |
| `user_profiles` | `db/user_profiles.rs` | `routes_auth` |
| `prime_verification_queue` | `db/prime_verification.rs` | `routes_prime_verification` |
| `ml_models`, `ml_experiments`, `ml_predictions` | `db/ai_engine.rs` | `routes_strategy` |
| `waitlist` | `db/operators.rs` | `routes_contribute` |
| `*_schedules` | `db/schedules.rs` | `routes_schedules` |

## Infrastructure Evolution

**Current state:** Self-hosted PostgreSQL 16 (CPX31, Hetzner Ashburn). Previously hosted on Supabase. All migrations are portable standard PostgreSQL — no Supabase-specific extensions or syntax.

**Database roadmap:** See [docs/roadmaps/database.md](../docs/roadmaps/database.md) for the full database infrastructure roadmap. Phases 1-6 are complete:
1. ~~**Optimize Supabase** — configurable connection pool, materialized views, time-based partitioning~~
2. ~~**Redis for hot-path data** — move worker heartbeats out of PostgreSQL~~
3. ~~**Self-hosted PostgreSQL + TimescaleDB** — full control over config, hypertables for time-series~~
4. ~~**Read replicas** — separate dashboard reads from write path~~
5. ~~**High availability** — automatic failover, PgBouncer~~
6. ~~**Frontend independence** — migrate frontend from Supabase JS to REST API~~

**Compatibility notes:**
- All migrations use standard SQL and will apply to any PostgreSQL 15+ instance
- TimescaleDB hypertables (Phase 3) require the `timescaledb` extension — these would be added as new migrations, not modifications to existing ones
- The `supabase_realtime` publication (used for live notifications) will be replaced by WebSocket push from the Axum server in Phase 6
- RLS policies are standard PostgreSQL and work on any self-hosted instance

---

## Agent Coding Guide

### Adding a new table

1. Create `supabase/migrations/NNN_<description>.sql` (next sequence number)
2. Follow conventions: snake_case, BIGINT identity PK, TIMESTAMPTZ, CHECK constraints
3. Enable RLS: `ALTER TABLE <name> ENABLE ROW LEVEL SECURITY;`
4. Add read policy: `CREATE POLICY "read_<name>" ON <name> FOR SELECT USING (true);`
5. Add write policy for authenticated users if needed
6. Add to Realtime if live updates needed: `ALTER PUBLICATION supabase_realtime ADD TABLE <name>;`
7. Create Rust types + queries in `src/db/<module>.rs`
8. Add API routes in `src/dashboard/routes_<module>.rs`

### Adding an RPC function

1. Add to the appropriate migration file (or create new one)
2. Use `LANGUAGE plpgsql` for complex logic, `LANGUAGE sql` for simple queries
3. Call from Rust via `sqlx::query!("SELECT * FROM <function>(...)")` in `src/db/`
4. Call from frontend via `supabase.rpc("<function>", { ... })` in hooks

### Migration numbering

- Sequential: 001, 002, ..., 061 (64 files total)
- Exceptions: `0121` exists between 012 and 013 (legacy conflict); duplicate prefix `033` and `055` (two files each)
- Always use the next available number (currently 062)
