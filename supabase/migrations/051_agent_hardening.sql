-- Agent Hardening: timeout_secs, working_directory, expanded event types
--
-- Part of the Agent Daemon Hardening plan (Phase 4):
-- 1. Per-task timeout and working directory on agent_tasks
-- 2. Expand event_type CHECK to include token_update and progress events

-- 1. Per-task timeout (defaults to 30 minutes) and working directory
ALTER TABLE agent_tasks ADD COLUMN IF NOT EXISTS timeout_secs INTEGER NOT NULL DEFAULT 1800;
ALTER TABLE agent_tasks ADD COLUMN IF NOT EXISTS working_directory TEXT;

-- 2. Expand event_type constraint for real-time tracking events
ALTER TABLE agent_events DROP CONSTRAINT IF EXISTS agent_events_event_type_check;
ALTER TABLE agent_events ADD CONSTRAINT agent_events_event_type_check
  CHECK (event_type IN (
    'created','started','completed','failed','cancelled','message',
    'tool_call','tool_result','error','claimed','budget_exceeded',
    'parent_completed','parent_failed','diagnosis',
    'progress','token_update'
  ));
