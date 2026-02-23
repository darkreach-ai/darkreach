#!/usr/bin/env bash
set -euo pipefail

# Deploy darkreach to a remote server.
# Usage: ./deploy.sh <user@host> [--coordinator <url>]
#
# Installs Rust + GMP if missing, pulls latest code, builds with
# target-cpu=native for maximum GMP performance, and installs the binary.

if [ $# -lt 1 ]; then
    echo "Usage: $0 <user@host> [--coordinator <url>]"
    exit 1
fi

TARGET="$1"
COORDINATOR="${3:-}"
REPO_URL="$(git remote get-url origin 2>/dev/null || echo '')"
BRANCH="$(git branch --show-current)"

echo "==> Deploying darkreach to ${TARGET}"

ssh "${TARGET}" bash -s -- "${REPO_URL}" "${BRANCH}" <<'REMOTE_SCRIPT'
set -euo pipefail

REPO_URL="$1"
BRANCH="$2"
INSTALL_DIR="/opt/darkreach"
BIN_DIR="/usr/local/bin"

echo "--- Installing dependencies ---"
source "$HOME/.cargo/env" 2>/dev/null || true
if ! command -v cargo &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Install system deps (non-fatal — deps may already be present)
if [ -f /etc/debian_version ]; then
    sudo apt-get update -qq && sudo apt-get install -y -qq build-essential libgmp-dev m4 git pkg-config || echo "WARN: apt-get failed (deps may already be installed)"
elif [ -f /etc/redhat-release ]; then
    sudo yum install -y gcc gcc-c++ gmp-devel m4 git pkgconfig || echo "WARN: yum failed (deps may already be installed)"
fi

echo "--- Fetching code ---"
if [ -d "${INSTALL_DIR}" ]; then
    cd "${INSTALL_DIR}"
    git fetch origin
    git checkout "${BRANCH}"
    git reset --hard "origin/${BRANCH}"
else
    git clone "${REPO_URL}" "${INSTALL_DIR}"
    cd "${INSTALL_DIR}"
    git checkout "${BRANCH}"
fi

echo "--- Building (release, target-cpu=native) ---"
source "$HOME/.cargo/env" 2>/dev/null || true
RUSTFLAGS="-C target-cpu=native" cargo build --release

echo "--- Installing binary ---"
# Stop the service first to avoid "Text file busy" errors when overwriting
sudo systemctl stop darkreach-coordinator 2>/dev/null || true
sudo cp target/release/darkreach "${BIN_DIR}/darkreach"
sudo chmod +x "${BIN_DIR}/darkreach"

echo "--- Installing .env ---"
if [ -f .env ]; then
    cp .env "${INSTALL_DIR}/.env"
    chmod 600 "${INSTALL_DIR}/.env"
fi

echo "--- Installing systemd units ---"
sudo cp deploy/darkreach-coordinator.service /etc/systemd/system/ 2>/dev/null || true
sudo cp deploy/darkreach-worker@.service /etc/systemd/system/ 2>/dev/null || true
sudo systemctl daemon-reload

echo "--- Restarting services ---"
sudo systemctl start darkreach-coordinator 2>/dev/null || true

echo "--- Done! ---"
darkreach --help | head -1
REMOTE_SCRIPT

echo "==> Deploy to ${TARGET} complete."
