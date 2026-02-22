-- Migration: Per-operator IP allowlisting
--
-- Allows operators to restrict API key usage to specific IP addresses or CIDR
-- ranges. When allowed_ips is NULL or empty, all IPs are accepted (default).

ALTER TABLE operators ADD COLUMN IF NOT EXISTS allowed_ips TEXT[] DEFAULT '{}';
ALTER TABLE operators ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true;

-- Index for deactivation checks
CREATE INDEX IF NOT EXISTS idx_operators_is_active ON operators (is_active) WHERE NOT is_active;
