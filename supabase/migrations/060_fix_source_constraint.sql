-- Fix agent_tasks source CHECK constraint to allow new source types.
--
-- The original constraint (migration 006) only allowed:
--   'manual', 'automated', 'agent'
--
-- New source types needed:
--   'ai_engine'  — tasks created by the AI engine OODA loop
--   'schedule'   — tasks fired by the agent scheduler daemon
--   'api'        — tasks created via external API calls

BEGIN;

ALTER TABLE agent_tasks DROP CONSTRAINT agent_tasks_source_check;

ALTER TABLE agent_tasks ADD CONSTRAINT agent_tasks_source_check
    CHECK (source IN ('manual', 'automated', 'agent', 'ai_engine', 'schedule', 'api'));

COMMIT;
