# Phase 4: Updates & Polish

## Context

Phases 1-3 are complete (design system, navigation, homepage, all pages, blog with MDX). Phase 4 is the cleanup pass: terminology updates on existing pages, docs nav reorganization, metadata for SEO, and a count-up animation for the ProofBar.

## Scope Assessment

From the roadmap, Phase 4 has 11 items. After auditing the codebase:

- **ScrollAnimate** already exists and is used on the homepage (hero, vision cards, how-it-works, initiative preview, operator recruit, social proof, waitlist CTA). Adding it to every section on every page would be excessive — the homepage pattern is sufficient. **Skip pervasive scroll animation additions.**
- **Responsive QA** is a visual task requiring a browser. **Skip** (can't verify in CLI).
- **vercel.json** — no new subdomains needed. **Skip.**
- The docs sidebar nav structure already has 3 well-organized sections (Overview, Guides, Reference). The roadmap suggests reorganizing into 4 sections, but the current 3-section layout is cleaner and matches the existing pages. **Adjust the wording only** — update the description text in Network & Operators to remove "fleet" language.

**In scope (7 concrete steps):**

---

### Step 1: Rename fleet/worker/volunteer terminology in status page

**File: `website/src/app/status/page.tsx`**

| Old | New |
|-----|-----|
| `"Fleet Overview"` | `"Network Overview"` |
| `"Active Workers"` | `"Active Nodes"` |
| `fleetStats` import → `fallbackFleet` | `networkStats` import → `fallbackNetwork` |
| `fleet` state var | `network` state var |
| `FleetStats` type | `NetworkStats` type |
| `activeWorkers` field references | `activeNodes` field references |

Add `<WaitlistCTA />` section at the bottom of the page. Since this is a `"use client"` page, import WaitlistCTA and render it after the incidents section.

---

### Step 2: Rename fleet/worker/volunteer terminology in status-data.ts

**File: `website/src/lib/status-data.ts`**

| Old | New |
|-----|-----|
| `FleetStats` interface | `NetworkStats` interface |
| `activeWorkers` field | `activeNodes` field |
| `fleetStats` export | `networkStats` export |
| `"Worker heartbeat delays"` incident | `"Node heartbeat delays"` |
| `"3 workers"` in incident description | `"3 nodes"` |
| `"Workers auto-reconnected"` | `"Nodes auto-reconnected"` |

---

### Step 3: Rename volunteer terminology in leaderboard page

**File: `website/src/app/leaderboard/page.tsx`**

| Old | New |
|-----|-----|
| `"Total Volunteers"` | `"Total Operators"` |
| `leaderboardStats.totalVolunteers` | `leaderboardStats.totalOperators` |

Add a "Join the leaderboard" CTA after the team standings section — a simple card with text + link to `/operators`.

Add `<WaitlistCTA />` at the bottom.

---

### Step 4: Rename volunteer terminology in leaderboard-data.ts

**File: `website/src/lib/leaderboard-data.ts`**

| Old | New |
|-----|-----|
| `totalVolunteers` | `totalOperators` |

---

### Step 5: Update docs page description

**File: `website/src/app/docs/page.tsx`**

Update the intro paragraph to reflect the broader platform vision:

Old: "darkreach is an AI-driven distributed computing platform for hunting special-form prime numbers. It combines high-performance number theory algorithms with autonomous AI agents to push the boundaries of mathematical discovery."

New: "darkreach is an open-source distributed computing platform that turns idle compute into scientific discovery. Prime number research is our first initiative — explore the docs to learn how the engine, AI orchestration, and network architecture work together."

Update the "Network & Operators" card description to remove "fleet":

Old: `"Operators, nodes, work distribution, and joining the fleet."`
New: `"Operators, nodes, work distribution, and joining the network."`

---

### Step 6: Add count-up animation to ProofBar

**File: `website/src/components/proof-bar.tsx`**

Add an IntersectionObserver-based count-up effect. When the ProofBar enters the viewport, animate the stat numbers from 0 to their target values over ~1.5 seconds with an easing function.

Implementation: a `useCountUp` hook or inline logic that:
1. Parses the target string to extract the numeric portion and suffix (e.g., "392K+" → 392, "K+")
2. On intersection, interpolates from 0 to the target number over ~1.5s using `requestAnimationFrame`
3. Formats the interpolated value with the suffix at each frame
4. Falls back gracefully if stats change from API (re-animate on value change)

---

### Step 7: Add metadata to pages missing it

Pages missing `export const metadata`: status (client component — needs separate approach), operators, download, download/server, download/worker, and all 9 docs subpages.

**For `"use client"` pages (status):** Can't export `metadata` from client components. Wrap in a parent layout or accept the root layout title template. Status already has the root layout's `%s — darkreach` template. **Skip** — the status page title comes from the root template.

**For server component pages — add `metadata` export:**

| Page | Title | Description |
|------|-------|-------------|
| `operators/page.tsx` | "Operators" | "Contribute your compute to scientific discovery. Join the darkreach network." |
| `download/page.tsx` | "Download" | "Install darkreach and start contributing compute." |
| `download/server/page.tsx` | "Coordinator Setup" | "Set up a darkreach coordinator server with systemd and Nginx." |
| `download/worker/page.tsx` | "Node Deployment" | "Deploy darkreach nodes and scale your contribution." |
| `docs/getting-started/page.tsx` | "Getting Started" | "Install, build, and run your first darkreach search." |
| `docs/architecture/page.tsx` | "Architecture" | "Five-layer system architecture: engine, AI, server, database, and frontend." |
| `docs/prime-forms/page.tsx` | "Prime Forms" | "All 12 special prime forms with formulas, algorithms, and CLI commands." |
| `docs/ai-engine/page.tsx` | "AI Engine" | "Autonomous OODA decision loop, scoring model, and cost prediction." |
| `docs/projects/page.tsx` | "Projects & Campaigns" | "Multi-phase research campaigns with budgets and orchestration." |
| `docs/network/page.tsx` | "Network & Operators" | "Operators, nodes, work distribution, and network architecture." |
| `docs/verification/page.tsx` | "Verification" | "Three-tier verification pipeline with deterministic primality certificates." |
| `docs/api/page.tsx` | "API Reference" | "REST endpoints, WebSocket protocol, and response schemas." |
| `docs/contributing/page.tsx` | "Contributing" | "How to contribute to darkreach: workflow, code style, and testing." |

---

## File Summary

### Modified files (12)
| File | Step | Scope |
|------|------|-------|
| `website/src/app/status/page.tsx` | 1 | Rename fleet→network, workers→nodes, add WaitlistCTA |
| `website/src/lib/status-data.ts` | 2 | Rename FleetStats→NetworkStats, fleet→network |
| `website/src/app/leaderboard/page.tsx` | 3 | Rename volunteers→operators, add join CTA + WaitlistCTA |
| `website/src/lib/leaderboard-data.ts` | 4 | Rename totalVolunteers→totalOperators |
| `website/src/app/docs/page.tsx` | 5 | Update intro text, fix "fleet" in Network card |
| `website/src/components/proof-bar.tsx` | 6 | Add count-up animation on viewport entry |
| `website/src/app/operators/page.tsx` | 7 | Add metadata |
| `website/src/app/download/page.tsx` | 7 | Add metadata |
| `website/src/app/download/server/page.tsx` | 7 | Add metadata |
| `website/src/app/download/worker/page.tsx` | 7 | Add metadata |
| 9 docs subpages | 7 | Add metadata to each |

### New files
None.

### Unchanged files
| File | Reason |
|------|--------|
| `website/src/lib/docs-nav.ts` | Current 3-section structure is clean; only text fix in docs page |
| `website/src/components/scroll-animate.tsx` | Already used on homepage; no changes needed |
| `website/vercel.json` | No new subdomains |

---

## Verification Checklist

- [ ] `npm run build` succeeds (~27 routes)
- [ ] Status page says "Network Overview" and "Active Nodes" (no fleet/worker language)
- [ ] Leaderboard says "Total Operators" (no volunteer language)
- [ ] Leaderboard has "Join the leaderboard" CTA linking to `/operators`
- [ ] Both status and leaderboard have waitlist CTA at bottom
- [ ] Docs page intro reflects broader platform vision
- [ ] Docs "Network & Operators" card says "network" not "fleet"
- [ ] ProofBar stats animate from 0 on viewport entry
- [ ] All non-client pages have metadata with title + description
- [ ] `npx tsc --noEmit` passes
