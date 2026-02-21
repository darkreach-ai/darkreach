#!/usr/bin/env bash
set -euo pipefail

# Decrypt SOPS-encrypted secrets to local .env files.
#
# Prerequisites: age, sops, yq
#   brew install age sops yq
#
# Your age private key must be at ~/.config/sops/age/keys.txt

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Check prerequisites
for cmd in sops yq; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: $cmd is not installed. Run: brew install $cmd"
        exit 1
    fi
done

if [ ! -f "${HOME}/.config/sops/age/keys.txt" ]; then
    echo "Error: No age key found at ~/.config/sops/age/keys.txt"
    echo "Generate one with: age-keygen -o ~/.config/sops/age/keys.txt"
    exit 1
fi

echo "Decrypting secrets..."

# Backend .env — contains DATABASE_URL, JWT secret, Redis URL
sops --decrypt "$ROOT/secrets/env.enc.yaml" | \
    yq -r 'to_entries | .[] | "\(.key)=\(.value)"' > "$ROOT/.env"
chmod 600 "$ROOT/.env"
echo "  Created .env ($(wc -l < "$ROOT/.env" | tr -d ' ') vars)"

# Frontend .env.local — public keys only (not secret, but convenient)
cat > "$ROOT/frontend/.env.local" <<'EOF'
NEXT_PUBLIC_SUPABASE_URL=https://nljvgyorzoxajodkkqdu.supabase.co
NEXT_PUBLIC_SUPABASE_ANON_KEY=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Im5sanZneW9yem94YWpvZGtrcWR1Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3Mzk2MTg5MTMsImV4cCI6MjA1NTE5NDkxM30.r5xRNcFNWbGdlarXOC3JOhEAmb9n0RLKF6lSPxzKN1c
NEXT_PUBLIC_API_URL=https://api.darkreach.ai
NEXT_PUBLIC_WS_URL=wss://api.darkreach.ai/ws
EOF
echo "  Created frontend/.env.local"

echo "Done. Secrets decrypted to .env and frontend/.env.local"
