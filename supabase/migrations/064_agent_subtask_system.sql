-- Agent subtask system enhancements.
-- Adds retry_count for child task retry tracking and extends on_child_failure
-- to support 'continue' (ignore failures) and 'retry' (auto-retry up to 3 times).

-- Add retry_count to track how many times a child task has been retried.
ALTER TABLE agent_tasks ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;

-- Drop the existing CHECK constraint and replace with extended values.
ALTER TABLE agent_tasks DROP CONSTRAINT IF EXISTS agent_tasks_on_child_failure_check;
ALTER TABLE agent_tasks ADD CONSTRAINT agent_tasks_on_child_failure_check
  CHECK (on_child_failure IN ('fail', 'skip', 'continue', 'retry'));
