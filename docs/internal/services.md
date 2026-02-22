# Services

> *Every external dependency, why we chose it, and what would replace it.*

---

## Service Philosophy

1. **Bare metal for compute, managed for everything else.** Prime hunting is CPU-bound. Cloud VMs add latency, cost, and overhead. Bare metal servers (Hetzner) give dedicated cores at 3-5x better price/performance. For everything else (database, auth, DNS), use managed services to reduce operational burden.

2. **PostgreSQL as the coordination bus.** One database for everything: prime records, search jobs, work blocks, node heartbeats, AI decisions, agent tasks, projects. No message queues, no Redis, no separate coordination layer. PostgreSQL's `FOR UPDATE SKIP LOCKED` provides work claiming. Supabase Realtime provides live updates.

3. **No vendor lock-in on critical path.** The engine runs on any machine with GMP installed. Nodes need only a `DATABASE_URL`. If Supabase disappears, we spin up a PostgreSQL instance. If Hetzner raises prices, we move servers. The only hard dependency is PostgreSQL.

4. **Encrypt secrets at rest.** SOPS + age for in-repo secret management. No plaintext credentials in git, no external secret managers. Each developer has an age key pair; CI uses a separate key.

5. **Minimize service count.** Every service is a potential failure point, a billing line item, and an integration to maintain. Add services only when the alternative (building it ourselves) would be worse.

---

## Quick Reference

| Category | Service | Purpose | Status |
|----------|---------|---------|--------|
| Compute | Hetzner (bare metal) | Coordinator + worker nodes | Active |
| Database | Supabase (PostgreSQL) | Prime records, coordination, all state | Active |
| Auth | Supabase Auth | Dashboard login, operator registration | Active |
| DNS | Cloudflare | DNS, DDoS protection, SSL, caching | Active |
| CI/CD | GitHub Actions | Build, test, Docker, release, deploy | Active |
| Container Registry | GHCR (GitHub Container Registry) | Docker images | Active |
| Source Control | GitHub | Code hosting, PRs, issues | Active |
| Monitoring | Grafana Cloud (free tier) | Dashboards, alerting | Active |
| Metrics | Prometheus | Metric collection (`darkreach_*`) | Active |
| Secrets | SOPS + age | Encrypted secrets in-repo | Active |
| AI Orchestration | OpenClaw | Agent gateway, Discord, kanban | Planned |
| LLM Routing | OpenRouter | Multi-model API (200+ models) | Planned |
| Web Scraping | Firecrawl | t5k.org, OEIS, competitor monitoring | Planned |
| Team Chat | Discord | Agent dispatch, notifications, approvals | Planned |
| Error Tracking | (TBD) | Runtime error capture | Planned |
| Analytics | (TBD) | Usage analytics | Planned |
| Email | (TBD) | Discovery notifications, operator comms | Planned |
| Payments | (TBD) | If/when monetized | Not planned |

---

## Compute: Hetzner

**Why Hetzner:** Best price/performance for dedicated CPU cores in Europe. No hypervisor overhead. AMD Ryzen/EPYC processors with high single-core performance (critical for primality testing).

| Server | Role | Specs | Cost |
|--------|------|-------|------|
| CX22 | Coordinator | 2 vCPU, 4GB RAM, 40GB NVMe | ~$5/mo |
| CCX23 (x4) | Worker nodes | 4 AMD vCPU, 16GB RAM, 80GB NVMe | ~$12/mo each |
| AX42 | Worker (Phase 1) | AMD Ryzen 7, 8 cores, 64GB RAM, 2x512GB NVMe | ~$53/mo |
| AX162-S | Worker (Phase 2) | AMD EPYC, 16 cores, 128GB RAM | ~$86/mo |

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| OVH | Similar pricing, more DCs | Less reliable network | Change IP addresses, redeploy |
| AWS EC2 | Global, managed services | 3-5x more expensive for compute | Not viable for sustained compute |
| DigitalOcean | Simple, good DX | Shared CPUs, worse price/perf | Change IP addresses, redeploy |
| Fly.io | Edge deployment, auto-scale | No bare metal, premium pricing | Different deploy mechanism |

---

## Database: Supabase (PostgreSQL)

