# Website v1 Rework Roadmap

Complete rework of the darkreach.ai marketing website — from "prime search tool" to "world's biggest supercomputer."

**Key files:** `website/`

**Related roadmaps:** [website.md](website.md), [frontend.md](frontend.md), [technology-vision.md](technology-vision.md)

---

## Context

The current website was built when darkreach was a prime number search tool. The project has since evolved into a general-purpose distributed compute platform expanding into research initiatives and university partnerships. The website needs to reflect this shift: primes become the *first* research initiative, not the entire product.

### Design Heroes

- **Stripe** — Clean typography, generous whitespace, subtle animations, gradient accents, crystal-clear hierarchy, flat/scannable navigation (no dropdowns)
- **GitHub** — Developer-friendly, dark mode done right, product screenshots, community focus
- **Apple** — Hero-centric, cinematic scrolling, bold statements, let the product breathe

### Core Design Direction

The current aesthetic reads as "developer tool" — it needs to become "world-changing platform." Target: **Stripe meets GitHub's dark mode. Clean, confident, premium. Less is more.**

### Key Messaging Pillars

1. **"The world's biggest supercomputer"** — Lead with the vision, not the current product
2. **"Solving the world's biggest problems"** — Aspirational, research-focused
3. **"Open source, operator-owned"** — Trust and transparency differentiator
4. **"AI-orchestrated"** — Technical sophistication without jargon
5. **"Contribute your compute"** — Clear operator value proposition

### Messaging Guidelines

| Do say | Don't say |
|--------|-----------|
| Research initiatives | Prime hunting |
| Operators | Workers / volunteers |
| Discoveries | Primes found |
| The network | The fleet |
| Contribute compute | Donate CPU cycles |

**Key narrative device:** "Prime number discovery is our first research initiative." This acknowledges what exists without being limited by it, implies more initiatives are coming, and positions the technology as general-purpose.

---

## Navigation Architecture

### Desktop Navbar

```
[logo] darkreach          Platform    Research    Operators    About    Blog     [Join Waitlist]
```

5 nav items + 1 CTA button. Flat links, **no dropdowns** (Stripe pattern). Waitlist button is indigo-500 pill.

| Item | Route | Audience |
|------|-------|----------|
| Platform | `/platform` | Engineers, researchers evaluating the tech |
| Research | `/research` | Academics, press, university partners |
| Operators | `/operators` | People who want to contribute compute |
| About | `/about` | Everyone — press, investors, curious visitors |
| Blog | `/blog` | All segments |

**Removed from nav:** Docs (footer + linked from relevant pages), Download (merged into Operators), Status (footer link), Leaderboard (footer link).

### Mobile Nav

Full-screen slide-over from right:

```
[logo] darkreach                                    [X]

Platform
Research
Operators
About
Blog

---
Status    Leaderboard    Docs
---
Discord    GitHub    X/Twitter
---
[Join Waitlist]  (full-width button, indigo-500)
```

### CTA Button Behavior

1. On pages with inline waitlist form (homepage) — smooth-scroll to the form section
2. On other pages — open lightweight modal with email input + "Join" button
3. Store submissions via Supabase function or Loops.so / Resend

### Footer (5 columns)

```
[logo] darkreach
Building the world's biggest supercomputer.

Platform          Research          Company        Community        Live
How It Works      Prime Discovery   About          GitHub           Leaderboard
Technology        For Universities  Blog           Discord          Status
Status            Initiatives       Careers        X/Twitter
Docs                                Open Source

Legal
MIT License
Privacy

---
© 2026 darkreach. Open source under MIT.
```

Five columns on desktop, 2 on tablet, stacked on mobile.

---

## Page Map

### Page 1: Homepage (`/`) — COMPLETE REWRITE

**Purpose:** Convert visitors into waitlist signups. Communicate the vision in under 10 seconds.

**Sections (top to bottom):**

1. **Hero** — Full viewport. Single bold headline: "The world's biggest supercomputer." Subhead: "A global network of contributed compute, solving problems too big for any one machine." Inline email input as primary CTA. Three.js node network background (subtler — fewer nodes, slower, lower opacity). No rotating headline carousel. Secondary CTA: "Learn how it works" (arrow down, scrolls).

