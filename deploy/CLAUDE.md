# deploy/ — Ops Domain

Deployment scripts, systemd units, Nginx config, Grafana dashboards, Helm charts, and Terraform infrastructure for the darkreach fleet.

> **Primary deploy command:** `scripts/deploy` (unified, with rollback, health checks, parallel nodes, Vercel co-deploy). The legacy scripts below (`deploy.sh`, `production-deploy.sh`, `worker-deploy.sh`) are deprecated.

## Directory Structure

```
deploy/
├── deploy.sh                      # DEPRECATED — use scripts/deploy
├── production-deploy.sh           # DEPRECATED — use scripts/deploy
├── worker-deploy.sh               # DEPRECATED — use scripts/deploy
├── pgo-build.sh                   # Profile-Guided Optimization build
├── nginx-darkreach.conf           # Nginx reverse proxy (rate limit, WebSocket, TLS)
├── darkreach-coordinator.service  # Systemd coordinator unit (port 7001, security-hardened)
├── darkreach-worker@.service      # Systemd worker template (auto-restart, instance %i)
├── grafana/
│   └── darkreach.json             # Grafana dashboard definition
├── helm/
│   └── darkreach/
│       ├── Chart.yaml             # Helm chart metadata
│       ├── values.yaml            # Default values
│       ├── values-production.yaml # Production overrides
│       └── templates/
│           ├── coordinator-deployment.yaml
│           ├── coordinator-service.yaml
│           ├── coordinator-ingress.yaml
│           ├── worker-deployment.yaml
│           ├── configmap.yaml
│           ├── secrets.yaml
│           ├── servicemonitor.yaml # Prometheus ServiceMonitor
│           └── keda-scaledobject.yaml # KEDA autoscaling
├── terraform/
│   ├── environments/              # Per-environment tfvars
│   └── modules/                   # Terraform modules
└── releases/
    └── worker-manifest.json       # Worker binary release manifest
```

## Files

| File | Purpose |
|------|---------|
| `deploy.sh` | DEPRECATED — use `scripts/deploy coordinator` |
| `production-deploy.sh` | DEPRECATED — use `scripts/deploy all` |
| `worker-deploy.sh` | DEPRECATED — use `scripts/deploy nodes` |
| `pgo-build.sh` | PGO build: instrument → profile run → optimized build (5-15% speedup) |
| `nginx-darkreach.conf` | Nginx: rate limiting, WebSocket upgrade, static caching, security headers, gzip |
| `darkreach-coordinator.service` | Coordinator: port 7001, strict filesystem, no new privs, 512M memory limit, 65k fds |
| `darkreach-worker@.service` | Node template: `%i` instance, `--coordinator` flag, auto-restart 10s |

## Deployment Flow

**Preferred:** `scripts/deploy` (unified command with rollback, health checks, parallel nodes)

```bash
scripts/deploy all              # Full deploy: coordinator + nodes + web
scripts/deploy coordinator      # Coordinator only (with health check + auto-rollback)
scripts/deploy nodes            # All worker nodes (parallel)
scripts/deploy web              # Frontend + website (parallel Vercel deploys)
scripts/deploy rollback         # Rollback coordinator to previous binary
scripts/deploy status           # Show health of all services
```

Connection details are loaded from `secrets/infra.enc.yaml` via `scripts/load-infra.sh`.

**Legacy flow (deprecated):**
```
deploy.sh → SSH to target → install deps (Rust, GMP)
  → clone/pull repo → cargo build --release
  → cp binary to /usr/local/bin/darkreach
  → systemctl enable/start coordinator + worker units
```

## Architecture Migration Note

> **Note:** The architecture is migrating from fleet/worker terminology to network/node terminology. Nodes now self-update via `darkreach run` instead of requiring SSH deployment. Old deployment scripts (`worker-deploy.sh`) remain for legacy setups but new nodes use the auto-update mechanism. See `docs/roadmaps/architecture.md`.

## Multi-Developer SSH Access

Both developers have full SSH access via the `deploy` user (not root).