**Why Supabase:** Managed PostgreSQL with built-in auth, Realtime (WebSocket subscriptions), and a generous free tier. Eliminates the need to manage database backups, replicas, and auth infrastructure.

**Used for:**
- Prime record storage and querying
- Search job lifecycle and work block coordination
- Node heartbeats and fleet state
- AI engine decisions and scoring weights
- Agent tasks, budgets, memory, schedules
- Project campaigns and cost tracking
- World record tracking
- Operator accounts and trust levels
- Observability metrics and logs

**Current plan:** Free tier (500MB, 50K monthly active users, 2 concurrent connections per project)

**Schema:** 40+ migrations in `supabase/migrations/`. Core tables: `primes`, `search_jobs`, `work_blocks`, `workers`, `agent_tasks`, `projects`, `operators`, `ai_engine_state`, `ai_engine_decisions`.

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| Self-hosted PostgreSQL | Full control, no limits | Must manage backups, HA, auth | Change `DATABASE_URL`, run migrations |
| Neon | Serverless, branching | Newer, less proven at scale | Change `DATABASE_URL` |
| CockroachDB | Distributed, HA by default | Overkill, PostgreSQL compat gaps | Schema migration needed |
| PlanetScale (MySQL) | Branching, serverless | Not PostgreSQL (sqlx rewrite needed) | Not viable |

---

## Auth: Supabase Auth

**Why Supabase Auth:** Integrated with the database. JWT tokens, email/password, OAuth providers. No separate auth service to manage.

**Used for:**
- Dashboard login (email/password)
- Operator registration (planned: GitHub OAuth)
- JWT verification in API middleware
- Row-Level Security (RLS) policies

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| Auth0 | Feature-rich, enterprise | Expensive, external dependency | Change JWT verification |
| Clerk | Modern DX, React components | Expensive at scale | Change auth hooks |
| Self-hosted (argon2 + JWT) | Full control | Must build password reset, OAuth, MFA | Significant engineering |

---

## DNS & CDN: Cloudflare

**Why Cloudflare:** Free tier covers DNS, DDoS protection, SSL termination, and basic caching. The free tier is genuinely sufficient for a project of this scale.

**Used for:**
- DNS management for `darkreach.ai`, `app.darkreach.ai`, `api.darkreach.ai`
- SSL certificates (automatic via Cloudflare)
- DDoS protection (free tier)
- Static asset caching (website)

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| AWS Route 53 | Reliable, SLA | Costs money for DNS | Change nameservers |
| Namecheap DNS | Included with domain | No DDoS, no CDN | Change nameservers |

---

## CI/CD: GitHub Actions

**Why GitHub Actions:** Integrated with GitHub (source control). Free tier covers ~2,000 minutes/month. YAML-based, well-documented.

**Current workflows:**

| Workflow | Trigger | What it does |
|----------|---------|-------------|
| `ci.yml` | Push, PR | cargo build, cargo test, clippy, fmt |
| `release.yml` | Tag push | Cross-compile (x86_64 + aarch64), sign, GitHub Release |
| `docker.yml` | Tag push | Docker build, push to GHCR |
| `frontend.yml` | Push (frontend/) | npm install, npm test, npm run build |
| `e2e.yml` | Push (frontend/) | Playwright E2E tests |
| `security.yml` | PR | cargo audit, dependency review |

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| GitLab CI | Self-hostable, built-in registry | Different platform | Rewrite YAML |
| CircleCI | Fast, good caching | Paid for private repos | Rewrite YAML |
| Buildkite | Self-hosted agents, fast | Operational overhead | Rewrite YAML |

---

## Container Registry: GHCR

**Why GHCR:** Free for public repos, integrated with GitHub Actions. Docker images pushed automatically on release.

**Images:**
- `ghcr.io/darkreach-ai/darkreach:latest` — Full binary (coordinator + node)
- `ghcr.io/darkreach-ai/darkreach:v0.x.y` — Versioned releases

---

## Monitoring: Grafana + Prometheus

**Why Grafana + Prometheus:** Industry standard for metrics. Prometheus scrapes `darkreach_*` metrics from the API server. Grafana visualizes them. Both have generous free tiers.