2. **Proof Bar** — Horizontal strip with 4 live stats using generic labels: "392K+ discoveries" / "14.2B candidates tested" / "4 active nodes" / "99.9% uptime". Green live indicator. Replaces current `StatsBar`.

3. **Vision Triptych** — Three large cards (Stripe-style): "Massive Scale" / "AI-Orchestrated" / "Open & Verifiable". Icon + title + 2-sentence description each. Subtle border transition on hover.

4. **How It Works** — 3-step horizontal flow (Apple-style): "Researchers define problems" → "Operators contribute compute" → "AI orchestrates discovery". Connecting line with muted gradient.

5. **First Research Initiative** — Section header: "Our first initiative: Prime Number Discovery". Brief paragraph on why primes matter. Mini live feed (3 recent discoveries). Link to `/research`. Primes should occupy ~15% of scroll length (down from ~60%).

6. **For Operators** — "Contribute your compute to discoveries that last forever." 3-step walkthrough with code snippet (`curl -sSf https://get.darkreach.ai | sh`). CTA to `/operators`.

7. **Social Proof** — "Trusted by researchers at..." with placeholder university logos (grayscale). Or testimonial quote if no logos yet.

8. **Waitlist CTA** — "Be part of something bigger." Email input + submit. "Join 500+ researchers and operators on the waitlist."

### Page 2: Platform (`/platform`) — NEW

Technology deep dive for technically curious visitors.

- **Page hero:** "One platform. Unlimited compute."
- **Architecture overview:** Clean SVG diagram of 5-layer stack (Apple chip architecture style, not code-heavy)
- **AI Engine:** OODA loop, scoring model, autonomous decisions (3 feature cards)
- **Compute Pipeline:** Generic framing — "Define → Distribute → Compute → Verify"
- **Network Architecture:** Coordinator + nodes diagram, trust levels
- **Technology Stack:** Grid of tech badges (Rust, PostgreSQL, GMP, Axum, Next.js)
- **CTA:** Docs link + waitlist

### Page 3: Research (`/research`) — NEW

- **Page hero:** "Solving the world's biggest problems, one computation at a time."
- **Active Initiatives** card grid:
  - Prime Number Discovery (active, flagship) — stats, mini feed, link to `/research/primes`
  - 2-3 "Coming Soon" placeholder cards (grayed out): "Protein Folding Verification" / "Climate Model Simulation" / "Cryptographic Research" — each with "Join waitlist to be notified"
- **For Universities:** "Partner with us" section with contact CTA
- **Publications & references**
- **Waitlist form**

### Page 4: Research/Primes (`/research/primes`) — NEW

All current prime-specific content relocated here.

- **Hero:** "Prime Number Discovery" as darkreach's first initiative
- **Why primes matter** (4-card grid)
- **12 Search Forms** (reuse `PrimeForms` component, collapsible cards)
- **Discovery Pipeline** (reuse `Pipeline` component)
- **Live Discoveries** (full `LiveFeed`)
- **Comparison vs GIMPS/PrimeGrid** (reuse `Comparison`)
- **Links to** `/operators` and `/docs/prime-forms`

### Page 5: Operators (`/operators`) — NEW

Merge of Download page + operator recruitment.

- **Hero:** "Your machine. Global discoveries."
- **Why Contribute:** 3 cards — Permanent Impact, Earn Recognition, Open Source
- **How to Get Started:** Tabbed install instructions with OS auto-detection (reuse `InstallCommand`)
- **System Requirements** table (reuse from current Download)
- **Deployment Guides:** Cards linking to coordinator + node setup docs
- **Live Network Stats** widget (active nodes, total cores, uptime)
- **CTA:** "Join the network" + leaderboard link

### Page 6: About (`/about`) — REWRITE

- **Mission:** "We're building the world's biggest supercomputer — open source, operator-owned, AI-orchestrated."
- **Timeline** (reuse `Timeline` component with updated events through 2026 H2)
- **Values:** 3-4 cards — "Transparent by Default" / "Compute for Everyone" / "Verifiable Results" / "Open Source Always"
- **Open Source** section with GitHub link
- **CTA:** Waitlist