```bash
# Copy entries from deploy/ssh_config.example to ~/.ssh/config
ssh darkreach-coordinator      # Coordinator (178.156.211.107)
ssh darkreach-worker-1         # Worker (178.156.158.184)

# DB access via SSH tunnel (no pg_hba.conf changes needed)
ssh -L 5432:localhost:5432 darkreach-coordinator
psql postgresql://postgres:...@localhost:5432/postgres
```

### Deploy coordination
- Announce deploys in chat before starting
- Only deploy from `master` branch
- `production-deploy.sh` uses `${DEPLOY_USER:-deploy}@` (override with `DEPLOY_USER=root` if needed)

### Server setup for new developer
On each server, add their SSH public key to `/home/deploy/.ssh/authorized_keys`.

## Fleet Architecture

```
Coordinator (1 instance, CX22)
  ├── Serves dashboard on port 7001
  ├── Manages worker registry (in-memory, 60s stale timeout)
  ├── REST API + WebSocket for frontend
  └── PostgreSQL-based work distribution (Supabase)

Nodes (N instances, CCX23 4-node)
  ├── Register via HTTP heartbeat (10s interval)
  ├── Claim work blocks via FOR UPDATE SKIP LOCKED
  ├── Run search subcommands
  ├── Self-update via `darkreach run` auto-updater
  └── Report primes to coordinator + database
```

Multiple workers on one host: `systemctl start darkreach-worker@1 darkreach-worker@2`

## Build Optimization

Release profile in `Cargo.toml`:
```toml
[profile.release]
lto = "fat"           # Whole-program link-time optimization
codegen-units = 1     # Single codegen unit for maximum optimization
opt-level = 3         # Maximum optimization
```

**Apple Silicon:** Use `-Ctarget-cpu=apple-m1`, NOT `-Ctarget-cpu=native` (Rust bug #93889).

**PGO build:** `./deploy/pgo-build.sh` (instrument → profile → optimize).

**mimalloc:** Global allocator configured in `main.rs`.

## Kubernetes (Helm)

```bash
# Deploy to Kubernetes
helm install darkreach deploy/helm/darkreach/ -f deploy/helm/darkreach/values-production.yaml

# Worker autoscaling via KEDA (based on work_blocks queue depth)
# Prometheus metrics via ServiceMonitor
```

## Monitoring

- **Grafana dashboard**: `deploy/grafana/darkreach.json` (import to Grafana)
- **Prometheus metrics**: `darkreach_*` prefix, scraped via ServiceMonitor
- **Metric types**: candidates tested, primes found, tests/sec, sieve efficiency, worker health

## Agent Coding Guide

### Adding a new deployment target

1. Create deployment script in `deploy/<target>-deploy.sh`
2. Create systemd unit if long-running: `deploy/darkreach-<service>.service`
3. Update Helm chart if Kubernetes: add template in `deploy/helm/darkreach/templates/`
4. Update Terraform if cloud infra: add module in `deploy/terraform/modules/`

### Adding monitoring

1. Add Prometheus metric in `src/prom_metrics.rs` (prefix: `darkreach_`)
2. Add panel to Grafana dashboard in `deploy/grafana/darkreach.json`
3. Add alert rule if critical

### Node release management

- Release manifest: `deploy/releases/worker-manifest.json`
- Nodes auto-update via release channel (see `src/db/releases.rs`)
- Dashboard UI at `/releases` page

## Website Deployment (Vercel)

The marketing website at `darkreach.ai` is deployed via Vercel from the `website/` directory.

- **Domain:** `darkreach.ai`
- **Framework:** Next.js (static export with MDX content)
- **Config:** `website/vercel.json`
- **Deploy:** Automatic on push to `master` (Vercel GitHub integration)

### deploy.sh Hardening

The `deploy.sh` script includes the following safety measures:
- Uses absolute `sudo` paths (`/usr/bin/sudo`) for security
- Guards `.env` copy (skips when already in install directory)
- Sources Cargo environment (`$HOME/.cargo/env`) before builds

## Roadmap

See `docs/roadmaps/ops.md` for planned improvements.
