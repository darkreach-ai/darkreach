# Website Roadmap

## Current State (February 2026)

The darkreach website (`website/`) is a multi-route Next.js 16 static export deployed to Vercel. It serves as the public face of the project across three domains:

- **darkreach.ai** — Landing page, about, blog, download, leaderboard, operators, platform, research
- **docs.darkreach.ai** — Documentation hub (9 pages + sidebar nav)
- **status.darkreach.ai** — Service health and network overview

### What's Built (23 pages)

| Route | Status | Description |
|-------|--------|-------------|
| `/` | Complete | Homepage with AI narrative, stats, feature grid, pipeline, 12 prime forms, discoveries, comparison, CTA |
| `/about` | Complete | Mission, timeline, tech stack, open source callout |
| `/blog` | Complete | Blog index with 5 mocked posts |
| `/blog/[slug]` | Complete | Individual blog post pages (MDX-ready dynamic route) |
| `/download` | Complete | OS detection, tabbed install methods, system requirements |
| `/download/server` | Complete | Dashboard/central service setup guide with systemd config |
| `/download/worker` | Complete | Node deployment guide with scaling instructions |
| `/operators` | Complete | Operator onboarding, registration flow, contribution overview |
| `/platform` | Complete | Platform overview, compute marketplace positioning |
| `/research` | Complete | Research overview, discovery strategy, publications |
| `/research/primes` | Complete | Prime discovery results and records |
| `/docs` | Complete | Quick-links grid hub |
| `/docs/getting-started` | Complete | Prerequisites -> build -> first search -> checkpointing |
| `/docs/architecture` | Complete | System diagram, engine/server/frontend breakdown |
| `/docs/prime-forms` | Complete | All 12 forms with OEIS refs, algorithms, CLI commands |
| `/docs/api` | Complete | REST endpoints + WebSocket events (mocked reference) |
| `/docs/contributing` | Complete | Fork/PR workflow, code style, adding new forms |
| `/docs/ai-engine` | Complete | AI engine OODA loop, ML subsystems, strategy documentation |
| `/docs/verification` | Complete | 3-tier verification pipeline, certificate types |
| `/docs/network` | Complete | Network architecture, node coordination, operator model |
| `/docs/projects` | Complete | Project lifecycle, cost tracking, campaign management |
| `/status` | Complete | Service cards, network stats, 90-day uptime bars, incidents |
| `/leaderboard` | Complete | Individual + team rankings (mocked data) |

### Architecture

- **Framework**: Next.js 16 with `output: "export"` (pure static)
- **Styling**: Tailwind CSS 4 with inline `@theme` custom properties
- **Icons**: lucide-react
- **Utilities**: clsx for class composition
- **Subdomain routing**: Vercel `rewrites` in `vercel.json` with host-based matching
- **No backend integration**: All data is hardcoded/mocked

### Design System