### Page 7: Blog (`/blog` + `/blog/[slug]`) — UPGRADE

Upgrade from hardcoded array to MDX files.

- **Blog index:** Card grid (restyle `BlogCard`)
- **Individual posts:** MDX rendering with `BlogPostLayout` component
- **3 launch posts:**
  1. "Announcing darkreach: Building the World's Biggest Supercomputer"
  2. "Our First Research Initiative: Prime Number Discovery"
  3. "Why We're Open Source"
- MDX files in `website/content/blog/` directory

### Page 8: Status (`/status`) — LIGHT UPDATE

Keep current implementation. Changes:
- Rename "Fleet Overview" → "Network Overview"
- Rename "Active Workers" → "Active Nodes"
- Add waitlist CTA at bottom

### Page 9: Leaderboard (`/leaderboard`) — LIGHT UPDATE

Keep current. Changes:
- Rename "Total Volunteers" → "Total Operators"
- Add "Join the leaderboard" CTA linking to `/operators`
- Add waitlist CTA at bottom

### Page 10: Docs (`/docs/*`) — REORGANIZE

Keep sidebar layout. Reorganized nav sections:

```
Getting Started          Platform              Research            Community
  Overview               AI Engine             Prime Forms         Contributing
  Quick Start            Network & Operators   Verification
  Architecture           Projects
                         API Reference
```

All 9 existing doc pages kept. Content lightly updated for new messaging.

### Page 11: Download Pages — REDIRECT

- `/download` → redirect banner pointing to `/operators`
- `/download/server` and `/download/worker` remain (linked from Operators page)

---

## Design System Changes

### Color Palette

Shift from GitHub Primer blue-grays to warm zinc grays. More premium, less "code editor."

```css
:root {
  --background: #09090b;          /* Darker (was #0d1117) */
  --foreground: #fafafa;          /* Brighter white */
  --card: #111113;                /* Subtler card surface (was #161b22) */
  --card-foreground: #fafafa;
  --muted: #18181b;
  --muted-foreground: #71717a;    /* Warm zinc (was cold #8b949e) */
  --border: #27272a;              /* Zinc-800, subtler (was #30363d) */
  --primary: #6366f1;             /* Keep indigo-500 */
  --primary-foreground: #ffffff;
  --accent-green: #22c55e;        /* Green-500, slightly more vivid */
  --accent-orange: #f97316;       /* Orange-500 — status page only */
}
```

### Typography

Add **Inter** via `next/font/google`:

