#!/usr/bin/env bash
# Decrypt infrastructure config from SOPS and export as shell variables.
#
# Usage: source scripts/load-infra.sh
#
# Exports:
#   DEPLOY_SSH_USER, DEPLOY_SSH_KEY_PATH
#   DEPLOY_COORDINATOR_HOST, DEPLOY_COORDINATOR_PORT, DEPLOY_COORDINATOR_NAME
#   DEPLOY_NODE_COUNT, DEPLOY_NODE_<N>_HOST, DEPLOY_NODE_<N>_PORT, DEPLOY_NODE_<N>_NAME
#   VERCEL_TOKEN, VERCEL_FRONTEND_PROJECT, VERCEL_WEBSITE_PROJECT

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INFRA_FILE="$ROOT/secrets/infra.enc.yaml"

if [ ! -f "$INFRA_FILE" ]; then
    echo "Error: $INFRA_FILE not found" >&2
    return 1 2>/dev/null || exit 1
fi

# Check for required tools
for cmd in sops yq; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: $cmd is required but not installed" >&2
        return 1 2>/dev/null || exit 1
    fi
done

# Decrypt once into a temp variable
_INFRA_YAML=$(sops --decrypt "$INFRA_FILE" 2>/dev/null)
if [ -z "$_INFRA_YAML" ]; then
    echo "Error: Failed to decrypt $INFRA_FILE (check your age key)" >&2
    return 1 2>/dev/null || exit 1
fi

# SSH config
export DEPLOY_SSH_USER=$(echo "$_INFRA_YAML" | yq -r '.ssh.user // "deploy"')
export DEPLOY_SSH_KEY_PATH=$(echo "$_INFRA_YAML" | yq -r '.ssh.key_path // "~/.ssh/darkreach_deploy"')

# Coordinator
export DEPLOY_COORDINATOR_HOST=$(echo "$_INFRA_YAML" | yq -r '.ssh.coordinator.host // ""')
export DEPLOY_COORDINATOR_PORT=$(echo "$_INFRA_YAML" | yq -r '.ssh.coordinator.port // "22"')
export DEPLOY_COORDINATOR_NAME=$(echo "$_INFRA_YAML" | yq -r '.ssh.coordinator.name // "coordinator"')

# Nodes (array)
NODE_COUNT=$(echo "$_INFRA_YAML" | yq -r '.ssh.nodes | length // 0')
export DEPLOY_NODE_COUNT="$NODE_COUNT"

for i in $(seq 0 $((NODE_COUNT - 1))); do
    idx=$((i + 1))
    export "DEPLOY_NODE_${idx}_HOST=$(echo "$_INFRA_YAML" | yq -r ".ssh.nodes[$i].host // \"\"")"
    export "DEPLOY_NODE_${idx}_PORT=$(echo "$_INFRA_YAML" | yq -r ".ssh.nodes[$i].port // \"22\"")"
    export "DEPLOY_NODE_${idx}_NAME=$(echo "$_INFRA_YAML" | yq -r ".ssh.nodes[$i].name // \"node-$idx\"")"
    export "DEPLOY_NODE_${idx}_WORKERS=$(echo "$_INFRA_YAML" | yq -r ".ssh.nodes[$i].workers // \"1\"")"
done

# Vercel
export VERCEL_TOKEN=$(echo "$_INFRA_YAML" | yq -r '.vercel.token // ""')
export VERCEL_FRONTEND_PROJECT=$(echo "$_INFRA_YAML" | yq -r '.vercel.frontend_project // ""')
export VERCEL_WEBSITE_PROJECT=$(echo "$_INFRA_YAML" | yq -r '.vercel.website_project // ""')

# Clean up
unset _INFRA_YAML

echo "Infrastructure config loaded from SOPS."