CSS custom properties aligned with the dashboard (`frontend/`):
- Colors: GitHub-style dark mode (#0d1117 bg, #161b22 secondary, #30363d borders)
- Brand purple: #bc8cff (logo, accents)
- Primary blue: #2f81f7 (links, interactive)
- System fonts: -apple-system stack (matches dashboard)

### Component Library (43 components)

Lightweight UI primitives (no shadcn -- minimal deps):
- `ui/button.tsx` -- Primary, outline, ghost variants
- `ui/card.tsx` -- Standard card with optional hover glow
- `ui/badge.tsx` -- Status tags with color variants
- `ui/code-block.tsx` -- Syntax display with copy button
- `ui/section.tsx` -- Consistent section wrapper

Section and page components:
- `hero.tsx`, `hero-logo.tsx` -- AI narrative hero with animated logo
- `feature-grid.tsx` -- 3-col AI capabilities grid
- `pipeline.tsx` -- 4-step pipeline visualization
- `cta-section.tsx` -- Two-path CTA (operator/dashboard)
- `prime-forms.tsx` -- 12 prime form showcase
- `discoveries.tsx` -- Recent discovery highlights
- `comparison.tsx` -- Platform comparison table
- `mission.tsx` -- Mission statement section
- `get-started.tsx` -- Getting started guide
- `how-it-works.tsx` -- How it works explainer
- `vision-cards.tsx` -- Technology vision cards
- `social-proof.tsx` -- Social proof section
- `network-stats.tsx` -- Network statistics display
- `proof-bar.tsx` -- Proof status bar
- `live-feed.tsx` -- Live prime discovery feed
- `node-network.tsx` -- Network topology visualization
- `operator-recruit.tsx` -- Operator recruitment section
- `waitlist-form.tsx`, `waitlist-cta.tsx` -- Waitlist signup forms
- `scroll-animate.tsx` -- Scroll-triggered animations
- `page-hero.tsx` -- Reusable page hero section
- `initiative-card.tsx`, `initiative-preview.tsx` -- Research initiative cards
- `values-grid.tsx` -- Values grid layout
- `discovery-toast.tsx` -- Discovery notification toast
- `blog-card.tsx`, `blog-post-layout.tsx` -- Blog components
- `darkreach-logo.tsx` -- Brand logo component
- `install-command.tsx`, `os-detector.tsx` -- Download page components
- `navbar.tsx`, `mobile-nav.tsx`, `footer.tsx` -- Navigation and layout
- `doc-sidebar.tsx` -- Documentation sidebar navigation
- `status-card.tsx`, `uptime-bar.tsx` -- Status page components
- `timeline.tsx` -- Vertical timeline

---

## Future Phases

### Phase A: Live Data Integration & Operator Onboarding -- PARTIAL
- ~~Add operator onboarding pages: registration flow, node setup guide, contribution dashboard~~ **DONE** -- `/operators` page with onboarding and recruitment (`operator-recruit.tsx`, `waitlist-form.tsx`, `waitlist-cta.tsx`)
- Connect stats bar to `api.darkreach.ai/api/stats` for real-time numbers
- Connect status page to `api.darkreach.ai/api/status` for live health checks
- Graceful fallback to static values when API is unreachable
- Connect leaderboard to real contributor data from PostgreSQL

### Phase B: MDX Blog -- PARTIAL
- Replace mocked blog cards with MDX-based content
- ~~Individual post pages (`/blog/[slug]`)~~ **DONE** -- Dynamic route with `blog-post-layout.tsx`
- Syntax highlighting with shiki or rehype-pretty-code
- RSS feed generation
- Author profiles

### Phase C: Documentation Enhancements
- Full-text search across docs (Pagefind or Algolia DocSearch)
- Table of contents sidebar for long pages
- Previous/Next navigation between doc pages
- Code copy buttons on all code blocks in prose
- Version selector (if multiple darkreach versions are maintained)

### Phase D: Interactive Features
- Dark/light mode toggle (currently dark-only, matching dashboard)
- Newsletter signup (email collection)
- Discord widget integration
- Real-time prime notification toast on homepage
- Interactive prime form explorer (try parameters, see sample output)

### Phase E: Performance & SEO
- Lighthouse audit and optimization (target 95+ on all metrics)
- Structured data (JSON-LD) for search engines
- Sitemap generation
- Image optimization (OG images for social sharing)
- Analytics integration (Plausible or self-hosted)

### Phase F: Internationalization
- i18n support for docs (priority: English, Chinese, Japanese)
- Locale-aware routing
- Translation workflow with community contributions

---

## Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Component library | Custom UI primitives | Keeps deps minimal; shadcn would add 10+ deps for 5 components |
| Subdomain routing | Vercel rewrites (host-based) | Single project, single deploy, no server needed |
| Content management | Hardcoded TSX | Simple for v1; migrate to MDX when blog grows |
| Styling | Tailwind CSS 4 + CSS custom properties | Matches dashboard, utility-first, no runtime cost |
| Icons | lucide-react | Same library as dashboard, tree-shakeable |
| State management | None (static export) | No client state beyond OS detection and mobile nav toggle |
| Data fetching | None (mocked) | Will add client-side fetch with SWR/React Query for live data |

---

## File Structure

```
website/
├── src/
│   ├── app/
│   │   ├── layout.tsx              # Shared navbar + footer
│   │   ├── page.tsx                # Homepage
│   │   ├── globals.css             # Design tokens
│   │   ├── about/page.tsx
│   │   ├── blog/
│   │   │   ├── page.tsx            # Blog index
│   │   │   └── [slug]/page.tsx     # Dynamic blog post route
│   │   ├── download/
│   │   │   ├── page.tsx            # Main download page
│   │   │   ├── server/page.tsx     # Dashboard/central service guide
│   │   │   └── worker/page.tsx     # Node deployment guide
│   │   ├── operators/page.tsx      # Operator onboarding + recruitment
│   │   ├── platform/page.tsx       # Platform overview + marketplace
│   │   ├── research/
│   │   │   ├── page.tsx            # Research overview
│   │   │   └── primes/page.tsx     # Prime discovery results
│   │   ├── docs/
│   │   │   ├── layout.tsx          # Sidebar layout
│   │   │   ├── page.tsx            # Docs hub
│   │   │   ├── getting-started/
│   │   │   ├── architecture/
│   │   │   ├── prime-forms/
│   │   │   ├── api/
│   │   │   ├── contributing/
│   │   │   ├── ai-engine/          # AI engine documentation
│   │   │   ├── verification/       # Verification pipeline docs
│   │   │   ├── network/            # Network architecture docs
│   │   │   └── projects/           # Project lifecycle docs
│   │   ├── status/page.tsx
│   │   └── leaderboard/page.tsx
│   ├── components/                 # 43 components
│   │   ├── ui/                     # Primitives (button, card, badge, code-block, section)
│   │   ├── navbar.tsx, mobile-nav.tsx, footer.tsx  # Navigation
│   │   ├── hero.tsx, hero-logo.tsx                 # Hero section
│   │   ├── feature-grid.tsx, pipeline.tsx          # Feature sections
│   │   ├── cta-section.tsx, get-started.tsx         # Call-to-action
│   │   ├── prime-forms.tsx, discoveries.tsx         # Prime showcase
│   │   ├── comparison.tsx, mission.tsx              # About sections
│   │   ├── how-it-works.tsx, vision-cards.tsx       # Explainers
│   │   ├── operator-recruit.tsx                     # Operator recruitment
│   │   ├── waitlist-form.tsx, waitlist-cta.tsx       # Waitlist
│   │   ├── social-proof.tsx, network-stats.tsx       # Social proof
│   │   ├── live-feed.tsx, discovery-toast.tsx        # Live data
│   │   ├── node-network.tsx, proof-bar.tsx           # Network viz
│   │   ├── scroll-animate.tsx, page-hero.tsx         # Utilities
│   │   ├── initiative-card.tsx, initiative-preview.tsx # Research
│   │   ├── values-grid.tsx                           # Values
│   │   ├── blog-card.tsx, blog-post-layout.tsx       # Blog
│   │   ├── darkreach-logo.tsx                        # Brand
│   │   ├── install-command.tsx, os-detector.tsx       # Download
│   │   ├── doc-sidebar.tsx                           # Docs sidebar
│   │   ├── status-card.tsx, uptime-bar.tsx           # Status
│   │   └── timeline.tsx                              # Timeline
│   └── lib/
│       ├── cn.ts                   # clsx helper
│       ├── prime-forms.ts          # 12 form definitions
│       ├── docs-nav.ts             # Sidebar navigation structure
│       ├── install-commands.ts     # OS-specific install commands
│       ├── status-data.ts          # Mock status/network data
│       ├── leaderboard-data.ts     # Mock contributor rankings
│       └── blog-posts.ts           # Mock blog posts
├── vercel.json                     # Subdomain rewrites (docs.darkreach.ai, status.darkreach.ai)
├── next.config.ts                  # Static export config
└── package.json
```