```css
--font-sans: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

Type scale refinement:

| Element | Current | Target |
|---------|---------|--------|
| Hero headline | `text-5xl sm:text-6xl lg:text-7xl` | `text-6xl lg:text-8xl font-bold tracking-[-0.04em]` |
| Page titles | varies | `text-4xl sm:text-5xl font-bold tracking-[-0.03em]` |
| Section headers | varies | `text-3xl font-semibold tracking-[-0.02em]` |
| Body | `text-base` | `text-base leading-relaxed` (unchanged) |
| Captions | `text-sm` | `text-sm text-muted-foreground` (unchanged) |

Remove `font-mono` from stats numbers. Use `tabular-nums` with sans-serif instead. Monospace reserved for code blocks only.

### Spacing

- Major sections: `py-32` (increased from `py-24`)
- Minor sections: `py-24`
- Increase whitespace ~30-40% overall

### Visual Restraint

- **Reduce gradient usage by ~80%:** `gradient-text` only on homepage hero headline. Remove `gradient-border-top`, `darkreach-glow`, `card-glow` effects from most uses.
- **Simplify card borders:** Default `border-border/40`, hover `border-border`. No glow shadows.
- **Reserve indigo for:** Logo, primary CTA button, active nav indicator, rare accents
- **Remove orange accent from nav:** Keep only for status page degraded state

### Animations

**Keep:**
- Three.js node network (reduced to ~50 desktop nodes, slower rotation, lower opacity)
- Hero logo draw-on animation
- `pulse-green` for live indicators (functional)
- Smooth scroll behavior

**Remove:**
- `darkreach-glow` text shadow
- `card-glow` hover effect
- `gradient-border-top::before` pseudo-element
- Rotating headline carousel in hero

**Add:**
- Scroll-triggered fade-in (`IntersectionObserver`, `opacity + translateY(20px)` → `opacity:1, translateY(0)`, 0.4s ease)
- Count-up animation for stats bar when entering viewport
- Subtle parallax on hero elements (Three.js scene moves slower than text on scroll)

---

## Component Inventory

### New Components to Create

| Component | File | Used On |
|-----------|------|---------|
| `WaitlistForm` | `waitlist-form.tsx` | Homepage, research, about, operators (inline + modal) |
| `WaitlistModal` | `waitlist-modal.tsx` | Triggered by nav CTA on non-homepage pages |
| `ProofBar` | `proof-bar.tsx` | Homepage (replaces StatsBar, generic labels) |
| `VisionCards` | `vision-cards.tsx` | Homepage (3-up card layout) |
| `HowItWorks` | `how-it-works.tsx` | Homepage (3-step horizontal flow) |
| `InitiativePreview` | `initiative-preview.tsx` | Homepage (condensed prime section) |
| `InitiativeCard` | `initiative-card.tsx` | Research page |
| `OperatorRecruit` | `operator-recruit.tsx` | Homepage + Operators page |
| `SocialProof` | `social-proof.tsx` | Homepage (logo strip / testimonials) |
| `PageHero` | `page-hero.tsx` | All subpages (reusable, Stripe-style) |
| `ValuesGrid` | `values-grid.tsx` | About page |
| `NetworkStats` | `network-stats.tsx` | Operators page (live metrics) |
| `BlogPostLayout` | `blog-post-layout.tsx` | Blog post pages (MDX renderer) |
| `ScrollAnimate` | `scroll-animate.tsx` | IntersectionObserver wrapper |

### Existing Components — Keep & Modify

| Component | Changes |
|-----------|---------|
| `navbar.tsx` | New nav items, waitlist CTA button, remove dropdowns |
| `footer.tsx` | New 5-column structure |
| `mobile-nav.tsx` | New sections matching nav restructure |
| `hero.tsx` | Major rewrite — single headline, inline email, subtler background |
| `node-network.tsx` | Reduce to ~50 nodes, slower rotation, lower opacity |
| `blog-card.tsx` | Restyle for cleaner aesthetic |
| `timeline.tsx` | Update events through 2026 H2 |
| `ui/card.tsx` | Add size variants (sm, md, lg) |
| `ui/section.tsx` | Add more padding options, max-width variants |

### Existing Components — Relocate to Subpages

These stay as files but are removed from homepage, used only on `/research/primes`:

- `comparison.tsx` — Used only on `/research/primes`
- `pipeline.tsx` — Used on `/platform` and `/research/primes`
- `prime-forms.tsx` — Used only on `/research/primes`
- `live-feed.tsx` — Condensed on homepage, full on `/research/primes`

### Existing Components — Replaced

| Old | Replaced By |
|-----|-------------|
| `stats-bar.tsx` | `proof-bar.tsx` |
| `feature-grid.tsx` | `vision-cards.tsx` |
| `cta-section.tsx` | `waitlist-form.tsx` + `operator-recruit.tsx` |
| `get-started.tsx` | Merged into `/operators` page |
| `mission.tsx` | Content absorbed into About page |
| `discoveries.tsx` | Replaced by `live-feed.tsx` usage |

### Existing Components — Keep As-Is

`darkreach-logo.tsx`, `hero-logo.tsx`, `status-card.tsx`, `uptime-bar.tsx`, `install-command.tsx`, `os-detector.tsx`, `doc-sidebar.tsx`, all `ui/*` components (badge, button, code-block)

---

## Content Strategy

### Blog Post Outlines (3 launch posts)

**Post 1: "Announcing darkreach: Building the World's Biggest Supercomputer"**
- The problem — most compute is wasted. What if we could pool it?
- The vision: A global network, operator-owned, AI-orchestrated
- What we've built so far: Prime discovery as proof of concept
- The roadmap: University partnerships, new research initiatives
- CTA: Join the waitlist

**Post 2: "Our First Research Initiative: Prime Number Discovery"**
- Why primes: Cryptography, unsolved conjectures, mathematical beauty
- 12 search forms explained (accessible, not overly technical)
- The pipeline: Sieve → Test → Prove
- Real results: Live from the network
- What's next: World-record targets
- CTA: Become an operator

**Post 3: "Why We're Open Source"**
- The problem with closed scientific computing
- MIT license: what it means for operators and researchers
- Verifiable proofs: trust the math, not us
- How to contribute
- CTA: View on GitHub

---

## Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Primary CTA | "Join Waitlist" (email) | Pre-launch, build audience |
| Prime content location | `/research/primes` subpage | Present but not dominant; homepage shows preview |
| Navigation depth | Flat (no dropdowns) | Stripe pattern; simpler, more scannable |
| Blog system | MDX files in `website/content/blog/` | Scales better than hardcoded `blog-posts.ts` array |
| Font | Inter via `next/font/google` | Industry standard for premium marketing sites |
| Color palette | Zinc-based (warm grays) | More premium than current blue-gray |
| Three.js background | Keep but reduce density | Communicates distributed computing visually |
| Docs | Keep sidebar layout, reorganize nav | Documentation important for operators/contributors |
| Status + Leaderboard | Keep, link from footer | Working live features should stay live |
| New dependencies | `@next/mdx` + `@mdx-js/loader` + `@mdx-js/react` | MDX blog rendering. No other new deps |

---

## Implementation Phases

### Phase 1: Foundation — Design System + Navigation + Homepage

**Status: TODO**

**Goal:** New navigation, new homepage, design system updates. This is the critical path — everything else builds on it.

**Scope:**

1. Update `website/src/app/globals.css` — new zinc color palette, remove excessive gradient utilities (`darkreach-glow`, `card-glow`, `gradient-border-top`)
2. Add Inter font via `next/font/google` in `website/src/app/layout.tsx`, update metadata
3. Rewrite `website/src/components/navbar.tsx` — new nav items (Platform, Research, Operators, About, Blog), waitlist CTA pill, remove dropdowns
4. Rewrite `website/src/components/footer.tsx` — new 5-column structure
5. Rewrite `website/src/components/mobile-nav.tsx` — new sections matching nav restructure
6. Create `website/src/components/waitlist-form.tsx` — inline email capture (reusable across pages)
7. Create `website/src/components/waitlist-modal.tsx` — modal variant triggered by nav CTA
8. Create `website/src/components/page-hero.tsx` — reusable page hero (Stripe-style)
9. Create `website/src/components/scroll-animate.tsx` — IntersectionObserver wrapper for fade-in
10. Rewrite `website/src/app/page.tsx` — complete homepage with all 8 sections
11. Create homepage components:
    - `website/src/components/proof-bar.tsx` (replaces `stats-bar.tsx`)
    - `website/src/components/vision-cards.tsx` (replaces `feature-grid.tsx`)
    - `website/src/components/how-it-works.tsx`
    - `website/src/components/initiative-preview.tsx`
    - `website/src/components/operator-recruit.tsx`
    - `website/src/components/social-proof.tsx`
12. Tune `website/src/components/node-network.tsx` — reduce to ~50 nodes, slower rotation, lower opacity
13. Create `website/src/lib/waitlist.ts` — submission logic (Supabase function endpoint)

**Files created:** 10 new components + 1 lib file
**Files modified:** `globals.css`, `layout.tsx`, `navbar.tsx`, `footer.tsx`, `mobile-nav.tsx`, `page.tsx`, `node-network.tsx`

**Acceptance criteria:**
- [ ] `npm run build` succeeds with zero errors
- [ ] Homepage renders all 8 sections in correct order
- [ ] Navigation shows 5 items + waitlist CTA on desktop
- [ ] Mobile nav opens as full-screen slide-over
- [ ] Footer renders 5 columns on desktop, stacks on mobile
- [ ] Waitlist form collects email (console log is fine for Phase 1, backend wired in Phase 5)
- [ ] Three.js background renders with reduced density
- [ ] Scroll animations trigger on viewport entry
- [ ] No `darkreach-glow`, `card-glow`, or `gradient-border-top` effects remain (except hero headline gradient)
- [ ] Inter font loads correctly
- [ ] Responsive at 375px, 768px, 1024px, 1440px

**Dependencies:** None (first phase)
**Complexity:** Large — ~15 files touched, homepage is the most complex page

---

### Phase 2: New Pages — Platform, Research, Operators, About

**Status: TODO**

**Goal:** All new marketing pages live. Visitors can navigate the full site.

**Scope:**

1. Create `website/src/app/platform/page.tsx` — technology deep dive with architecture diagram, AI engine, compute pipeline, network architecture, tech stack
2. Create `website/src/app/research/page.tsx` — initiatives index with active (Prime Discovery) and coming-soon placeholder cards
3. Create `website/src/app/research/primes/page.tsx` — relocate all prime-specific content (reuses `PrimeForms`, `Pipeline`, `LiveFeed`, `Comparison` components)
4. Create `website/src/app/operators/page.tsx` — merge of Download + operator recruitment (reuses `InstallCommand`, `OsDetector`)
5. Rewrite `website/src/app/about/page.tsx` — new mission, values, timeline, open source sections
6. Create supporting components:
    - `website/src/components/initiative-card.tsx`
    - `website/src/components/values-grid.tsx`
    - `website/src/components/network-stats.tsx`
7. Update `website/src/app/download/page.tsx` — add redirect banner pointing to `/operators`
8. Update `website/src/components/timeline.tsx` — extend events through 2026 H2

**Files created:** 4 new pages + 3 new components
**Files modified:** `about/page.tsx`, `download/page.tsx`, `timeline.tsx`

**Acceptance criteria:**
- [ ] `/platform` renders with architecture diagram, AI engine, pipeline, tech stack sections
- [ ] `/research` shows Prime Discovery as active initiative + 2-3 grayed-out coming-soon cards
- [ ] `/research/primes` contains all relocated prime content (forms, pipeline, live feed, comparison)
- [ ] `/operators` has install instructions with OS auto-detection, system requirements, network stats
- [ ] `/about` has mission, timeline, values, open source sections
- [ ] `/download` shows redirect banner to `/operators`
- [ ] All pages use `PageHero` component for consistent hero sections
- [ ] All pages include waitlist CTA (inline or modal trigger)
- [ ] `npm run build` succeeds

**Dependencies:** Phase 1 (navbar, footer, PageHero, WaitlistForm, design system)
**Complexity:** Large — 4 new pages with significant content

---

### Phase 3: Blog System

**Status: TODO**

**Goal:** MDX blog with 3 launch posts, replacing hardcoded blog array.

**Scope:**

1. Add dependencies: `@next/mdx`, `@mdx-js/loader`, `@mdx-js/react` to `website/package.json`
2. Create `website/content/blog/` directory for MDX files
3. Create `website/src/app/blog/[slug]/page.tsx` — individual blog post page with `generateStaticParams`
4. Create `website/src/components/blog-post-layout.tsx` — MDX post renderer with metadata, reading time, back link
5. Update `website/src/app/blog/page.tsx` — read from MDX files instead of `blog-posts.ts`
6. Restyle `website/src/components/blog-card.tsx` — cleaner card design
7. Write 3 MDX launch posts:
    - `website/content/blog/announcing-darkreach.mdx`
    - `website/content/blog/prime-discovery-initiative.mdx`
    - `website/content/blog/why-open-source.mdx`
8. Update `website/next.config.ts` for MDX support

**Files created:** 1 page + 1 component + 3 MDX posts + `content/blog/` directory
**Files modified:** `blog/page.tsx`, `blog-card.tsx`, `package.json`, `next.config.ts`

**Acceptance criteria:**
- [ ] `/blog` shows 3 launch post cards with title, date, excerpt
- [ ] `/blog/announcing-darkreach` renders full MDX post with proper layout
- [ ] `/blog/prime-discovery-initiative` and `/blog/why-open-source` render correctly
- [ ] Blog posts have metadata (title, date, excerpt, author)
- [ ] Blog cards link to individual post pages
- [ ] Code blocks in MDX render with syntax highlighting
- [ ] `npm run build` succeeds (static params generated for all slugs)

**Dependencies:** Phase 1 (design system, navbar, footer)
**Complexity:** Medium — MDX setup is well-documented, most work is content writing

---

### Phase 4: Updates & Polish

**Status: TODO**

**Goal:** Terminology updates on existing pages, docs reorganization, animations, responsive QA, SEO.

**Scope:**

1. Update `website/src/app/status/page.tsx` — rename "Fleet Overview" → "Network Overview", "Active Workers" → "Active Nodes", add waitlist CTA
2. Update `website/src/app/leaderboard/page.tsx` — rename "Total Volunteers" → "Total Operators", add "Join the leaderboard" CTA
3. Update `website/src/lib/status-data.ts` — rename fleet terminology
4. Update `website/src/lib/leaderboard-data.ts` — rename volunteer terminology
5. Reorganize `website/src/lib/docs-nav.ts`:
   ```ts
   [
     { title: "Getting Started", items: ["Overview", "Quick Start", "Architecture"] },
     { title: "Platform", items: ["AI Engine", "Network & Operators", "Projects", "API Reference"] },
     { title: "Research", items: ["Prime Forms", "Verification"] },
     { title: "Community", items: ["Contributing"] },
   ]
   ```
6. Update `website/src/app/docs/page.tsx` — update intro text for new messaging
7. Add scroll-triggered animations across all pages using `ScrollAnimate` wrapper
8. Add count-up animation for ProofBar stats
9. Responsive QA pass at 375px, 768px, 1024px, 1440px — fix any issues
10. Add OG image + metadata for all pages (`metadata` export in each `page.tsx`)
11. Update `website/vercel.json` if any new subdomain rewrites needed

**Files modified:** `status/page.tsx`, `leaderboard/page.tsx`, `status-data.ts`, `leaderboard-data.ts`, `docs-nav.ts`, `docs/page.tsx`, multiple page files for metadata, `vercel.json`

**Acceptance criteria:**
- [ ] Status page says "Network Overview" and "Active Nodes" (no fleet/worker terminology)
- [ ] Leaderboard says "Total Operators" (no volunteer terminology)
- [ ] Docs sidebar shows reorganized 4-section navigation
- [ ] Scroll animations trigger smoothly on all pages
- [ ] Stats count up when ProofBar enters viewport
- [ ] No layout breaks at 375px, 768px, 1024px, 1440px
- [ ] All pages have OG metadata (title, description, image)
- [ ] `npm run build` succeeds
- [ ] `npm run lint` passes

**Dependencies:** Phases 1-3 (all pages must exist before polish pass)
**Complexity:** Medium — many small changes across multiple files

---

### Phase 5: Live Integration

**Status: TODO**

**Goal:** Connect waitlist form to backend, verify live data feeds, deploy.

**Scope:**

1. Wire waitlist form to backend — Supabase edge function or Loops.so/Resend integration
2. Verify ProofBar connects to `api.darkreach.ai/api/stats` with new generic labels
3. Verify Status page live probing still works at `status.darkreach.ai`
4. Verify LiveFeed on `/research/primes` connects to real-time prime data
5. Verify NetworkStats on `/operators` shows live node data
6. Test full deploy to Vercel — all routes resolve, subdomains work
7. Lighthouse audit — target 95+ on all metrics
8. Cross-browser test (Chrome, Firefox, Safari)

**Files modified:** `waitlist.ts` (backend wiring), potentially `proof-bar.tsx` and `network-stats.tsx` for API adjustments

**Acceptance criteria:**
- [ ] Waitlist form submits and stores email in backend
- [ ] ProofBar shows live stats from API (with graceful fallback to static values)
- [ ] Status page shows real service health
- [ ] LiveFeed shows real-time prime discoveries
- [ ] All routes resolve on Vercel (including subdomain rewrites)
- [ ] Lighthouse scores: Performance 95+, Accessibility 95+, Best Practices 95+, SEO 95+
- [ ] Works in Chrome, Firefox, Safari (latest versions)

**Dependencies:** Phase 4 (all pages finalized and polished)
**Complexity:** Small — mostly integration testing and backend wiring

---

## File Map

### Files to CREATE

```
website/src/app/platform/page.tsx           # Phase 2
website/src/app/research/page.tsx           # Phase 2
website/src/app/research/primes/page.tsx    # Phase 2
website/src/app/operators/page.tsx          # Phase 2
website/src/app/blog/[slug]/page.tsx        # Phase 3

website/src/components/waitlist-form.tsx     # Phase 1
website/src/components/waitlist-modal.tsx    # Phase 1
website/src/components/proof-bar.tsx         # Phase 1
website/src/components/vision-cards.tsx      # Phase 1
website/src/components/how-it-works.tsx      # Phase 1
website/src/components/initiative-preview.tsx # Phase 1
website/src/components/initiative-card.tsx   # Phase 2
website/src/components/operator-recruit.tsx  # Phase 1
website/src/components/social-proof.tsx      # Phase 1
website/src/components/page-hero.tsx         # Phase 1
website/src/components/values-grid.tsx       # Phase 2
website/src/components/network-stats.tsx     # Phase 2
website/src/components/blog-post-layout.tsx  # Phase 3
website/src/components/scroll-animate.tsx    # Phase 1

website/src/lib/waitlist.ts                 # Phase 1

website/content/blog/announcing-darkreach.mdx           # Phase 3
website/content/blog/prime-discovery-initiative.mdx     # Phase 3
website/content/blog/why-open-source.mdx                # Phase 3
```

### Files to MODIFY (heavily)

```
website/src/app/globals.css                # Phase 1 — new color palette
website/src/app/layout.tsx                 # Phase 1 — Inter font, metadata
website/src/app/page.tsx                   # Phase 1 — complete homepage rewrite
website/src/app/about/page.tsx             # Phase 2 — full rewrite
website/src/app/blog/page.tsx              # Phase 3 — MDX source
website/src/components/navbar.tsx           # Phase 1 — new nav items, waitlist CTA
website/src/components/footer.tsx           # Phase 1 — new column structure
website/src/components/mobile-nav.tsx       # Phase 1 — new section structure
website/src/components/node-network.tsx     # Phase 1 — reduce density
website/package.json                       # Phase 3 — add @next/mdx deps
website/next.config.ts                     # Phase 3 — MDX support
```

### Files to MODIFY (lightly)

```
website/src/app/status/page.tsx            # Phase 4 — terminology
website/src/app/leaderboard/page.tsx       # Phase 4 — terminology
website/src/app/download/page.tsx          # Phase 2 — redirect banner
website/src/app/docs/page.tsx              # Phase 4 — intro text
website/src/components/timeline.tsx         # Phase 2 — extend events
website/src/components/blog-card.tsx        # Phase 3 — restyle
website/src/lib/docs-nav.ts               # Phase 4 — reorganize sections
website/src/lib/status-data.ts             # Phase 4 — terminology
website/src/lib/leaderboard-data.ts        # Phase 4 — terminology
website/vercel.json                        # Phase 4 — rewrites if needed
```

### Files to KEEP AS-IS

```
website/src/components/darkreach-logo.tsx
website/src/components/hero-logo.tsx
website/src/components/status-card.tsx
website/src/components/uptime-bar.tsx
website/src/components/install-command.tsx
website/src/components/os-detector.tsx
website/src/components/doc-sidebar.tsx
website/src/components/ui/*               # All UI primitives
website/src/lib/cn.ts
website/src/lib/install-commands.ts
website/src/lib/prime-forms.ts
website/tsconfig.json
website/postcss.config.mjs
website/public/favicon.svg
```

---

## Implementation Priority

| Phase | Effort | Impact | Dependencies |
|-------|--------|--------|-------------|
| 1. Foundation | Large | Very High | None |
| 2. New Pages | Large | High | Phase 1 |
| 3. Blog System | Medium | Medium | Phase 1 |
| 4. Updates & Polish | Medium | Medium | Phases 1-3 |
| 5. Live Integration | Small | High | Phase 4 |

**Recommended order:** Phase 1 → Phase 2 + Phase 3 (parallel) → Phase 4 → Phase 5

Phases 2 and 3 can be worked on in parallel since they share only the Phase 1 foundation (design system, navbar, footer, `PageHero`, `WaitlistForm`) and don't modify the same files.
