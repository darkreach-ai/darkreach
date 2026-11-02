Work on the darkreach web dashboard frontend.

## Context

Read `frontend/CLAUDE.md` for the full component catalog and patterns.

**Stack:** Next.js 16 + React 19 + Tailwind 4 + shadcn/ui + Recharts + Supabase JS

**Data flow:**
- **Supabase** → prime data (queries, stats, charts, auth, realtime notifications)
- **WebSocket** → coordination data only (fleet, searches, deployments, status)
- Static export served by Rust backend (no Node.js runtime)

## Pages (14 routes)

| Route | File | Purpose |
|-------|------|---------|
| `/` | `app/page.tsx` | Main dashboard (stats, primes table, charts) |
| `/login` | `app/login/page.tsx` | Supabase Auth (email/password) |
| `/browse` | `app/browse/page.tsx` | Primes browser (filter, paginate, detail) |
| `/searches` | `app/searches/page.tsx` | Search job management |
| `/network` | `app/network/page.tsx` | Network monitoring |
| `/agents` | `app/agents/page.tsx` | AI agent management |
| `/projects` | `app/projects/page.tsx` | Project campaigns |
| `/leaderboard` | `app/leaderboard/page.tsx` | Form rankings |
| `/performance` | `app/performance/page.tsx` | Performance metrics |
| `/logs` | `app/logs/page.tsx` | System logs |
| `/releases` | `app/releases/page.tsx` | Worker releases |
| `/prime` | `app/prime/page.tsx` | Single prime permalink |
| `/docs` | `app/docs/page.tsx` | Documentation viewer (KaTeX) |

## Key Components

- `components/ui/` — 17 shadcn primitives (badge, button, card, dialog, table, tabs, etc.)
- `components/charts/` — 4 Recharts (discovery-timeline, digit-distribution, throughput-gauge, cost-history)
- `components/agents/` — 8 agent UI components (tasks, budgets, memory, schedules)
- `components/app-header.tsx` — Navigation (add links here for new pages)
- `components/primes-table.tsx` — Main primes table
- `components/prime-notifier.tsx` — Supabase Realtime toast notifications

## Hooks (18 total)

**Supabase:** use-primes, use-stats, use-timeline, use-distribution, use-form-leaderboard, use-records, use-prime-realtime
**WebSocket:** use-websocket
**Agents:** use-agents, use-agent-tasks, use-agent-budgets, use-agent-memory, use-agent-schedules
**UI:** use-theme, use-mobile, use-notifications, use-polling, use-projects

## Build & Dev

```bash
cd frontend
npm install
npm run dev        # Hot reload dev server
npm run build      # Static export to out/
npm test           # Vitest unit tests
npm run test:e2e   # Playwright E2E
```

## Conventions

- shadcn/ui components: `npx shadcn@latest add <name>`
- Tailwind utility classes only (no custom CSS except globals.css)
- TypeScript strict mode
- Icons: Lucide React
- Data fetching in hooks, not components
- localStorage keys: `darkreach-theme`, `darkreach-notifications-enabled`

## Adding a New Page

1. Create `src/app/<name>/page.tsx`
2. Add nav link in `src/components/app-header.tsx`
3. Create hook in `src/hooks/use-<name>.ts`
4. Use shadcn/ui + Tailwind for styling
5. Wrap with `AuthGuard` if auth required

## Backend API Reference

Routes defined in `src/dashboard/routes_*.rs`:
- `/api/health`, `/api/status` — Health/status
- `/api/workers`, `/api/network` — Network management
- `/api/search_jobs`, `/api/searches` — Search management
- `/api/agents/*` — Agent tasks, budgets, memory, roles
- `/api/projects/*` — Project campaigns
- `/api/observability/*` — Metrics, logs
- `/api/releases/*` — Release channels
- `/api/volunteer/*` — Volunteer management
- `/ws` — WebSocket (2s push interval)

## Roadmap

See `docs/roadmaps/frontend.md` for planned features.
