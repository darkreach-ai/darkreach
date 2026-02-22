# OpenClaw Integration Roadmap

Team-facing AI orchestration: Discord dispatch, kanban tracking, multi-model routing, web research agents.

**Key files:** `deploy/docker-compose.openclaw.yml`, `~/.openclaw/characters/`, `~/openclaw/workspace/skills/`

**Concept doc:** [docs/internal/openclaw.md](../internal/openclaw.md)

---

## Current State

- **AI Engine**: Fully operational OODA loop managing search campaigns autonomously
- **Agent System**: `src/agent.rs` (1,387 lines) — Claude Code CLI subprocess spawning with budgets, permissions, task hierarchies
- **Dashboard**: REST API with 15 route modules, WebSocket for live updates
- **No team orchestration layer**: No Discord integration, no kanban board, no multi-model routing, no automated web research

---

## Phase 0: Infrastructure Setup

> Deploy OpenClaw + kanban on the Hetzner coordinator. Wire up Discord bot and secrets.

### 0.1 Discord Server & Bot

**Current:** No Discord presence.

**Target:** Discord server with 6 channels and a bot connected to OpenClaw.

**Steps:**
1. Create Discord server with channels: `#agent-feed`, `#discoveries`, `#fleet-status`, `#agent-commands`, `#approvals`, `#research`
2. Create bot in Discord Developer Portal with permissions: Send Messages, Embed Links, Read Message History, Use Slash Commands, Manage Threads
3. Invite bot to server
4. Record bot token

**Verification:** Bot appears online in Discord server.

### 0.2 Secrets Configuration

**Current:** SOPS-encrypted secrets for DATABASE_URL, Supabase keys, etc.

**Target:** Add OpenClaw secrets (OpenRouter, Firecrawl, Discord) to SOPS.

**Steps:**
1. Create `secrets/openclaw.enc.yaml` with 4 new keys
2. Update `scripts/decrypt-secrets.sh` to generate `.env.openclaw`
3. Verify decryption works locally

**Key files:** `secrets/openclaw.enc.yaml`, `scripts/decrypt-secrets.sh`, `.sops.yaml`

**Verification:** `./scripts/decrypt-secrets.sh` produces `.env.openclaw` with all 4 keys.

### 0.3 Docker Compose Deployment

**Current:** darkreach coordinator runs as a systemd service (bare metal).

**Target:** OpenClaw + kanban running alongside in Docker containers.

**Steps:**
1. Create `deploy/docker-compose.openclaw.yml`
2. Install Docker on coordinator (if not present)
3. Copy character files and skill stubs to coordinator
4. `docker compose up -d`
5. Add Nginx reverse proxy rules for `openclaw.darkreach.ai` and `kanban.darkreach.ai`
6. Update Cloudflare DNS for new subdomains

**Key files:** `deploy/docker-compose.openclaw.yml`, `deploy/nginx-darkreach.conf`

**Verification:**
- `curl https://openclaw.darkreach.ai` returns OpenClaw web UI
- `curl https://kanban.darkreach.ai` returns kanban board
- OpenClaw gateway logs show "Connected to Discord"

---

## Phase 1: Researcher Agent

> First agent role. Validates the skill system, Firecrawl integration, and Discord posting.

### 1.1 Research Skill

**Current:** No OpenClaw skills.

**Target:** `darkreach-research` skill that scrapes t5k.org, OEIS, and mersenneforum via Firecrawl.

**Implementation:**
```javascript
// ~/openclaw/workspace/skills/darkreach-research.js
// - scrape_t5k_top(n): Fetch top N primes from t5k.org
// - scrape_oeis(sequence_id): Fetch OEIS sequence data
// - check_competitors(): Monitor mersenneforum, PrimeGrid, GIMPS for new records
// - compare_records(form): Compare our records vs world records
```

**Key files:** `~/openclaw/workspace/skills/darkreach-research.js`

**Verification:** Skill returns structured data from t5k.org and OEIS when invoked manually.

### 1.2 Database Query Skill

**Current:** No OpenClaw skills.

**Target:** `darkreach-db` skill that queries prime records, search stats, and cost data via the REST API.

**Implementation:**
```javascript
// ~/openclaw/workspace/skills/darkreach-db.js
// - get_primes(form, limit): GET /api/primes?form=...&limit=...
// - get_records(): GET /api/records
// - get_observability(): GET /api/observability/metrics
// - get_projects(): GET /api/projects
```

