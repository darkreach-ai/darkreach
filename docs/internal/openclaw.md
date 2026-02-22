# OpenClaw Integration

> *Team-facing AI orchestration — Discord dispatch, kanban tracking, multi-model routing, web research.*

---

## Why OpenClaw

darkreach already has two AI systems:

1. **AI Engine** (`src/ai_engine.rs`) — Autonomous OODA loop that manages search campaigns 24/7. It scores forms, allocates compute, rebalances fleets, and learns from results. No human interaction needed.
2. **Agent System** (`src/agent.rs`) — Spawns Claude Code CLI subprocesses with budget controls, permission levels, and task hierarchies. Handles code tasks dispatched from the dashboard.

What's missing is the **team-facing orchestration layer** — the glue between humans (Discord), visual task management (kanban), multi-model LLM routing (cost optimization), and web research (t5k.org, OEIS, competitor monitoring). OpenClaw fills this gap.

**OpenClaw** is an open-source autonomous AI agent platform (Node.js, 145K+ GitHub stars):

- **Gateway architecture**: Single daemon that owns messaging surfaces (Discord, Slack, Telegram)
- **Agent runs**: Spawns agent sessions via WebSocket, streams progress, persists results
- **OpenRouter integration**: 200+ LLM models with automatic routing and cost tracking
- **Firecrawl integration**: Web scraping and research (no browser needed)
- **Community kanban**: ClawDeck / OCD (OpenClaw Dashboard) for visual task management
- **Custom skills**: Extend agents with project-specific API integrations

OpenClaw runs **alongside** the existing systems. It doesn't replace the AI engine or the agent infrastructure — it adds a team interaction layer on top.

---

## Architecture

```
                    Discord (team)
                         │
                         ▼
              ┌─────────────────────┐
              │   OpenClaw Gateway   │
              │   (Docker, :18789)   │
              │                     │
              │  ┌───────────────┐  │
              │  │ Custom Skills │  │
              │  │  (darkreach)  │──────────► darkreach API (api.darkreach.ai)
              │  └───────────────┘  │              │
              │  ┌───────────────┐  │              ▼
              │  │  OpenRouter   │  │        ┌──────────┐
              │  │ (multi-model) │  │        │  Axum    │
              │  └───────────────┘  │        │ Dashboard│
              │  ┌───────────────┐  │        │ (port    │
              │  │  Firecrawl    │  │        │  7001)   │
              │  │ (web scrape)  │  │        └────┬─────┘
              │  └───────────────┘  │             │
              └─────────────────────┘             ▼
                         │                  ┌──────────┐
                         ▼                  │ Supabase │
              ┌─────────────────────┐       │PostgreSQL│
              │   OpenClaw Kanban   │       └────┬─────┘
              │   (OCD, :18790)     │            │
              └─────────────────────┘       ┌────┴────┐
                                            ▼         ▼
                                        Compute    Compute
                                        Node 1     Node N

Existing (unchanged):
  darkreach AI Engine (OODA loop) → PostgreSQL → Compute Nodes
  darkreach Agent System (src/agent.rs) → Claude CLI subprocesses
```

### Two Systems, Complementary Roles

| System | Scope | Interaction | Runs |
|--------|-------|-------------|------|
| **darkreach AI Engine** | Search campaign management — scoring, allocation, fleet rebalancing | Fully autonomous, no human interaction | 24/7 on coordinator |
| **OpenClaw Agents** | Team-facing work — development, research, monitoring, deployments | Dispatched from Discord, tracked on kanban | On demand or scheduled |

The AI engine optimizes *what to search for*. OpenClaw agents handle *everything else* — the development, research, monitoring, and operational work that a human team member would do.

---

## Custom Skills

OpenClaw agents interact with darkreach through custom skills — thin wrappers around the REST API.