**Metrics exported (23 total):**

| Metric | Type | Description |
|--------|------|-------------|
| `darkreach_candidates_tested_total` | Counter | Total candidates tested (per form) |
| `darkreach_primes_found_total` | Counter | Primes discovered (per form) |
| `darkreach_tests_per_second` | Gauge | Current testing throughput |
| `darkreach_sieve_efficiency` | Gauge | Fraction eliminated by sieve |
| `darkreach_workers_online` | Gauge | Connected workers |
| `darkreach_work_blocks_pending` | Gauge | Unclaimed work blocks |
| `darkreach_work_blocks_completed` | Counter | Completed work blocks |

**Dashboard:** `deploy/grafana/darkreach.json` — importable Grafana dashboard definition.

---

## Secrets: SOPS + age

**Why SOPS + age:** Encrypted secrets stored in git. No external secret manager. Each developer has an age key pair; public keys listed in `.sops.yaml`. CI uses a separate `SOPS_AGE_KEY` GitHub org secret.

**Encrypted files:** `secrets/*.enc.yaml`

**Workflow:**
```bash
# Decrypt to local .env (never committed)
./scripts/decrypt-secrets.sh

# Encrypt a new secret
sops --encrypt --age <public-key> secrets/new-secret.yaml > secrets/new-secret.enc.yaml
```

---

## Planned Services

### AI Orchestration: OpenClaw

**Why OpenClaw:** Open-source autonomous AI agent platform (145K+ GitHub stars). Gateway architecture that owns messaging surfaces (Discord), spawns agent sessions, and supports OpenRouter + Firecrawl out of the box. Adds team-facing orchestration alongside the existing AI engine and agent system.

**Used for:**
- Discord-based task dispatch and agent status notifications
- Multi-model LLM routing via OpenRouter (cost optimization)
- Web research via Firecrawl (t5k.org, OEIS, competitor monitoring)
- Kanban board (OCD) for visual task management
- 4 agent roles: Researcher, Developer, Operator, Strategist

**Deployment:** Docker Compose on Hetzner coordinator (port 18789 for gateway, 18790 for kanban).

**Budget:** ~$33/day across all agent roles ($5 Researcher + $10 Developer + $3 Operator + $15 Strategist), with provider-level hard cap at $100/month.

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| Custom orchestration | Full control, no dependencies | Months of development, reinventing the wheel | N/A |
| LangGraph | Good Python ecosystem | Python-only, no Discord/kanban built-in | Rewrite skills |
| CrewAI | Multi-agent framework | Less mature, smaller community | Rewrite config |

See [OpenClaw Concept](openclaw.md) for full architecture and [OpenClaw Roadmap](../roadmaps/openclaw.md) for implementation plan.

### LLM Routing: OpenRouter

**Why OpenRouter:** Single API for 200+ LLM models from Anthropic, OpenAI, Google, Meta, etc. Cost-based routing lets us use cheap models (Haiku) for monitoring and expensive models (Opus) for strategy. Usage tracking and spending limits built in.

**Used for:** All OpenClaw agent LLM calls (research, code, monitoring, analysis).

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| Direct Anthropic API | Lower latency, no middleman | Single provider, no cost routing | Change API base URL |
| LiteLLM | Self-hostable proxy | Operational overhead | Swap proxy URL |

### Web Scraping: Firecrawl

**Why Firecrawl:** API-based web scraping without running a browser. Handles JavaScript-rendered pages, anti-scraping, and rate limiting. First-class OpenClaw integration.

**Used for:** Researcher agent scraping t5k.org (world records), OEIS (sequence data), mersenneforum (competitor activity).

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| Playwright/Puppeteer | Full browser control | Heavy resource usage, complex setup | Change scraping code |
| Crawlee | Open-source, self-hosted | More operational overhead | Change scraping code |

### Team Chat: Discord

**Why Discord:** Free, real-time messaging with bot API, slash commands, reactions (for approvals), threads, and channel organization. The team already uses Discord-style tools. OpenClaw has first-class Discord support.