**Key files:** `~/openclaw/workspace/skills/darkreach-db.js`

**Verification:** Skill returns prime records and stats from the darkreach API.

### 1.3 Researcher Character

**Current:** No OpenClaw characters.

**Target:** `darkreach-researcher` character with research + DB skills, Sonnet model, $5/day budget, every-6-hours schedule.

**Key files:** `~/.openclaw/characters/darkreach-researcher.json`

**Verification:**
- Agent runs on schedule (every 6 hours)
- Agent posts t5k.org world record updates to `#research`
- Agent posts competitor analysis reports to `#research`
- Daily spend stays under $5

### 1.4 Discovery Event Forwarding

**Current:** Prime discoveries are logged to PostgreSQL and emitted on the event bus.

**Target:** Researcher agent listens for discovery events via WebSocket and posts to `#discoveries`.

**Implementation:** Subscribe to `wss://api.darkreach.ai/ws` for `prime_found` events. Format and post to Discord.

**Verification:** When a new prime is found, announcement appears in `#discoveries` within 30 seconds.

---

## Phase 2: Developer Agent

> Second role. Validates code skills, PR workflow, and kanban integration.

### 2.1 Search Management Skill

**Current:** Research and DB skills from Phase 1.

**Target:** `darkreach-search` skill for creating and managing search campaigns.

**Implementation:**
```javascript
// ~/openclaw/workspace/skills/darkreach-search.js
// - create_search(form, params): POST /api/searches
// - list_searches(): GET /api/search_jobs
// - stop_search(id): PUT /api/search_jobs/:id/stop
// - get_search_status(id): GET /api/search_jobs/:id
```

**Key files:** `~/openclaw/workspace/skills/darkreach-search.js`

**Verification:** Skill can create and list searches via the API.

### 2.2 Developer Character

**Current:** Researcher character only.

**Target:** `darkreach-developer` character with search + DB skills plus code tools, Sonnet/Opus model, $10/day budget.

**Key files:** `~/.openclaw/characters/darkreach-developer.json`

**Verification:**
- Agent picks up tasks from kanban board
- Agent creates branches and opens PRs
- PRs appear in `#approvals` for human review
- Daily spend stays under $10

### 2.3 Kanban Integration

**Current:** OCD kanban board deployed but no agent integration.

**Target:** Two-way sync between kanban cards and agent tasks.

**Implementation:**
- Agent polls kanban for `pending` cards assigned to `developer` role
- Agent updates card status as it works (`in_progress` → `review` → `completed`)
- Humans can create cards in kanban that agents pick up
- Status changes are reflected in both kanban and `agent_tasks` table

**Verification:**
- Create card in kanban → agent picks it up within 5 minutes
- Agent completes task → card moves to `completed`

### 2.4 PR Approval Workflow

**Current:** No automated PR creation from agents.

**Target:** Developer agent creates PRs and posts to `#approvals` for human review.

**Implementation:**
1. Agent creates branch (`feat/agent-<task-id>`)
2. Agent commits changes
3. Agent opens PR via `gh pr create`
4. Agent posts PR link to `#approvals` with summary
5. Human reviews and merges (or requests changes)
6. Agent responds to review comments if needed

**Verification:**
- PR created with descriptive title and summary
- Approval message appears in `#approvals`
- Agent responds to review feedback

---

## Phase 3: Operator Agent

> Third role. Validates monitoring skills, alert handling, and deployment triggers.

### 3.1 Monitor Skill

**Current:** Research, DB, and search skills.

**Target:** `darkreach-monitor` skill for fleet health and status.

**Implementation:**
```javascript
// ~/openclaw/workspace/skills/darkreach-monitor.js
// - fleet_status(): GET /api/fleet — online/stale/offline nodes
// - health_check(): GET /api/health — API health
// - active_searches(): GET /api/search_jobs?status=running
// - prometheus_query(metric): Query Prometheus for specific metrics
// - check_alerts(): Check for stale workers, high error rate, disk usage
```

**Key files:** `~/openclaw/workspace/skills/darkreach-monitor.js`

**Verification:** Skill returns fleet status with node health breakdown.

### 3.2 Deploy Skill

**Current:** Deployments are manual SSH + `deploy/production-deploy.sh`.

**Target:** `darkreach-deploy` skill that triggers deployments with human approval.

