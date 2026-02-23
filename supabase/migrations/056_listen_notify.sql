-- PG LISTEN/NOTIFY triggers for event-driven WebSocket updates.
--
-- Replaces 2s polling with instant push: trigger functions fire pg_notify()
-- on key table mutations, the dashboard's PgListener receives them, and the
-- WebSocket bridge pushes to clients within milliseconds.
--
-- Four channels:
--   prime_discovered  — INSERT on primes
--   block_status      — UPDATE on work_blocks (status change)
--   job_status        — UPDATE on search_jobs (status change)
--   node_command      — INSERT on node_commands

-- ── Channel: prime_discovered ────────────────────────────────────────

CREATE OR REPLACE FUNCTION notify_prime_discovered() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('prime_discovered', json_build_object(
        'id', NEW.id,
        'form', NEW.form,
        'expression', NEW.expression,
        'digits', NEW.digits
    )::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER prime_discovered_notify
    AFTER INSERT ON primes
    FOR EACH ROW EXECUTE FUNCTION notify_prime_discovered();

-- ── Channel: block_status ────────────────────────────────────────────

CREATE OR REPLACE FUNCTION notify_block_status() RETURNS TRIGGER AS $$
BEGIN
    IF OLD.status IS DISTINCT FROM NEW.status THEN
        PERFORM pg_notify('block_status', json_build_object(
            'id', NEW.id,
            'job_id', NEW.search_job_id,
            'status', NEW.status,
            'worker_id', NEW.claimed_by
        )::text);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER block_status_notify
    AFTER UPDATE ON work_blocks
    FOR EACH ROW EXECUTE FUNCTION notify_block_status();

-- ── Channel: job_status ──────────────────────────────────────────────

CREATE OR REPLACE FUNCTION notify_job_status() RETURNS TRIGGER AS $$
BEGIN
    IF OLD.status IS DISTINCT FROM NEW.status THEN
        PERFORM pg_notify('job_status', json_build_object(
            'id', NEW.id,
            'status', NEW.status,
            'search_type', NEW.search_type
        )::text);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER job_status_notify
    AFTER UPDATE ON search_jobs
    FOR EACH ROW EXECUTE FUNCTION notify_job_status();

-- ── Channel: node_command ────────────────────────────────────────────

CREATE OR REPLACE FUNCTION notify_node_command() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('node_command', json_build_object(
        'id', NEW.id,
        'node_id', NEW.node_id,
        'command', NEW.command
    )::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER node_command_notify
    AFTER INSERT ON node_commands
    FOR EACH ROW EXECUTE FUNCTION notify_node_command();