**Used for:**
- Agent task dispatch (`#agent-commands`)
- Agent activity feed (`#agent-feed`)
- Human approval workflow (`#approvals`)
- Prime discovery announcements (`#discoveries`)
- Fleet status and alerts (`#fleet-status`)
- Research reports (`#research`)

**Alternatives considered:**

| Alternative | Pros | Cons | Migration path |
|------------|------|------|----------------|
| Slack | Enterprise features, integrations | Paid for history, OpenClaw support is newer | Change bot token + config |
| Telegram | Simple, lightweight | Less structured channels | Change bot config |

### Error Tracking (TBD)

**Leading candidate:** Sentry

**Why needed:** Runtime errors in the API server and node binary need centralized tracking. Currently errors go to stderr/logs only.

**Requirements:** Rust SDK, source map upload for frontend, low overhead.

### Analytics (TBD)

**Leading candidate:** PostHog (self-hosted or cloud)

**Why needed:** Understanding how operators use the dashboard, which pages are visited, where users drop off in onboarding.

**Requirements:** Privacy-respecting (no selling data), GDPR compliant, self-hostable option.

### Email (TBD)

**Leading candidate:** Resend or Postmark

**Why needed:** Discovery notifications ("Your node found a prime!"), operator onboarding emails, password reset.

**Requirements:** Transactional email API, reliable delivery, low volume pricing.

### Payments (TBD — not actively planned)

**Leading candidate:** Stripe

**Why needed:** Only if/when monetization is pursued. See [business.md](business.md) for options.

---

## Environment Variables

| Variable | Service | Where Used |
|----------|---------|-----------|
| `DATABASE_URL` | Supabase | API server, node binary |
| `NEXT_PUBLIC_SUPABASE_URL` | Supabase | Frontend |
| `NEXT_PUBLIC_SUPABASE_ANON_KEY` | Supabase | Frontend |
| `SUPABASE_SERVICE_ROLE_KEY` | Supabase | API server (admin operations) |
| `NEXT_PUBLIC_API_URL` | API server | Frontend (`https://api.darkreach.ai`) |
| `NEXT_PUBLIC_WS_URL` | API server | Frontend (`wss://api.darkreach.ai/ws`) |
| `OPENROUTER_API_KEY` | OpenRouter | OpenClaw gateway |
| `FIRECRAWL_API_KEY` | Firecrawl | OpenClaw gateway |
| `DISCORD_BOT_TOKEN` | Discord | OpenClaw gateway |
| `DISCORD_WEBHOOK_URL` | Discord | darkreach notifications |
| `SOPS_AGE_KEY` | SOPS | CI (secret decryption) |
| `GHCR_TOKEN` | GHCR | CI (Docker push) |

---

## Cost Summary

### Current (~$53/mo)

| Service | Cost |
|---------|------|
| Hetzner AX42 (coordinator + compute) | $53/mo |
| Supabase (free tier) | $0 |
| Cloudflare (free tier) | $0 |
| GitHub (free for public repo) | $0 |
| Grafana Cloud (free tier) | $0 |
| **Total** | **~$53/mo** |

### Phase 2 (~$215/mo)

| Service | Cost |
|---------|------|
| Hetzner AX42 | $53/mo |
| Hetzner AX162-S (x1-2 workers) | $86-172/mo |
| Supabase (free or Pro $25/mo) | $0-25/mo |
| Other services | $0 |
| **Total** | **~$139-250/mo** |

### Phase 3 (~$430/mo)

| Service | Cost |
|---------|------|
| Hetzner (3-4 servers) | $300-400/mo |
| Supabase Pro | $25/mo |
| Error tracking (Sentry) | $0-29/mo |
| **Total** | **~$325-454/mo** |

### Phase 4 (~$760/mo)

| Service | Cost |
|---------|------|
| Hetzner (5+ servers) | $500-600/mo |
| Supabase Pro | $25/mo |
| Error tracking | $29/mo |
| Email service | $10-20/mo |
| Analytics | $0-29/mo |
| **Total** | **~$564-703/mo** |

---

## Related Documents

- [Concept](concept.md) — Vision and product definition
- [Platform](platform.md) — Repository map and open/closed split
- [Infrastructure](infrastructure.md) — Hosting, deployment, monitoring details
- [Architecture](architecture.md) — Technical design