| Skill | What it does | API endpoints used |
|-------|-------------|-------------------|
| `darkreach-search` | Create, manage, and monitor search campaigns | `POST /api/searches`, `GET /api/search_jobs`, `PUT /api/search_jobs/:id` |
| `darkreach-monitor` | Fleet status, prime discoveries, alerts | `GET /api/status`, `GET /api/fleet`, `GET /api/primes` |
| `darkreach-research` | Scrape t5k.org, OEIS, mersenneforum via Firecrawl | Firecrawl API + `GET /api/records`, `GET /api/primes` |
| `darkreach-deploy` | Trigger deployments, check health | SSH via deploy scripts, `GET /api/health` |
| `darkreach-db` | Query prime records, search stats, cost data | `GET /api/observability/*`, `GET /api/projects/*` |

### Skill Implementation

Each skill is a JavaScript module in the OpenClaw workspace:

```
~/openclaw/workspace/skills/
├── darkreach-search.js    # Search CRUD + monitoring
├── darkreach-monitor.js   # Fleet + discovery alerts
├── darkreach-research.js  # t5k.org, OEIS, mersenneforum scraping
├── darkreach-deploy.js    # Deployment triggers + health checks
└── darkreach-db.js        # Records, stats, observability queries
```

Skills call the darkreach API at `https://api.darkreach.ai` using the standard REST endpoints documented in the OpenAPI spec (`src/dashboard/openapi.rs`).

---

## Agent Roles

Roles are deployed incrementally, one at a time. Each role is an OpenClaw agent configuration with specific skills, model, and budget.

| Role | Priority | What it does | Model | Daily Budget |
|------|----------|-------------|-------|-------------|
| **Researcher** | 1st | Scrapes t5k.org, OEIS, competitor sites. Monitors world records. Reports findings to Discord. Analyzes search ROI. | Sonnet (cost-effective for research) | $5/day |
| **Developer** | 2nd | Handles code changes, bug fixes, features. Creates PRs. Runs tests. Works on tasks from kanban board. | Sonnet/Opus (Opus for complex tasks) | $10/day |
| **Operator** | 3rd | Monitors fleet health, node status, stale workers. Triggers deployments. Handles alerts. Checks Prometheus metrics. | Haiku (cheap, fast for monitoring) | $3/day |
| **Strategist** | 4th | Analyzes search results, evaluates campaign ROI, recommends form allocation. Interfaces with AI engine data. | Opus (needs deep reasoning) | $15/day |

### Role Configurations

Each role maps to an OpenClaw character file:

```
~/.openclaw/characters/
├── darkreach-researcher.json
├── darkreach-developer.json
├── darkreach-operator.json
└── darkreach-strategist.json
```

Example (Researcher):

```json
{
  "name": "darkreach-researcher",
  "description": "Monitors prime discovery landscape, scrapes t5k.org and OEIS, reports findings",
  "model": "anthropic/claude-sonnet-4-20250514",
  "skills": ["darkreach-research", "darkreach-db", "firecrawl"],
  "budget": { "daily_usd": 5, "weekly_usd": 30 },
  "schedule": "0 */6 * * *"
}
```

### Role Progression

```
Phase 1: Researcher only
  └── Validates skill system, Firecrawl integration, Discord posting

Phase 2: + Developer
  └── Validates code skills, PR workflow, kanban integration

Phase 3: + Operator
  └── Validates monitoring skills, alert handling, deploy triggers

Phase 4: + Strategist
  └── Validates AI engine data access, campaign analysis

Phase 5: Cross-role workflows
  └── Researcher finds opportunity → Strategist evaluates → Developer implements
```

---

## Safety Model

### Defense in Depth

Five layers, from coarsest to finest:

```
Layer 1: Provider hard caps (OpenRouter account spending limit)
    └── The real circuit breaker. Set at the provider level.
    └── Even if all other controls fail, spending stops here.

Layer 2: Per-agent budget limits (OpenClaw config)
    └── Daily and weekly caps per role.
    └── Agent session terminates when budget exhausted.

Layer 3: Permission gates (human approval)
    └── Destructive actions require Discord approval.
    └── PRs, deployments, DB writes, git push → approval channel.

Layer 4: Isolation (restricted tool policies)
    └── Each role runs in its own OpenClaw session.
    └── Tool access is scoped to the role's skills only.

Layer 5: Audit trail (full logging)
    └── All agent actions logged to PostgreSQL + Discord.
    └── Reviewable after the fact.
```

