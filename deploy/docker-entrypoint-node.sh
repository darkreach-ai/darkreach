#!/bin/sh
# docker-entrypoint-node.sh — Entrypoint wrapper for darkreach operator nodes.
#
# Converts environment variables into a ~/.darkreach/config.toml if one does
# not already exist, then execs the darkreach binary with the provided arguments.
#
# Supported environment variables:
#   DARKREACH_SERVER       - Coordinator URL (e.g. https://api.darkreach.ai)
#   DARKREACH_API_KEY      - Operator API key from `darkreach register`
#   DARKREACH_USERNAME     - Operator username
#   DARKREACH_WORKER_ID    - Node identifier (defaults to hostname)
#   DATABASE_URL           - PostgreSQL connection string (required for most modes)
#   DARKREACH_THREADS      - Number of rayon worker threads (0 = all cores)
#   DARKREACH_LOG_FORMAT   - Log format: "json" for structured, default for human
#   DARKREACH_SEARCH_JOB_ID - Search job ID (convenience for `work` subcommand)
#
# Usage:
#   # Operator daemon mode (default CMD = "run"):
#   docker run -e DARKREACH_SERVER=https://api.darkreach.ai \
#              -e DARKREACH_API_KEY=dk_... \
#              -e DARKREACH_USERNAME=alice \
#              -e DATABASE_URL=postgres://... \
#              darkreach-node
#
#   # Direct work block mode:
#   docker run -e DATABASE_URL=postgres://... \
#              darkreach-node work --search-job-id 42
set -e

CONFIG_DIR="/root/.darkreach"
CONFIG_FILE="${CONFIG_DIR}/config.toml"

# Generate config.toml from env vars if it doesn't already exist
if [ ! -f "${CONFIG_FILE}" ] && [ -n "${DARKREACH_SERVER}" ] && [ -n "${DARKREACH_API_KEY}" ]; then
    mkdir -p "${CONFIG_DIR}"
    cat > "${CONFIG_FILE}" <<EOF
server = "${DARKREACH_SERVER}"
api_key = "${DARKREACH_API_KEY}"
username = "${DARKREACH_USERNAME:-operator}"
worker_id = "${DARKREACH_WORKER_ID:-$(hostname)}"
EOF
    chmod 600 "${CONFIG_FILE}"
    echo "Generated ${CONFIG_FILE} from environment variables"
fi

# Set LOG_FORMAT for structured logging in containers
if [ -n "${DARKREACH_LOG_FORMAT}" ]; then
    export LOG_FORMAT="${DARKREACH_LOG_FORMAT}"
fi

# Build CLI flags from env vars
EXTRA_ARGS=""
if [ -n "${DARKREACH_THREADS}" ]; then
    EXTRA_ARGS="${EXTRA_ARGS} --threads ${DARKREACH_THREADS}"
fi

# If first arg looks like a darkreach subcommand, pass everything through.
# If first arg is "run" or "work", inject extra args.
case "${1}" in
    run|work|dashboard|verify|verify-worker|register|factorial|kbn|palindromic|primorial|cullen-woodall|wagstaff|carol-kynea|twin|sophie-germain|repunit|gen-fermat|near-repdigit|project)
        # Known subcommand — exec darkreach with all args
        # shellcheck disable=SC2086
        exec darkreach ${EXTRA_ARGS} "$@"
        ;;
    -*)
        # Looks like a flag — pass everything to darkreach
        # shellcheck disable=SC2086
        exec darkreach ${EXTRA_ARGS} "$@"
        ;;
    *)
        # Unknown — pass through (could be a shell command like "sh")
        exec "$@"
        ;;
esac
