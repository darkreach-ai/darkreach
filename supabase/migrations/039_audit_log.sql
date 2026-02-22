-- Audit log for admin actions.
--
-- Records who performed what action on which resource, with timestamp
-- and optional payload snapshot. Used for security auditing and
-- compliance tracking.

CREATE TABLE IF NOT EXISTS audit_log (
    id          BIGSERIAL PRIMARY KEY,
    user_id     TEXT NOT NULL,
    user_email  TEXT,
    action      TEXT NOT NULL,           -- e.g. "search.create", "project.activate", "release.rollback"
    resource    TEXT,                     -- e.g. "/api/searches/42"
    method      TEXT NOT NULL,            -- HTTP method: POST, PUT, DELETE
    status_code INT,                      -- response status code
    ip_address  TEXT,
    user_agent  TEXT,
    payload     JSONB,                    -- request body snapshot (sensitive fields redacted)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_user_id ON audit_log (user_id);
CREATE INDEX idx_audit_log_action ON audit_log (action);
CREATE INDEX idx_audit_log_created_at ON audit_log (created_at DESC);