### Approval Workflow

Actions that require human approval before execution:

| Action | Approval Channel | Timeout |
|--------|-----------------|---------|
| Git push / PR creation | `#approvals` | 30 min |
| Production deployment | `#approvals` | No timeout (must be explicit) |
| Database writes (INSERT/UPDATE/DELETE) | `#approvals` | 15 min |
| Budget increase request | `#approvals` | No timeout |
| High-cost operation (>$2 estimated) | `#approvals` | 15 min |

### Kill Switch

Discord command to immediately halt all agents:

```
/darkreach halt-all
```

This sends SIGTERM to all running OpenClaw agent sessions and disables the task scheduler until manually re-enabled.

### Budget Controls

| Control | Scope | Default |
|---------|-------|---------|
| OpenRouter account limit | All agents combined | $100/month |
| Researcher daily cap | Single role | $5/day |
| Developer daily cap | Single role | $10/day |
| Operator daily cap | Single role | $3/day |
| Strategist daily cap | Single role | $15/day |
| Per-session max | Single agent run | $5/session |

---

## Discord Setup

### Bot Configuration

Create a Discord bot with the following permissions:

- Send Messages, Embed Links, Attach Files (for reports)
- Read Message History (for context)
- Use Slash Commands (for `/darkreach` commands)
- Manage Threads (for task discussion threads)

### Channel Structure

| Channel | Purpose | Who posts |
|---------|---------|-----------|
| `#agent-feed` | All agent activity (task started, completed, failed) | All agents |
| `#discoveries` | Prime discovery announcements (from darkreach events) | Operator agent |
| `#fleet-status` | Network health, node alerts, deployment notifications | Operator agent |
| `#agent-commands` | Team dispatches tasks to agents (primary interaction channel) | Humans → Agents |
| `#approvals` | Human approval requests (PRs, deploys, high-cost operations) | Agents → Humans |
| `#research` | Researcher agent reports (t5k.org updates, competitor analysis) | Researcher agent |

### Slash Commands

| Command | What it does |
|---------|-------------|
| `/darkreach status` | Show fleet status, active searches, agent states |
| `/darkreach search <form> <params>` | Create a new search campaign |
| `/darkreach task <description>` | Create a task for the Developer agent |
| `/darkreach research <topic>` | Send Researcher agent to investigate |
| `/darkreach deploy` | Trigger production deployment (requires approval) |
| `/darkreach halt-all` | Kill switch — stop all agents immediately |
| `/darkreach budget` | Show current budget usage across all agents |

---

## Kanban Board

### OCD (OpenClaw Dashboard)

Adopt **OCD** — a lightweight, open-source kanban board designed for OpenClaw:

