# Infrastructure

> *Bare metal for compute, managed for data, observable everything.*

---

## Infrastructure Philosophy

1. **Bare metal for compute.** Primality testing is CPU-bound. Hypervisor overhead, noisy neighbors, and cloud pricing make VMs 3-5x worse per dollar than dedicated hardware. Hetzner bare metal gives us dedicated AMD cores at commodity prices.

2. **Managed for data.** We don't want to manage PostgreSQL backups, replication, failover, auth, or real-time subscriptions. Supabase handles all of this. If we outgrow Supabase, we self-host PostgreSQL — but not before then.

3. **Reproducible deploys.** Every deployment is scripted. `deploy.sh` for SSH, Helm for Kubernetes, Terraform for infrastructure. No manual configuration that isn't captured in code.

4. **Observable everything.** Prometheus metrics (`darkreach_*`), Grafana dashboards, structured logging. If something breaks, we should know before a user tells us.

5. **Secure by default.** SOPS-encrypted secrets, systemd security hardening (NoNewPrivileges, ProtectSystem), rate limiting, RLS on every table, UFW firewall rules.

---

## Architecture Overview

```
                              Internet
                                 │
                    ┌────────────┴────────────┐
                    │       Cloudflare         │
                    │  DNS + DDoS + SSL + CDN  │
                    └────────────┬────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                   │
              ▼                  ▼                   ▼
     darkreach.ai       app.darkreach.ai     api.darkreach.ai
     ┌──────────┐       ┌──────────────┐     ┌──────────────┐
     │  CDN /   │       │    Nginx     │     │    Nginx     │
     │  Static  │       │  (reverse    │     │  (reverse    │
     │  Export  │       │   proxy)     │     │   proxy)     │
     └──────────┘       └──────┬───────┘     └──────┬───────┘
                               │                     │
                               └──────┬──────────────┘
                                      │
                                      ▼
                               ┌──────────────┐
                               │    Axum      │
                               │  Dashboard   │
                               │  (port 7001) │
                               │              │
                               │ ┌──────────┐ │
                               │ │ Static   │ │     ┌──────────────┐
                               │ │ Frontend │ │     │  Prometheus  │
                               │ └──────────┘ │◀────│  (scrape)    │
                               │ ┌──────────┐ │     └──────┬───────┘
                               │ │ REST API │ │            │
                               │ └──────────┘ │     ┌──────▼───────┐
                               │ ┌──────────┐ │     │   Grafana    │
                               │ │WebSocket │ │     │  Dashboards  │
                               │ └──────────┘ │     └──────────────┘
                               │ ┌──────────┐ │
                               │ │AI Engine │ │
                               │ └──────────┘ │
                               └──────┬───────┘
                                      │
                                      ▼
                               ┌──────────────┐
                               │   Supabase   │
                               │  PostgreSQL  │
                               │              │
                               │  + Auth      │
                               │  + Realtime  │
                               │  + Storage   │
                               └──────┬───────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    │                 │                  │
                    ▼                 ▼                  ▼
              ┌──────────┐     ┌──────────┐      ┌──────────┐
              │  Node 1  │     │  Node 2  │      │  Node N  │
              │  (AX42)  │     │ (CCX23)  │      │ (AX162)  │
              │          │     │          │      │          │
              │ darkreach│     │ darkreach│      │ darkreach│
              │   run    │     │   run    │      │   run    │
              └──────────┘     └──────────┘      └──────────┘
                    │                 │                  │
                    └─────────────────┼──────────────────┘
                                      │
                              PG heartbeat (10s)
                              Block claiming (FOR UPDATE SKIP LOCKED)
                              Result reporting (INSERT)

                    ┌─────────────────────────────────────────┐
                    │       Coordinator (also runs):           │
                    │                                         │
                    │  ┌─────────────┐   ┌────────────────┐   │
                    │  │  OpenClaw   │   │ OpenClaw Kanban │   │
                    │  │  Gateway    │   │ (OCD)          │   │
                    │  │ (port 18789)│   │ (port 18790)   │   │
                    │  └──────┬──────┘   └────────────────┘   │
                    │         │                               │
                    │         ├── Discord (team chat)          │
                    │         ├── OpenRouter (multi-model LLM) │
                    │         └── Firecrawl (web scraping)     │
                    └─────────────────────────────────────────┘
```

