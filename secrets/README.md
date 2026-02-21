# Encrypted Secrets

This directory contains SOPS-encrypted secrets using [age](https://github.com/FiloSottile/age) encryption.

## Files

| File | Contents |
|------|----------|
| `env.enc.yaml` | Runtime env vars (DATABASE_URL, JWT secret, Redis) |
| `infra.enc.yaml` | Infrastructure secrets (Hetzner API token, signing keys) |

## Usage

```bash
# Decrypt all secrets to local .env files
./scripts/decrypt-secrets.sh

# Edit a secret file (decrypts in-place, re-encrypts on save)
sops secrets/env.enc.yaml

# Encrypt after manual edit
sops --encrypt --in-place secrets/env.enc.yaml
```

## Setup

Each developer needs an age key pair. See `scripts/setup.sh` for automated setup, or:

```bash
brew install age sops yq
mkdir -p ~/.config/sops/age
age-keygen -o ~/.config/sops/age/keys.txt
```

Share your **public key** (`age1...`) with the team to be added to `.sops.yaml`.
