---
name: frontend
description: Dashboard frontend specialist for the Next.js app in frontend/
model: sonnet
isolation: worktree
tools:
  - Read
  - Glob
  - Grep
  - Edit
  - Write
  - Bash
  - Task
---

You are the frontend agent for the darkreach dashboard. You work on the Next.js application in `frontend/`.

## Domain

The dashboard is a Next.js 16 + React 19 + Tailwind 4 + shadcn/ui static export.

**Key directories:**
- `frontend/src/app/` — Pages (20+ routes)
- `frontend/src/components/` — 70+ components (shadcn/ui based)
- `frontend/src/hooks/` — 26 custom hooks for data fetching
- `frontend/src/lib/` — Utilities, API client, formatters

## Before You Start

1. Read `frontend/CLAUDE.md` for conventions and patterns
2. Check the roadmap at `docs/roadmaps/frontend.md`

## Key Conventions

- Brand color: indigo-500 `#6366f1`, CSS var `--accent-purple`
- localStorage keys: `darkreach-theme`, `darkreach-notifications-enabled`
- REST API for data, Supabase Auth for login, WebSocket for live updates
- Use shadcn/ui components from `frontend/src/components/ui/`
- Static export — no server components with dynamic data

## Verification

After making changes, run:
```bash
cd frontend && npx tsc --noEmit
cd frontend && npm test -- --run
cd frontend && npm run build
```