---

## Current Fleet

### Coordinator

| Attribute | Value |
|-----------|-------|
| Server | Hetzner CX22 |
| IP | 178.156.211.107 |
| Specs | 2 vCPU, 4GB RAM, 40GB NVMe |
| OS | Ubuntu 22.04 |
| Role | Dashboard server, API, WebSocket, AI engine |
| Port | 7001 (behind Nginx) |
| Cost | ~$5/mo |

### Workers

| Server | IP | Specs | Role | Cost |
|--------|-----|-------|------|------|
| AX42 | (provisioned) | AMD Ryzen 7, 8 cores, 64GB RAM | Primary compute | $53/mo |
| CCX23 (x4) | 178.156.158.184 (and 3 more) | 4 AMD vCPU, 16GB RAM | Secondary compute | $12/mo each |

### Database

| Attribute | Value |
|-----------|-------|
| Provider | Supabase |
| Region | eu-central-1 (Frankfurt) |
| Plan | Free tier |
| Version | PostgreSQL 15 |
| Extensions | pgcrypto, pg_stat_statements |

---

## Deployment Pipeline

### Development

```
Developer machine
    │
    ├── cargo build --release          (engine + server)
    ├── cd frontend && npm run build   (dashboard)
    └── cargo test                     (1,235 tests)
```

### CI (GitHub Actions)

```
Push to PR branch
    │
    ├── cargo build         ─── compile check
    ├── cargo test          ─── unit + integration tests
    ├── cargo clippy        ─── lint
    ├── cargo fmt --check   ─── formatting
    ├── cargo audit         ─── security audit
    ├── npm test            ─── frontend unit tests
    └── npm run test:e2e    ─── Playwright E2E
```

### Release

```
Tag push (v0.x.y)
    │
    ├── Cross-compile (x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu)
    ├── Sign binaries (GPG)
    ├── Create GitHub Release with artifacts
    ├── Docker build (multi-arch: amd64 + arm64)
    └── Push to GHCR (ghcr.io/darkreach-ai/darkreach:v0.x.y)
```

### Production Deploy

```
SSH to coordinator
    │
    ├── git pull origin master
    ├── cargo build --release (with PGO if available)
    ├── cp target/release/darkreach /usr/local/bin/
    ├── systemctl restart darkreach-coordinator
    └── Verify health check (curl localhost:7001/api/health)

Nodes auto-update via:
    darkreach run --auto-update
```

---

## Component Configuration

### Nginx (`deploy/nginx-darkreach.conf`)

```
- Rate limiting: 10 req/s per IP (burst 20), 60 req/s for API
- WebSocket upgrade: /ws path with Connection: upgrade
- Static caching: 1 year for /assets/*, 1 hour for HTML
- Security headers: X-Frame-Options, X-Content-Type-Options, CSP
- Gzip: enabled for text/html, application/json, text/css, application/javascript
- SSL: Cloudflare origin certificate (or Let's Encrypt)
```

### systemd (`deploy/darkreach-coordinator.service`)

```
- ExecStart: /usr/local/bin/darkreach dashboard --port 7001
- Restart: always (RestartSec=5)
- MemoryMax: 512M
- NoNewPrivileges: true
- ProtectSystem: strict
- ReadWritePaths: /var/lib/darkreach
- LimitNOFILE: 65536
- Environment: DATABASE_URL loaded from /etc/darkreach/env
```

### systemd (`deploy/darkreach-worker@.service`)

```
- ExecStart: /usr/local/bin/darkreach run
- Restart: always (RestartSec=10)
- Instance: %i (allows multiple per host)
- Environment: DATABASE_URL loaded from /etc/darkreach/env
```

### PGO Build (`deploy/pgo-build.sh`)

```bash
# 1. Instrument
RUSTFLAGS="-Cprofile-generate=/tmp/pgo" cargo build --release

# 2. Profile (run representative workload)
target/release/darkreach kbn --k 3 --base 2 --min-n 1 --max-n 10000

# 3. Optimize
llvm-profdata merge -o /tmp/pgo/merged.profdata /tmp/pgo
RUSTFLAGS="-Cprofile-use=/tmp/pgo/merged.profdata" cargo build --release
```

Result: 5-15% speedup on primality testing.

---

## Monitoring & Observability

### Prometheus Metrics