**Implementation:**
```javascript
// ~/openclaw/workspace/skills/darkreach-deploy.js
// - check_deploy_readiness(): Verify CI passing, no active searches at risk
// - request_deploy_approval(): Post to #approvals, wait for reaction
// - trigger_deploy(): SSH to coordinator, run deploy script
// - verify_deploy(): Health check + metric comparison pre/post
```

**Key files:** `~/openclaw/workspace/skills/darkreach-deploy.js`

**Verification:** Deploy skill creates approval request, waits for human, then deploys.

### 3.3 Operator Character

**Current:** Researcher + Developer characters.

**Target:** `darkreach-operator` character with monitor + deploy skills, Haiku model, $3/day budget, runs every 15 minutes.

**Key files:** `~/.openclaw/characters/darkreach-operator.json`

**Verification:**
- Agent posts fleet status to `#fleet-status` every 15 minutes
- Agent detects stale workers and posts alerts
- Agent triggers deployment on schedule (with approval)
- Daily spend stays under $3

### 3.4 Alert Rules

**Current:** Grafana alerting rules exist but don't reach Discord.

**Target:** Operator agent monitors for alert conditions and posts to Discord.

**Alert conditions:**

| Alert | Condition | Channel | Action |
|-------|-----------|---------|--------|
| Worker stale | Heartbeat > 5 min | `#fleet-status` | Notify + investigate |
| API errors | 5xx rate > 5% for 5 min | `#fleet-status` | Notify + check logs |
| Disk full | Usage > 90% | `#fleet-status` | Notify + suggest cleanup |
| No workers | `workers_online == 0` for 10 min | `#fleet-status` | Notify + escalate |
| Search stalled | No progress for 30 min | `#fleet-status` | Notify + check node |

**Verification:** Simulate alert condition → message appears in `#fleet-status` within 2 minutes.

---

## Phase 4: Strategist Agent

> Fourth role. Validates AI engine data access, campaign analysis, and ROI evaluation.

### 4.1 AI Engine Data Skill

**Current:** Research, DB, search, monitor, deploy skills.

**Target:** Skill to query AI engine state — scoring weights, decisions, cost model.

**Implementation:**
```javascript
// Extended darkreach-db.js or new darkreach-strategy.js
// - get_scoring_weights(): Current 7-component weights
// - get_recent_decisions(n): Last N AI engine decisions with reasoning
// - get_cost_model(): Power-law coefficients per form
// - get_campaign_roi(project_id): Cost vs discoveries for a campaign
// - get_form_comparison(): Side-by-side form performance
```

**Key files:** `~/openclaw/workspace/skills/darkreach-strategy.js`

**Verification:** Skill returns scoring weights, recent decisions, and cost model data.

### 4.2 Strategist Character

**Current:** Researcher + Developer + Operator characters.

**Target:** `darkreach-strategist` character with strategy + DB skills, Opus model, $15/day budget, weekly analysis runs.

**Key files:** `~/.openclaw/characters/darkreach-strategist.json`

**Verification:**
- Agent produces weekly campaign ROI report
- Agent recommends form allocation changes based on scoring data
- Agent identifies opportunities from AI engine drift detection
- Daily spend stays under $15

### 4.3 Campaign Analysis Reports

**Current:** AI engine makes decisions but doesn't produce human-readable reports.

**Target:** Strategist agent generates weekly reports posted to Discord.

**Report contents:**
- Form performance comparison (yield rate, cost efficiency, record gap)
- Campaign ROI analysis (cost vs discoveries, time to next record)
- AI engine decision summary (what changed, why, confidence)
- Recommendations (reallocate compute, adjust parameters, new form targets)

**Verification:** Weekly report posted to `#research` with actionable recommendations.

---

## Phase 5: Multi-Agent Orchestration

> Cross-role workflows where agents collaborate on complex tasks.

### 5.1 Research → Strategy → Development Pipeline

**Target:** End-to-end workflow where a research finding triggers strategy evaluation and code changes.

**Example flow:**
1. Researcher detects new world record in palindromic primes (t5k.org scrape)
2. Researcher posts finding to `#research` and creates kanban card
3. Strategist picks up card, evaluates impact on our campaigns
4. Strategist recommends parameter adjustments, creates development task
5. Developer picks up task, implements changes, opens PR
6. Human reviews and merges

**Verification:** Full pipeline executes with minimal human intervention (only PR review).

### 5.2 Operator → Developer Incident Response