- **6 status columns**: `pending` → `in_progress` → `review` → `blocked` → `completed` → `icebox`
- **3 priority levels**: Low, Medium, High (matches darkreach's existing `agent_tasks` priority)
- **Drag-and-drop**: Move tasks between columns manually or via agent updates
- **Agent integration**: OpenClaw agents can create, update, and complete kanban cards
- **Accessible at**: `http://coordinator-ip:18790/` (separate port from darkreach dashboard)

### Task Flow

```
Human creates task in Discord (#agent-commands)
    │
    ▼
OpenClaw creates kanban card (pending)
    │
    ▼
Agent claims task → card moves to in_progress
    │
    ├── Agent works on task (streams progress to #agent-feed)
    │
    ├── If PR needed → card moves to review → human reviews
    │
    └── Task complete → card moves to completed
```

### Integration with Existing Agent Tasks

The OpenClaw kanban syncs with darkreach's `agent_tasks` table:

| Kanban Column | `agent_tasks.status` |
|---------------|---------------------|
| `pending` | `pending` |
| `in_progress` | `in_progress` |
| `review` | `in_progress` (with `needs_review` tag) |
| `blocked` | `blocked` |
| `completed` | `completed` |
| `icebox` | `cancelled` |

---

## Deployment

### Docker Compose (Hetzner Coordinator)

OpenClaw runs alongside the darkreach coordinator on the same server. Docker Compose keeps it isolated.

```yaml
# deploy/docker-compose.openclaw.yml
version: "3.8"

services:
  openclaw:
    image: openclaw/openclaw:latest
    container_name: darkreach-openclaw
    volumes:
      - ~/.openclaw:/root/.openclaw
      - ~/openclaw/workspace:/root/openclaw/workspace
    ports:
      - "127.0.0.1:18789:18789"   # OpenClaw web UI (localhost only, behind Nginx)
    environment:
      - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
      - FIRECRAWL_API_KEY=${FIRECRAWL_API_KEY}
      - DISCORD_TOKEN=${DISCORD_BOT_TOKEN}
    restart: unless-stopped
    mem_limit: 512m

  openclaw-kanban:
    image: ghcr.io/keeeeeeeks/opencode-dashboard:latest
    container_name: darkreach-kanban
    ports:
      - "127.0.0.1:18790:3000"    # Kanban board (localhost only, behind Nginx)
    restart: unless-stopped
    mem_limit: 256m
```

### Nginx Configuration

Add to the existing Nginx config (`deploy/nginx-darkreach.conf`):

```nginx
# OpenClaw web UI (internal access only)
server {
    listen 443 ssl;
    server_name openclaw.darkreach.ai;

    location / {
        proxy_pass http://127.0.0.1:18789;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        # WebSocket support for agent streaming
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}

# Kanban board (internal access only)
server {
    listen 443 ssl;
    server_name kanban.darkreach.ai;

    location / {
        proxy_pass http://127.0.0.1:18790;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Resource Budget

The coordinator (CX22, 2 vCPU, 4GB RAM) also runs the darkreach dashboard. OpenClaw containers are memory-limited:

| Container | Memory Limit | CPU | Notes |
|-----------|-------------|-----|-------|
| `darkreach-openclaw` | 512 MB | Shared | Gateway + agent sessions |
| `darkreach-kanban` | 256 MB | Shared | Static web app |
| darkreach dashboard | ~256 MB | Shared | Existing Axum server |
| **Total coordinator** | ~1 GB | 2 vCPU | Fits in CX22 (4 GB RAM) |

---

## Secrets & Environment

### New Environment Variables

| Variable | Service | Source |
|----------|---------|--------|
| `OPENROUTER_API_KEY` | OpenClaw → OpenRouter | OpenRouter account (already available) |
| `FIRECRAWL_API_KEY` | OpenClaw → Firecrawl | Firecrawl account (already available) |
| `DISCORD_BOT_TOKEN` | OpenClaw → Discord | Discord Developer Portal (to be created) |
| `DISCORD_WEBHOOK_URL` | darkreach → Discord | Discord channel settings (for notifications) |

### SOPS Integration

Add to the existing SOPS-encrypted secrets:

```bash
# Add new keys to secrets/openclaw.enc.yaml
sops --encrypt --age $(cat ~/.sops/age-public-key) <<EOF > secrets/openclaw.enc.yaml
openrouter_api_key: "sk-or-..."
firecrawl_api_key: "fc-..."
discord_bot_token: "..."
discord_webhook_url: "https://discord.com/api/webhooks/..."
EOF
```

Decrypt script (`scripts/decrypt-secrets.sh`) updated to generate `.env.openclaw`:

```bash
# Added to decrypt-secrets.sh
sops -d secrets/openclaw.enc.yaml | yq -r 'to_entries | .[] | "\(.key | ascii_upcase)=\(.value)"' > .env.openclaw
```

---

## Phased Rollout

| Phase | What | Duration | Success Metric |
|-------|------|----------|---------------|
| **Phase 0** | Deploy OpenClaw + kanban on coordinator, Discord bot setup | 1 day | Gateway running, Discord connected |
| **Phase 1** | Researcher role | 1 week | Agent scraping t5k.org, posting findings to Discord |
| **Phase 2** | Developer role | 1 week | Agent completing tasks from kanban, creating PRs |
| **Phase 3** | Operator role | 1 week | Agent monitoring fleet, alerting on issues |
| **Phase 4** | Strategist role | 1 week | Agent analyzing campaigns, recommending allocation |
| **Phase 5** | Cross-role workflows | Ongoing | Multi-agent task decomposition working |

### Phase 0: Infrastructure (Day 1)

1. Create Discord server with channel structure
2. Create Discord bot in Developer Portal
3. Add secrets to SOPS
4. Deploy Docker Compose on coordinator
5. Verify OpenClaw gateway connects to Discord
6. Verify kanban board accessible

### Phase 1: Researcher (Week 1)

1. Write `darkreach-research` skill (Firecrawl + records API)
2. Create `darkreach-researcher` character file
3. Configure scheduled runs (every 6 hours)
4. Verify: agent posts t5k.org updates to `#research`
5. Verify: agent posts competitor analysis reports
6. Monitor: budget stays within $5/day

### Phase 2: Developer (Week 2)

1. Write `darkreach-search` and `darkreach-db` skills
2. Create `darkreach-developer` character file
3. Set up kanban → agent task flow
4. Verify: agent picks up kanban tasks, creates branches, opens PRs
5. Verify: approval workflow works in `#approvals`
6. Monitor: budget stays within $10/day

### Phase 3: Operator (Week 3)

1. Write `darkreach-monitor` and `darkreach-deploy` skills
2. Create `darkreach-operator` character file
3. Configure alert triggers (stale workers, high error rate, disk full)
4. Verify: agent posts fleet status to `#fleet-status`
5. Verify: agent triggers deploy on approval
6. Monitor: budget stays within $3/day

### Phase 4: Strategist (Week 4)

1. Write AI engine data access skill (scoring, decisions, cost model)
2. Create `darkreach-strategist` character file
3. Configure weekly analysis runs
4. Verify: agent produces campaign ROI reports
5. Verify: agent recommends form allocation changes
6. Monitor: budget stays within $15/day

### Phase 5: Cross-Role Workflows (Ongoing)

1. Test: Researcher finds opportunity → Strategist evaluates → Developer implements
2. Test: Operator detects issue → Developer fixes → Operator verifies
3. Tune: Adjust budgets, schedules, and permissions based on observed patterns
4. Document: Capture effective workflows for team reference

---

## Interaction with Existing Systems

### AI Engine

The AI engine continues to run autonomously. OpenClaw agents can **read** AI engine state (scoring weights, decisions, cost model) but cannot **write** to it directly. The Strategist agent may recommend changes that a human approves and applies.

```
AI Engine (autonomous)  ←── read-only ←── Strategist Agent
                        ──► PostgreSQL ──► darkreach API ──► OpenClaw skills
```

### Agent System (src/agent.rs)

The existing agent system handles code-level tasks spawned from the dashboard. OpenClaw agents handle team-level orchestration. They can coexist:

- **Dashboard agents**: Triggered from the web UI, run Claude Code CLI, operate on the codebase
- **OpenClaw agents**: Triggered from Discord or schedule, run via OpenRouter models, operate on APIs and web research

If a task requires direct code changes, the OpenClaw Developer agent can delegate to the dashboard agent system.

### Events System

darkreach's event bus (`src/events.rs`) emits prime discovery events. The Operator agent subscribes to these events (via WebSocket at `wss://api.darkreach.ai/ws`) and posts announcements to `#discoveries`.

---

## Related Documents

- [OpenClaw Roadmap](../roadmaps/openclaw.md) — Phased implementation plan
- [Agents Roadmap](../roadmaps/agents.md) — Existing agent infrastructure
- [AI Engine Roadmap](../roadmaps/ai-engine.md) — OODA decision loop
- [Infrastructure](infrastructure.md) — Hosting and deployment details
- [Services](services.md) — External service inventory
- [Path C Roadmap](../roadmaps/path-c.md) — Hybrid compute marketplace strategy
- [Business](business.md) — Business model and market analysis