23 metrics with `darkreach_` prefix. Scraped by Prometheus (or Grafana Agent) at 15s intervals.

**Key metrics:**

| Metric | Type | Labels |
|--------|------|--------|
| `darkreach_candidates_tested_total` | Counter | form |
| `darkreach_primes_found_total` | Counter | form |
| `darkreach_tests_per_second` | Gauge | form |
| `darkreach_sieve_efficiency` | Gauge | form |
| `darkreach_sieve_candidates_eliminated` | Counter | form |
| `darkreach_workers_online` | Gauge | — |
| `darkreach_workers_active` | Gauge | — |
| `darkreach_work_blocks_pending` | Gauge | job_id |
| `darkreach_work_blocks_completed_total` | Counter | job_id |
| `darkreach_work_blocks_reclaimed_total` | Counter | job_id |
| `darkreach_checkpoint_saves_total` | Counter | — |
| `darkreach_api_requests_total` | Counter | method, path, status |
| `darkreach_api_request_duration_seconds` | Histogram | method, path |
| `darkreach_websocket_connections` | Gauge | — |
| `darkreach_ai_engine_decisions_total` | Counter | action |
| `darkreach_ai_engine_tick_duration_seconds` | Histogram | — |

### Grafana Dashboard

Importable from `deploy/grafana/darkreach.json`. Panels:

- **Overview**: Primes found (counter), tests/sec (gauge), workers online (gauge)
- **Engine**: Per-form testing rate, sieve efficiency, candidate funnel
- **Network**: Worker count, heartbeat age, work block distribution
- **API**: Request rate, latency percentiles (p50, p95, p99), error rate
- **AI Engine**: Decision frequency, confidence distribution, scoring weights
- **System**: CPU, memory, disk (from node_exporter)

### Alerting Rules

| Alert | Condition | Severity |
|-------|-----------|----------|
| `DarkreachDown` | No metrics scraped for 5 min | Critical |
| `HighErrorRate` | API 5xx > 5% for 5 min | Warning |
| `NoWorkersOnline` | `darkreach_workers_online == 0` for 10 min | Warning |
| `StaleWorker` | Worker heartbeat > 5 min old | Info |
| `DiskFull` | Disk usage > 90% | Warning |
| `HighMemory` | Memory usage > 80% for 10 min | Warning |

---

## Environments

| Environment | Purpose | Database | Domain | Deploy |
|-------------|---------|----------|--------|--------|
| **Local** | Development | Local PostgreSQL or Supabase dev | `localhost:7001`, `localhost:3001` | `cargo run`, `npm run dev` |
| **Staging** | Pre-production testing | Supabase staging project | `staging.darkreach.ai` (planned) | Manual deploy to staging server |
| **Production** | Live platform | Supabase production project | `app.darkreach.ai`, `api.darkreach.ai` | Deploy from master, announce first |

### Local Development

```bash
# Terminal 1: Backend
DATABASE_URL=postgresql://... cargo run -- dashboard --port 7001

# Terminal 2: Frontend (dev server with HMR)
cd frontend && npm run dev    # Runs on port 3001

# Terminal 3: Run a search
DATABASE_URL=postgresql://... cargo run -- kbn --k 3 --base 2 --min-n 1 --max-n 1000
```

### Development with Caddy (HTTPS locally)

```bash
# Setup local TLS for *.darkreach.test domains
./scripts/dev-caddy-setup.sh
./scripts/dev-caddy.sh
```

---

## Disaster Recovery

### Database Backups

| Mechanism | Frequency | Retention | Recovery |
|-----------|-----------|-----------|----------|
| Supabase automatic | Daily | 7 days (free), 30 days (Pro) | Supabase dashboard restore |
| Manual pg_dump | Before major migrations | Indefinite (stored locally) | `pg_restore` |

### Coordinator Failover

The coordinator is currently a single point of failure for the dashboard and API. Nodes continue working if the coordinator goes down (they talk to PG directly).

**Recovery procedure:**
1. Provision new Hetzner server
2. Run `deploy/production-deploy.sh`
3. Update DNS A records for `app.darkreach.ai` and `api.darkreach.ai`
4. Verify health check
5. Estimated recovery time: ~30 minutes

### Node Recovery

