-- Node Command Queue
--
-- Replaces the single `pending_command TEXT` column on the workers table with
-- a proper command queue supporting multiple commands, ACK lifecycle, retries,
-- and expiration. Commands are delivered FIFO during worker heartbeats.
--
-- Lifecycle: queued → delivered → acked  (happy path)
--            queued → delivered → expired (timeout + max retries exceeded)
--            queued → cancelled           (admin cancel)

CREATE TABLE node_commands (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    node_id         TEXT NOT NULL,
    command         TEXT NOT NULL CHECK (command IN ('stop','restart','update_config','reassign','upgrade')),
    params          JSONB,
    status          TEXT NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued','delivered','acked','expired','cancelled')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at    TIMESTAMPTZ,
    acked_at        TIMESTAMPTZ,
    expired_at      TIMESTAMPTZ,
    expires_after_s INTEGER NOT NULL DEFAULT 300,
    retry_count     INTEGER NOT NULL DEFAULT 0,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    error_message   TEXT,
    created_by      TEXT
);

-- Fast heartbeat fetch: find queued or re-deliverable commands for a node
CREATE INDEX idx_node_commands_pending
    ON node_commands (node_id, status)
    WHERE status IN ('queued', 'delivered');

-- Expiry sweep: find delivered commands past their timeout
CREATE INDEX idx_node_commands_expiry
    ON node_commands (status, delivered_at)
    WHERE status = 'delivered';

-- Dashboard listing: recent commands ordered by creation time
CREATE INDEX idx_node_commands_created
    ON node_commands (created_at DESC);

-- ── fetch_node_commands ─────────────────────────────────────────
-- Called during worker heartbeat. Atomically marks queued commands as
-- delivered, re-delivers stale ones (under max_retries), and returns
-- them in FIFO order for processing.
CREATE OR REPLACE FUNCTION fetch_node_commands(p_node_id TEXT)
RETURNS TABLE(command_id BIGINT, command TEXT, params JSONB) AS $$
BEGIN
    -- Mark fresh queued commands as delivered
    UPDATE node_commands nc
    SET status = 'delivered',
        delivered_at = NOW()
    WHERE nc.node_id = p_node_id
      AND nc.status = 'queued';

    -- Re-deliver stale commands that haven't exceeded max_retries.
    -- A command is stale if delivered more than expires_after_s ago
    -- without being acked.
    UPDATE node_commands nc
    SET status = 'delivered',
        delivered_at = NOW(),
        retry_count = nc.retry_count + 1
    WHERE nc.node_id = p_node_id
      AND nc.status = 'delivered'
      AND nc.delivered_at < NOW() - (nc.expires_after_s || ' seconds')::interval
      AND nc.retry_count < nc.max_retries;

    -- Return all currently delivered commands in FIFO order
    RETURN QUERY
    SELECT nc.id AS command_id, nc.command, nc.params
    FROM node_commands nc
    WHERE nc.node_id = p_node_id
      AND nc.status = 'delivered'
    ORDER BY nc.id ASC;
END;
$$ LANGUAGE plpgsql;

-- ── ack_node_command ────────────────────────────────────────────
-- Called by the worker after processing a command. If error_message
-- is provided, it's stored but the command is still marked acked
-- (the worker tried its best).
CREATE OR REPLACE FUNCTION ack_node_command(p_command_id BIGINT, p_error TEXT DEFAULT NULL)
RETURNS BOOLEAN AS $$
DECLARE
    v_updated BOOLEAN;
BEGIN
    UPDATE node_commands
    SET status = 'acked',
        acked_at = NOW(),
        error_message = p_error
    WHERE id = p_command_id
      AND status = 'delivered';

    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- ── expire_stale_commands ───────────────────────────────────────
-- Background sweep: expires delivered commands that have been stale
-- for longer than their timeout AND exceeded max_retries.
CREATE OR REPLACE FUNCTION expire_stale_commands()
RETURNS INTEGER AS $$
DECLARE
    v_count INTEGER;
BEGIN
    UPDATE node_commands
    SET status = 'expired',
        expired_at = NOW()
    WHERE status = 'delivered'
      AND delivered_at < NOW() - (expires_after_s || ' seconds')::interval
      AND retry_count >= max_retries;

    GET DIAGNOSTICS v_count = ROW_COUNT;
    RETURN v_count;
END;
$$ LANGUAGE plpgsql;
