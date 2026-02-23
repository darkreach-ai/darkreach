#!/usr/bin/env bash
# Apply all darkreach migrations to PostgreSQL.
#
# Designed for Docker init containers — idempotent, strips Supabase-specific
# DDL that doesn't exist in plain PostgreSQL (publications, RLS, policies).
#
# Usage: ./init-db-docker.sh [DATABASE_URL]
#
# Environment:
#   DATABASE_URL — PostgreSQL connection string (fallback if no arg given)
#
# Exit codes:
#   0 — all migrations applied (or already applied)
#   1 — DATABASE_URL not set

set -euo pipefail

DB_URL="${1:-${DATABASE_URL:-}}"

if [[ -z "$DB_URL" ]]; then
    echo "ERROR: DATABASE_URL not set. Pass as argument or environment variable." >&2
    exit 1
fi

MIGRATIONS_DIR="${MIGRATIONS_DIR:-/app/migrations}"

echo "Applying migrations from ${MIGRATIONS_DIR} ..."

count=0
for f in "${MIGRATIONS_DIR}"/*.sql; do
    [ -f "$f" ] || continue
    echo "  $(basename "$f")"
    # Strip Supabase-specific lines that fail on plain PostgreSQL
    sed \
        -e '/ALTER PUBLICATION/d' \
        -e '/ENABLE ROW LEVEL SECURITY/d' \
        -e '/^CREATE POLICY/d' \
        "$f" | psql "$DB_URL" -q -v ON_ERROR_STOP=0 2>&1 | grep -v "^$" || true
    count=$((count + 1))
done

echo "Applied ${count} migration files. Database ready."
