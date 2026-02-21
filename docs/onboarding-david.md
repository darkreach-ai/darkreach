# Darkreach — Setup Guide for David Elvar (Windows)

Welcome to darkreach! This guide gets you from zero to a fully working dev environment with SSH access to production servers, encrypted secrets, and the full build/test/deploy workflow.

Since you're on Windows, we'll use **WSL2** (Windows Subsystem for Linux). The project is Rust + GMP + bash scripts — WSL2 gives you a native Linux environment where everything just works.

---

## Step 1: Install WSL2

Open PowerShell as Administrator:

```powershell
wsl --install -d Ubuntu-24.04
```

Restart your PC when prompted. After restart, Ubuntu will finish setup and ask you to create a username/password.

> **Tip:** Install [Windows Terminal](https://aka.ms/terminal) from the Microsoft Store for a much better terminal experience. It integrates with WSL2 out of the box.

From here on, **run everything inside the WSL2 Ubuntu terminal** unless stated otherwise.

---

## Step 2: Install system dependencies

```bash
sudo apt update && sudo apt install -y \
  build-essential libgmp-dev m4 pkg-config libssl-dev \
  git curl unzip
```

---

## Step 3: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
```

Verify:

```bash
rustc --version   # should print 1.8x+
cargo --version
```

---

## Step 4: Install Node.js 22

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
```

Verify:

```bash
node --version   # v22.x
npm --version
```

---

## Step 5: Install secrets tooling (age, sops, yq)

```bash
# age
sudo apt install -y age

# sops (download binary — not in apt)
SOPS_VERSION=3.9.4
curl -Lo /tmp/sops "https://github.com/getsops/sops/releases/download/v${SOPS_VERSION}/sops-v${SOPS_VERSION}.linux.amd64"
sudo install /tmp/sops /usr/local/bin/sops

# yq
YQ_VERSION=4.44.6
curl -Lo /tmp/yq "https://github.com/mikefarah/yq/releases/download/v${YQ_VERSION}/yq_linux_amd64"
sudo install /tmp/yq /usr/local/bin/yq
```

Verify:

```bash
age --version
sops --version
yq --version
```

---

## Step 6: Generate SSH key

```bash
ssh-keygen -t ed25519 -C "david@darkreach" -f ~/.ssh/id_ed25519
```

Press Enter for no passphrase (or set one — your call).

Print your public key:

```bash
cat ~/.ssh/id_ed25519.pub
```

**Send this to Oddur.** He needs to:
1. Add it to your GitHub account (or you do that yourself at https://github.com/settings/keys)
2. Add it to the servers (`/home/deploy/.ssh/authorized_keys` on coordinator + workers)

---

## Step 7: Configure Git

```bash
git config --global user.name "David Elvar"
git config --global user.email "YOUR_EMAIL@example.com"
git config --global core.autocrlf false
git config --global init.defaultBranch master
```

---

## Step 8: Clone the repo

```bash
cd ~
git clone git@github.com:darkreach-ai/darkreach.git
cd darkreach
```

---

## Step 9: Generate age key (for secrets decryption)

```bash
mkdir -p ~/.config/sops/age
age-keygen -o ~/.config/sops/age/keys.txt
```

This prints your public key like:

```
Public key: age1abc123...
```

**Send this public key to Oddur.** He'll add it to `.sops.yaml` and re-encrypt the secrets so you can decrypt them too. Until then, use the example env:

```bash
cp .env.example .env
```

Once your key is added to `.sops.yaml` (Oddur will push a commit):

```bash
git pull
./scripts/decrypt-secrets.sh
```

This generates `.env` and `frontend/.env.local` with real credentials.

---

## Step 10: Build the project

```bash
# Rust backend (debug build — faster to compile)
cargo build

# Frontend
cd frontend && npm install && cd ..
```

For an optimized release build:

```bash
cargo build --release
```

---

## Step 11: Run tests

```bash
# Unit tests (should see 449+ passing)
cargo test

# Frontend tests
cd frontend && npm test && cd ..
```

---

## Step 12: Install pre-commit hook

```bash
ln -sf ../../scripts/pre-commit .git/hooks/pre-commit
```

This runs `cargo fmt --check`, `cargo test`, and TypeScript checks before each commit.

---

## Step 13: Set up SSH access to servers

Add these to `~/.ssh/config`:

```
Host darkreach-coordinator
    HostName 178.156.211.107
    User deploy
    IdentityFile ~/.ssh/id_ed25519

Host darkreach-worker-1
    HostName 178.156.158.184
    User deploy
    IdentityFile ~/.ssh/id_ed25519
```

Once Oddur adds your SSH key to the servers, verify:

```bash
ssh darkreach-coordinator whoami    # should print: deploy
ssh darkreach-worker-1 whoami      # should print: deploy
```

### Database access (via SSH tunnel)

```bash
# In one terminal, open the tunnel:
ssh -L 5432:localhost:5432 darkreach-coordinator

# In another terminal, connect:
psql "$DATABASE_URL"
```

---

## Step 14: Install GitHub CLI (optional but recommended)

```bash
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
sudo apt update && sudo apt install -y gh
gh auth login
```

---

## Day-to-Day Workflow

### Making changes

```bash
# Create feature branch
git checkout -b feat/my-feature

# ... make changes ...

# Format, test, commit
cargo fmt
cargo test
git add <files>
git commit -m "Add feature X"

# Push and create PR
git push -u origin feat/my-feature
gh pr create
```

### Branch naming

```
feat/<description>    — New features
fix/<description>     — Bug fixes
chore/<description>   — Maintenance, deps
docs/<description>    — Documentation
deploy/<description>  — Infrastructure changes
```

### PR rules

- All PRs target `master`
- CI must pass (fmt, clippy, test, frontend build)
- Requires 1 approval
- Squash-merge by default
- Trivial changes (typos, deps): self-approve after 30 min with `trivial` label
- Engine/DB/deploy changes: always need the other dev's review

### Deploying

- Only deploy from `master`
- Announce in chat before deploying
- Coordinator: `./deploy/production-deploy.sh`
- Workers: `./deploy/worker-deploy.sh deploy@<host> http://178.156.211.107 --workers 4`

---

## Editing secrets

To update a secret value:

```bash
# Opens your $EDITOR with decrypted YAML, re-encrypts on save
sops secrets/env.enc.yaml

# Or for infra secrets
sops secrets/infra.enc.yaml
```

After editing, commit and push the encrypted file:

```bash
git add secrets/env.enc.yaml
git commit -m "Rotate DATABASE_URL"
git push
```

---

## Project structure at a glance

```
src/                    Rust engine + server
├── 12 search forms     factorial, palindromic, kbn, twin, ...
├── dashboard/          Axum web server (REST API + WebSocket)
├── db/                 PostgreSQL layer
└── project/            Campaign management

frontend/               Next.js dashboard (React + Tailwind + shadcn/ui)
deploy/                 Systemd units, Nginx, Helm, Terraform
supabase/               Database migrations
secrets/                SOPS-encrypted credentials
scripts/                Dev scripts (setup, decrypt, pre-commit)
docs/                   Research + roadmaps
```

Each domain has its own `CLAUDE.md` with detailed docs. Start with the root `CLAUDE.md` for the full architecture overview.

---

## Useful commands

```bash
# Run a quick prime search to verify the engine works
cargo run -- factorial --start 1 --end 100
cargo run -- kbn --k 3 --base 2 --min-n 1 --max-n 1000

# Start local dev stack (backend + frontend)
./scripts/dev.sh

# Check server status
ssh darkreach-coordinator systemctl status darkreach-coordinator
curl https://api.darkreach.ai/api/status

# View coordinator logs
ssh darkreach-coordinator journalctl -u darkreach-coordinator -f

# View all worker logs
ssh darkreach-worker-1 journalctl -u 'darkreach-worker@*' -f
```

---

## Troubleshooting

**`cargo build` fails with "libgmp not found"**
```bash
sudo apt install libgmp-dev
```

**`scripts/decrypt-secrets.sh` fails with "no master key"**
Your age public key hasn't been added to `.sops.yaml` yet. Ask Oddur to add it and re-encrypt.

**SSH to servers times out**
Your SSH public key hasn't been added to the server yet. Ask Oddur to run:
```bash
ssh darkreach-coordinator "echo 'YOUR_PUB_KEY' >> /home/deploy/.ssh/authorized_keys"
```

**Pre-commit hook fails**
Run `cargo fmt` and `cargo test` manually, fix any errors, then commit again.

**VS Code can't find Rust in WSL2**
Install the "WSL" extension in VS Code, then open the project with `code .` from inside WSL2. Install "rust-analyzer" extension in the WSL2 remote.
