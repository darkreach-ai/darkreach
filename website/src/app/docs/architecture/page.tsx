"use client";

import { CodeBlock } from "@/components/ui/code-block";

export default function ArchitecturePage() {
  return (
    <div className="prose-docs">
      <h1>Architecture</h1>
      <p>
        darkreach is a five-layer system: a high-performance Rust engine for
        number theory computation, an AI engine for autonomous strategy, an Axum
        server for coordination, a PostgreSQL database for persistence, and
        two Next.js frontends for public and operational interfaces.
      </p>

      <h2>System Overview</h2>
      <CodeBlock>
        {`┌─────────────────────────────────────────────────────────┐
│                    darkreach.ai                          │
│               (Next.js static export)                   │
│        Landing · Docs · Status · Blog · Leaderboard      │
└─────────────────────────────────────────────────────────┘

┌────────────────────┐      ┌────────────────────────────────┐
│  app.darkreach.ai  │─────▶│      api.darkreach.ai          │
│  (Next.js + SPA)   │ WS   │    (Axum web server)           │
│                    │◀─────│                                │
│  Browse · Searches │      │  REST API (15 route modules)   │
│  Network · Agents  │      │  WebSocket (2s push)           │
│  Projects · Logs   │      │  AI Engine (OODA loop)         │
│  Strategy · Verify │      │  Project Orchestrator          │
└────────────────────┘      └───────────┬────────────────────┘
                                        │
                             ┌──────────▼─────────────────┐
                             │       PostgreSQL            │
                             │   (Supabase / self-hosted)  │
                             │                             │
                             │  primes · workers · jobs    │
                             │  projects · agents · ai     │
                             │  operators · releases       │
                             │  24 migrations              │
                             └──────────┬─────────────────┘
                                        │
           ┌────────────────────────────┼─────────────────────────┐
           │                            │                         │
  ┌────────▼────────┐      ┌────────────▼──────┐     ┌───────────▼────────┐
  │  Operator Alice  │      │  Operator Bob     │     │  Operator Charlie  │
  │  ┌─────┐┌─────┐ │      │  ┌─────┐┌─────┐  │     │  ┌─────┐           │
  │  │Node1││Node2│ │      │  │Node1││Node2│  │     │  │Node1│           │
  │  │8cor ││8cor │ │      │  │16cor││4cor │  │     │  │32cor│           │
  │  └─────┘└─────┘ │      │  └─────┘└─────┘  │     │  └─────┘           │
  └──────────────────┘      └──────────────────┘     └────────────────────┘`}
      </CodeBlock>

      <h2>Engine</h2>
      <p>
        The engine is the core Rust library implementing{" "}
        <strong>12 prime search algorithms</strong>. Each form follows the same
        pipeline:
      </p>
      <ol>
        <li>
          <strong>Sieve</strong> — Eliminate composites using form-specific
          sieves (wheel factorization, BSGS, Montgomery multiplication)
        </li>
        <li>
          <strong>Filter</strong> — Deep composite elimination via adaptive
          Pollard P-1 (Stage 1 + Stage 2)
        </li>
        <li>
          <strong>Test</strong> — Miller-Rabin pre-screening (25 rounds), then
          specialized tests (Proth, LLR, Pepin). Frobenius RQFT for &gt;10K-bit
          candidates
        </li>
        <li>
          <strong>Prove</strong> — Generate deterministic primality certificates
          (Pocklington, Morrison, BLS, Proth, LLR)
        </li>
        <li>
          <strong>Report</strong> — Log results to PostgreSQL with certificate
          data, update project progress
        </li>
      </ol>

      <h3>Key engine modules</h3>
      <table>
        <thead>
          <tr>
            <th>Module</th>
            <th>Purpose</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><code>src/sieve.rs</code></td>
            <td>Eratosthenes sieve, Montgomery multiplication, BitSieve (packed u64), wheel factorization, BSGS</td>
          </tr>
          <tr>
            <td><code>src/lib.rs</code></td>
            <td>Trial division, MR pre-screening, Frobenius RQFT, small primes table</td>
          </tr>
          <tr>
            <td><code>src/proof.rs</code></td>
            <td>Pocklington (N-1), Morrison (N+1), BLS deterministic proofs</td>
          </tr>
          <tr>
            <td><code>src/verify.rs</code></td>
            <td>3-tier verification pipeline (deterministic &rarr; BPSW+MR &rarr; PFGW)</td>
          </tr>
          <tr>
            <td><code>src/p1.rs</code></td>
            <td>Pollard P-1 factoring with adaptive B1/B2 tuning</td>
          </tr>
          <tr>
            <td><code>src/certificate.rs</code></td>
            <td>PrimalityCertificate enum (7 variants) for witness serialization</td>
          </tr>
          <tr>
            <td><code>src/pfgw.rs</code></td>
            <td>PFGW subprocess integration (50-100x acceleration)</td>
          </tr>
          <tr>
            <td><code>src/prst.rs</code></td>
            <td>PRST subprocess for k*b^n forms</td>
          </tr>
        </tbody>
      </table>

      <h3>12 search form modules</h3>
      <table>
        <thead>
          <tr>
            <th>Module</th>
            <th>Form</th>
          </tr>
        </thead>
        <tbody>
          <tr><td><code>factorial.rs</code></td><td>n! &plusmn; 1</td></tr>
          <tr><td><code>primorial.rs</code></td><td>p# &plusmn; 1</td></tr>
          <tr><td><code>kbn.rs</code></td><td>k&middot;b^n &plusmn; 1 (Proth/Riesel)</td></tr>
          <tr><td><code>cullen_woodall.rs</code></td><td>n&middot;2^n &plusmn; 1</td></tr>
          <tr><td><code>gen_fermat.rs</code></td><td>b^(2^n) + 1</td></tr>
          <tr><td><code>wagstaff.rs</code></td><td>(2^p + 1) / 3</td></tr>
          <tr><td><code>carol_kynea.rs</code></td><td>(2^n &plusmn; 1)&sup2; &minus; 2</td></tr>
          <tr><td><code>twin.rs</code></td><td>Twin primes (p, p+2)</td></tr>
          <tr><td><code>sophie_germain.rs</code></td><td>Sophie Germain (p, 2p+1)</td></tr>
          <tr><td><code>palindromic.rs</code></td><td>Palindromic primes</td></tr>
          <tr><td><code>near_repdigit.rs</code></td><td>Near-repdigit palindromic</td></tr>
          <tr><td><code>repunit.rs</code></td><td>R(b,n) = (b^n &minus; 1) / (b &minus; 1)</td></tr>
        </tbody>
      </table>

      <h2>AI Engine</h2>
      <p>
        The <a href="/docs/ai-engine">AI engine</a> is an autonomous decision
        system that replaces manual tuning with a unified OODA (Observe &rarr;
        Orient &rarr; Decide &rarr; Act) loop running every 30 seconds:
      </p>
      <ul>
        <li><strong>WorldSnapshot</strong> — Consistent view of fleet, costs, records (~50ms via parallel DB queries)</li>
        <li><strong>7-component scoring</strong> — record_gap, yield_rate, cost_efficiency, opportunity_density, fleet_fit, momentum, competition</li>
        <li><strong>Online learning</strong> — Weights updated via gradient descent on actual outcomes</li>
        <li><strong>Power-law cost model</strong> — OLS-fitted coefficients for compute time prediction</li>
        <li><strong>Drift detection</strong> — Automatic response to fleet changes, stalls, and discoveries</li>
      </ul>

      <h2>Server</h2>
      <p>
        The server is a modular Axum web application with 15 route modules:
      </p>

      <h3>Dashboard modules (<code>src/dashboard/</code>)</h3>
      <table>
        <thead>
          <tr>
            <th>Module</th>
            <th>Path</th>
            <th>Purpose</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><code>routes_health.rs</code></td>
            <td>/api/health</td>
            <td>Health check, readiness probes</td>
          </tr>
          <tr>
            <td><code>routes_status.rs</code></td>
            <td>/api/status</td>
            <td>Coordinator status summary</td>
          </tr>
          <tr>
            <td><code>routes_workers.rs</code></td>
            <td>/api/workers</td>
            <td>Worker/node registration, heartbeat, pruning</td>
          </tr>
          <tr>
            <td><code>routes_operator.rs</code></td>
            <td>/api/v1/operators, /api/v1/nodes</td>
            <td>Operator accounts, node management</td>
          </tr>
          <tr>
            <td><code>routes_fleet.rs</code></td>
            <td>/api/fleet</td>
            <td>Fleet overview (all nodes + searches)</td>
          </tr>
          <tr>
            <td><code>routes_jobs.rs</code></td>
            <td>/api/search_jobs</td>
            <td>Job CRUD, work blocks, claiming</td>
          </tr>
          <tr>
            <td><code>routes_searches.rs</code></td>
            <td>/api/searches</td>
            <td>Search management</td>
          </tr>
          <tr>
            <td><code>routes_agents.rs</code></td>
            <td>/api/agents</td>
            <td>AI agent tasks, budgets, memory, schedules</td>
          </tr>
          <tr>
            <td><code>routes_projects.rs</code></td>
            <td>/api/projects</td>
            <td>Multi-phase project campaigns</td>
          </tr>
          <tr>
            <td><code>routes_verify.rs</code></td>
            <td>/api/verify</td>
            <td>Prime re-verification</td>
          </tr>
          <tr>
            <td><code>routes_docs.rs</code></td>
            <td>/api/docs</td>
            <td>Documentation search and serving</td>
          </tr>
          <tr>
            <td><code>routes_releases.rs</code></td>
            <td>/api/releases</td>
            <td>Worker release channels, canary rollout</td>
          </tr>
          <tr>
            <td><code>routes_observability.rs</code></td>
            <td>/api/observability</td>
            <td>Metrics, logs, performance charts</td>
          </tr>
          <tr>
            <td><code>routes_notifications.rs</code></td>
            <td>/api/notifications</td>
            <td>Push notification management</td>
          </tr>
          <tr>
            <td><code>websocket.rs</code></td>
            <td>/ws</td>
            <td>Real-time push (2s interval)</td>
          </tr>
        </tbody>
      </table>

      <h3>Database modules (<code>src/db/</code>)</h3>
      <p>
        PostgreSQL access is split into 13 domain-specific submodules:
      </p>
      <table>
        <thead>
          <tr>
            <th>Module</th>
            <th>Tables</th>
          </tr>
        </thead>
        <tbody>
          <tr><td><code>primes.rs</code></td><td>primes (insert, query, filter, verify)</td></tr>
          <tr><td><code>workers.rs</code></td><td>workers (heartbeat, registration, pruning)</td></tr>
          <tr><td><code>jobs.rs</code></td><td>search_jobs, work_blocks (lifecycle, claiming)</td></tr>
          <tr><td><code>agents.rs</code></td><td>agent_tasks, agent_events, agent_budgets</td></tr>
          <tr><td><code>memory.rs</code></td><td>agent_memory (key-value store)</td></tr>
          <tr><td><code>projects.rs</code></td><td>projects, project_phases, project_events</td></tr>
          <tr><td><code>operators.rs</code></td><td>operators, operator_nodes</td></tr>
          <tr><td><code>records.rs</code></td><td>world_records (t5k.org tracking)</td></tr>
          <tr><td><code>calibrations.rs</code></td><td>cost_calibrations (power-law coefficients)</td></tr>
          <tr><td><code>releases.rs</code></td><td>worker_releases, rollout_events</td></tr>
          <tr><td><code>observability.rs</code></td><td>metrics, worker_rates, logs</td></tr>
          <tr><td><code>roles.rs</code></td><td>agent_roles (configuration)</td></tr>
          <tr><td><code>schedules.rs</code></td><td>agent_schedules (automation)</td></tr>
        </tbody>
      </table>

      <h3>Project orchestration (<code>src/project/</code>)</h3>
      <p>
        Campaign management is a separate module with its own submodules:
      </p>
      <ul>
        <li><code>config.rs</code> — TOML configuration parsing and validation</li>
        <li><code>cost.rs</code> — Power-law cost estimation model</li>
        <li><code>orchestration.rs</code> — Phase state machine, 30s tick loop</li>
        <li><code>records.rs</code> — World record tracking via t5k.org scraping</li>
        <li><code>types.rs</code> — Database row types for projects, phases, events</li>
      </ul>

      <h2>Frontends</h2>
      <p>
        Two separate Next.js 16 applications:
      </p>
      <ul>
        <li>
          <strong>Website</strong> (<code>website/</code>) — Public-facing site
          at darkreach.ai. Landing page, documentation, blog, status, and
          leaderboard. Static export deployed to Vercel.
        </li>
        <li>
          <strong>Dashboard</strong> (<code>frontend/</code>) — Operational
          dashboard at app.darkreach.ai. 17 pages including prime browser,
          search management, network monitoring, AI agent control, project
          campaigns, performance metrics, and system logs. React 19 + Tailwind
          4 + shadcn/ui + Recharts. Supabase Auth for login. WebSocket for
          real-time coordination. PWA-enabled with offline support.
        </li>
      </ul>

      <h2>Data Flow</h2>
      <ol>
        <li>AI engine scores forms and decides which searches to run</li>
        <li>Coordinator generates work blocks and inserts them into PostgreSQL</li>
        <li>Nodes claim blocks using <code>FOR UPDATE SKIP LOCKED</code></li>
        <li>Nodes run the sieve &rarr; filter &rarr; test &rarr; prove pipeline</li>
        <li>Results (primes + certificates) are written back to PostgreSQL</li>
        <li>AI engine observes outcomes and updates scoring weights</li>
        <li>Project orchestrator tracks phase progress and manages budgets</li>
        <li>Dashboard queries PostgreSQL via Supabase for display</li>
        <li>WebSocket pushes real-time notifications for new primes and fleet status</li>
      </ol>

      <h2>Deployment</h2>
      <p>
        The production deployment runs on Hetzner Cloud:
      </p>
      <ul>
        <li><strong>Coordinator</strong> — CX22 instance running the Axum server, AI engine, and project orchestrator</li>
        <li><strong>Nodes</strong> — CCX23 instances (4+ cores) running darkreach in worker mode</li>
        <li><strong>Database</strong> — Supabase-hosted PostgreSQL (24 migrations)</li>
        <li><strong>Website</strong> — Vercel static deployment</li>
        <li><strong>Dashboard</strong> — Static export served by the coordinator</li>
        <li><strong>Monitoring</strong> — Prometheus metrics + Grafana dashboards</li>
      </ul>
      <p>
        See the <a href="/download/server">coordinator setup</a> and{" "}
        <a href="/download/worker">worker deployment</a> guides for detailed
        instructions.
      </p>
    </div>
  );
}