**Target:** Automated incident response where monitoring triggers code fixes.

**Example flow:**
1. Operator detects elevated error rate in API
2. Operator posts alert to `#fleet-status` with stack trace
3. Operator creates high-priority kanban card
4. Developer investigates, identifies root cause, creates fix
5. Operator verifies fix after deployment

**Verification:** Error detection to fix deployed in < 2 hours (with human approval gates).

### 5.3 Scheduled Campaign Reviews

**Target:** Periodic automated reviews combining all agent perspectives.

**Schedule:** Weekly (Sunday 18:00 UTC)

**Steps:**
1. Researcher gathers latest competitive intelligence
2. Strategist analyzes campaign performance
3. Operator reports fleet health and capacity
4. Combined report posted to `#research` with recommendations

**Verification:** Weekly combined report appears on schedule.

---

## Phase 6: AI Engine Integration

> OpenClaw agents feeding data back into the darkreach AI engine's decision loop.

### 6.1 Research → World Snapshot

**Current:** AI engine's `WorldSnapshot` is assembled from database queries only.

**Target:** Researcher agent writes competitive intelligence to a `competitive_intel` table that the AI engine reads during snapshot assembly.

**Implementation:**
1. New migration: `competitive_intel` table (form, record_holder, digit_count, date, source)
2. Researcher agent writes findings to this table via API
3. AI engine includes competitive intel in `WorldSnapshot.competition` component

**Key files:** `supabase/migrations/NNN_competitive_intel.sql`, `src/db/ai_engine.rs`, `src/ai_engine.rs`

**Verification:** AI engine scoring accounts for fresh competitive data from Researcher agent.

### 6.2 Strategist → Scoring Feedback

**Current:** AI engine learns weights via online gradient descent (EWA) on outcome data.

**Target:** Strategist agent can submit "human-like" feedback that influences weight updates.

**Implementation:**
1. New API endpoint: `POST /api/ai_engine/feedback` (with approval gate)
2. Strategist posts weight adjustment suggestions
3. AI engine incorporates feedback as a prior in the next EWA update

**Key files:** `src/dashboard/routes_strategy.rs`, `src/ai_engine.rs`

**Verification:** Strategist feedback influences next weight update (visible in `ai_engine_decisions` audit trail).

### 6.3 Operator → Fleet Actions

**Current:** AI engine can request fleet rebalancing but doesn't directly control nodes.

**Target:** Operator agent executes fleet actions recommended by the AI engine.

**Implementation:**
1. AI engine writes recommended actions to `ai_engine_decisions` with `action_type = 'fleet_rebalance'`
2. Operator agent polls for pending fleet actions
3. Operator executes actions (with approval for destructive operations)

**Key files:** `src/ai_engine.rs`, `~/openclaw/workspace/skills/darkreach-monitor.js`

**Verification:** AI engine recommendation → Operator agent execution → fleet state changes.

---

## Success Metrics

| Metric | Phase 0-1 | Phase 2-3 | Phase 4-5 | Phase 6 |
|--------|-----------|-----------|-----------|---------|
| Agent uptime | >95% | >98% | >99% | >99% |
| Budget adherence | Within 120% of cap | Within 110% | Within 105% | Within 105% |
| Discord response time | <5 min | <2 min | <1 min | <1 min |
| Tasks completed/week | 5 | 15 | 25 | 30+ |
| False alerts | <5/day | <2/day | <1/day | <1/day |
| Human interventions/day | 10+ | 5 | 2 | 1 |

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Runaway spending | Provider-level hard caps + per-agent daily limits |
| Agent hallucination (wrong action) | Approval gates for destructive actions + audit trail |
| OpenClaw instability | Docker restart policy + health check monitoring |
| Discord API rate limits | Batch messages, use threads for verbose output |
| Firecrawl scraping failures | Graceful degradation, cache previous results |
| Coordinator resource exhaustion | Memory limits on Docker containers, monitoring |

---

## Related Documents

- [OpenClaw Concept](../internal/openclaw.md) — Architecture, safety, deployment details
- [Agents Roadmap](agents.md) — Existing agent infrastructure
- [AI Engine Roadmap](ai-engine.md) — OODA decision loop
- [Path C Roadmap](path-c.md) — Hybrid compute marketplace strategy
- [Infrastructure](../internal/infrastructure.md) — Hosting and deployment
- [Services](../internal/services.md) — External service inventory
