Handle deployment and infrastructure tasks for darkreach.

## Context

Read `deploy/CLAUDE.md` for the full ops stack. Fleet architecture: coordinator + N worker instances.

## Infrastructure

| File | Purpose |
|------|---------|
| `deploy/deploy.sh` | Quick SSH deploy (install deps, build, systemd units) |
| `deploy/production-deploy.sh` | Full production setup (swap, UFW, kernel, nginx, systemd, frontend) |
| `deploy/worker-deploy.sh` | Worker-only deployment |
| `deploy/pgo-build.sh` | Profile-Guided Optimization build (5-15% speedup) |
| `deploy/nginx-darkreach.conf` | Nginx reverse proxy (rate limit, WebSocket, TLS, caching) |
| `deploy/darkreach-coordinator.service` | Coordinator systemd (port 7001, security-hardened) |
| `deploy/darkreach-worker@.service` | Worker template (instance %i, auto-restart 10s) |
| `deploy/grafana/darkreach.json` | Grafana dashboard definition |
| `deploy/helm/darkreach/` | Kubernetes Helm chart (coordinator + workers + KEDA + Prometheus) |
| `deploy/terraform/` | Infrastructure-as-Code (environments + modules) |
| `deploy/releases/worker-manifest.json` | Worker binary release manifest |

## Fleet Architecture

```
Coordinator (CX22, 178.156.211.107)
  ├── Dashboard on port 7001
  ├── Worker registry (60s stale timeout)
  ├── REST API + WebSocket
  └── PostgreSQL work distribution (Supabase)

Workers (CCX23 4-worker)
  ├── HTTP heartbeat (10s)
  ├── FOR UPDATE SKIP LOCKED block claiming
  └── Auto-restart on failure
```

Multiple workers per host: `systemctl start darkreach-worker@{1,2,3,4}`

## Build Optimization

- **Release**: `lto=fat`, `codegen-units=1`, `opt-level=3`
- **Apple Silicon**: `-Ctarget-cpu=apple-m1` (NOT native — Rust bug #93889)
- **PGO**: `./deploy/pgo-build.sh` (instrument → profile → optimize)
- **mimalloc**: Global allocator in `main.rs`

## Deployment Flow

```bash
# Quick deploy
./deploy/deploy.sh <host>

# Full production setup
./deploy/production-deploy.sh <host>

# Worker-only deploy
./deploy/worker-deploy.sh <host> <coordinator-url>

# Kubernetes
helm install darkreach deploy/helm/darkreach/ -f deploy/helm/darkreach/values-production.yaml
```

## Monitoring

- **Prometheus metrics**: `darkreach_*` prefix (via `src/prom_metrics.rs`)
- **Grafana dashboard**: import `deploy/grafana/darkreach.json`
- **System metrics**: `src/metrics.rs` (CPU, memory, disk)
- **Observability API**: `/api/observability/*` routes

## Server-Side Files

Related source files in `src/`:
- `deploy.rs` — SSH deployment, rolling updates, service management
- `dashboard/mod.rs` — Axum server setup, router, middleware
- `metrics.rs` + `prom_metrics.rs` — Metrics collection + Prometheus export
- `network.rs` — Node registry (60s stale timeout)
- `pg_worker.rs` — PostgreSQL work claiming
- `operator.rs` — Operator node management

## Roadmap

See `docs/roadmaps/ops.md` for planned improvements.
