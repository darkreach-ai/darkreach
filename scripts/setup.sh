#!/usr/bin/env bash
set -euo pipefail

# One-command developer setup for darkreach.
#
# Checks prerequisites, generates age key if missing,
# decrypts secrets, builds the project, and installs hooks.
#
# Usage: ./scripts/setup.sh

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ERRORS=0

echo "=== darkreach developer setup ==="
echo ""

# ── Step 1: Check prerequisites ──────────────────────────────
echo "==> [1/6] Checking prerequisites"

check_cmd() {
    local cmd="$1"
    local install_hint="$2"
    if command -v "$cmd" &>/dev/null; then
        echo "  [ok] $cmd ($(command -v "$cmd"))"
    else
        echo "  [MISSING] $cmd — install with: $install_hint"
        ERRORS=$((ERRORS + 1))
    fi
}

check_cmd cargo "https://rustup.rs"
check_cmd node "brew install node  (or nvm install 22)"
check_cmd npm "comes with node"
check_cmd age "brew install age"
check_cmd sops "brew install sops"
check_cmd yq "brew install yq"

# Check GMP
if pkg-config --exists gmp 2>/dev/null || [ -f /opt/homebrew/lib/libgmp.dylib ] || [ -f /usr/lib/libgmp.so ]; then
    echo "  [ok] GMP library"
else
    echo "  [MISSING] GMP — install with: brew install gmp (macOS) or apt install libgmp-dev (Linux)"
    ERRORS=$((ERRORS + 1))
fi

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo "Fix the $ERRORS missing prerequisite(s) above, then re-run this script."
    exit 1
fi

# ── Step 2: Generate age key if missing ──────────────────────
echo ""
echo "==> [2/6] Checking age key"

AGE_KEY_FILE="${HOME}/.config/sops/age/keys.txt"
if [ -f "$AGE_KEY_FILE" ]; then
    PUB_KEY=$(grep "public key:" "$AGE_KEY_FILE" | awk '{print $NF}')
    echo "  Age key exists. Public key: $PUB_KEY"
else
    echo "  Generating new age key..."
    mkdir -p "$(dirname "$AGE_KEY_FILE")"
    age-keygen -o "$AGE_KEY_FILE" 2>&1
    PUB_KEY=$(grep "public key:" "$AGE_KEY_FILE" | awk '{print $NF}')
    echo "  Generated. Public key: $PUB_KEY"
    echo ""
    echo "  IMPORTANT: Share this public key with the team so it can be added to .sops.yaml"
    echo "  Your private key is at $AGE_KEY_FILE — never share it."
fi

# ── Step 3: Decrypt secrets ──────────────────────────────────
echo ""
echo "==> [3/6] Decrypting secrets"

if [ -f "$ROOT/secrets/env.enc.yaml" ]; then
    # Only attempt if the file looks encrypted (has sops metadata)
    if grep -q "sops:" "$ROOT/secrets/env.enc.yaml" 2>/dev/null; then
        "$ROOT/scripts/decrypt-secrets.sh"
    else
        echo "  secrets/env.enc.yaml exists but is not yet encrypted."
        echo "  Fill in real values, then run: sops --encrypt --in-place secrets/env.enc.yaml"
        echo "  Skipping decryption for now."
        if [ ! -f "$ROOT/.env" ]; then
            cp "$ROOT/.env.example" "$ROOT/.env"
            echo "  Copied .env.example → .env (fill in real values)"
        fi
    fi
else
    echo "  No encrypted secrets found. Copying .env.example → .env"
    cp "$ROOT/.env.example" "$ROOT/.env"
fi

# ── Step 4: Build Rust project ───────────────────────────────
echo ""
echo "==> [4/6] Building Rust project"
cd "$ROOT"
cargo build 2>&1 | tail -1
echo "  Build complete"

# ── Step 5: Install frontend dependencies ────────────────────
echo ""
echo "==> [5/6] Installing frontend dependencies"
cd "$ROOT/frontend"
npm install --silent 2>&1 | tail -3
echo "  Frontend deps installed"

# ── Step 6: Install pre-commit hook ──────────────────────────
echo ""
echo "==> [6/6] Installing pre-commit hook"
cd "$ROOT"
ln -sf ../../scripts/pre-commit .git/hooks/pre-commit
echo "  Pre-commit hook installed"

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "=== Setup complete ==="
echo ""
echo "Next steps:"
echo "  1. If your age public key isn't in .sops.yaml yet, share it with the team"
echo "  2. Fill in real values in .env (or run ./scripts/decrypt-secrets.sh after key setup)"
echo "  3. Copy SSH config entries from deploy/ssh_config.example to ~/.ssh/config"
echo "  4. Run 'cargo test' to verify everything works"
echo ""
echo "Day-to-day workflow:"
echo "  git checkout -b feat/my-feature"
echo "  # ... make changes ..."
echo "  git push -u origin feat/my-feature"
echo "  gh pr create"