Nodes are stateless. Recovery is automatic:
1. systemd auto-restarts on crash (RestartSec=10)
2. Uncompleted work blocks are reclaimed after stale timeout (120s default)
3. Checkpoints saved every 60s — max 60s of lost work per restart

---

## Cost Estimation

### Phase 1: Foundation ($53/mo)

```
1x AX42 (coordinator + compute)         $53/mo
Supabase (free tier)                      $0
Cloudflare (free tier)                    $0
GitHub (free)                             $0
                                    ──────────
Total                                   $53/mo
```

8 cores, 64GB RAM. Sufficient for engine optimization and single-form searches.

### Phase 2: Scale Up ($215-430/mo)

```
1x AX42 (coordinator)                   $53/mo
1-2x AX162-S (workers)              $86-172/mo
Supabase Pro (if needed)              $0-25/mo
                                    ──────────
Total                              $139-250/mo
```

24-40 cores. GWNUM unlocks 50-100x speedup — hardware investment pays off.

### Phase 3: Full Fleet ($430-760/mo)

```
1x CX22 (coordinator, downsize)          $5/mo
1x AX42 + 2-3x AX162-S (workers)  $225-310/mo
Supabase Pro                            $25/mo
Sentry (error tracking)              $0-29/mo
                                    ──────────
Total                              $255-369/mo
```

40-56 cores. Distributed campaigns across multiple workers.

### Phase 4: Public Compute ($760+/mo)

```
Coordinator (dedicated)                  $53/mo
3-5x workers                       $260-430/mo
Supabase Pro                            $25/mo
Sentry                                  $29/mo
Email service                       $10-20/mo
Analytics                            $0-29/mo
                                    ──────────
Total                              $377-586/mo
```

Plus volunteer-contributed compute (free additional capacity).

---

## Scaling Checklist

### Phase 1 Milestones (Current)
- [x] Single-server deployment with systemd
- [x] Nginx reverse proxy with rate limiting
- [x] PGO build script
- [x] Prometheus metrics export
- [x] Grafana dashboard
- [ ] Automated backup verification
- [ ] Staging environment

### Phase 2 Milestones
- [ ] Multi-server deployment (separate coordinator and workers)
- [ ] Helm chart deployment option
- [ ] Auto-update mechanism for worker binary
- [ ] Health check monitoring with alerts
- [ ] Database connection pooling (PgBouncer or Supabase built-in)

### Phase 3 Milestones
- [ ] Terraform for infrastructure provisioning
- [ ] Blue-green deployment for coordinator
- [ ] Database read replicas (if query load requires)
- [ ] Log aggregation (Loki or equivalent)
- [ ] Incident runbook documentation

### Phase 4 Milestones
- [ ] Kubernetes deployment option
- [ ] KEDA autoscaling based on work queue depth
- [ ] Multi-region coordinator (for latency-sensitive operator API)
- [ ] Database HA (hot standby or Supabase Pro multi-AZ)
- [ ] SOC 2 compliance assessment (if institutional customers)

---

## Docker Services (Coordinator)

OpenClaw runs alongside the darkreach dashboard on the coordinator in Docker containers.

| Container | Image | Port | Memory | Purpose |
|-----------|-------|------|--------|---------|
| `darkreach-openclaw` | `openclaw/openclaw:latest` | 18789 | 512 MB | Agent gateway, Discord bot, OpenRouter/Firecrawl |
| `darkreach-kanban` | `ghcr.io/keeeeeeeks/opencode-dashboard:latest` | 18790 | 256 MB | Kanban board (OCD) for task management |

**Compose file:** `deploy/docker-compose.openclaw.yml`

**Nginx routes:** `openclaw.darkreach.ai` → `:18789`, `kanban.darkreach.ai` → `:18790`

See [OpenClaw Concept](openclaw.md) for full deployment details.

---

## Related Documents

- [Concept](concept.md) — Vision and product definition
- [Platform](platform.md) — Repository map and open/closed split
- [Services](services.md) — External service details
- [Architecture](architecture.md) — Technical design
- [OpenClaw](openclaw.md) — AI agent orchestration (Discord, kanban, multi-model)
- [Architecture Roadmap](../roadmaps/architecture.md) — Migration plan
- [Ops Roadmap](../roadmaps/ops.md) — Planned operational improvements
- [OpenClaw Roadmap](../roadmaps/openclaw.md) — OpenClaw integration phases
