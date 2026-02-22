"use client";

import { CodeBlock } from "@/components/ui/code-block";
import { Badge } from "@/components/ui/badge";

export default function ProjectsPage() {
  return (
    <div className="prose-docs">
      <h1>Projects &amp; Campaigns</h1>
      <p>
        A <strong>project</strong> is a multi-phase campaign to discover primes
        in a specific form and range. Projects coordinate resources, track costs,
        and manage the lifecycle from initial scouting through record-breaking
        verification.
      </p>

      <h2>Project Lifecycle</h2>
      <CodeBlock>
        {`┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Scout   │────▶│  Search  │────▶│  Verify  │────▶│ Complete │
│          │     │          │     │          │     │          │
│ Quick    │     │ Full     │     │ Re-verify│     │ Archive  │
│ sweep to │     │ range    │     │ all PRPs,│     │ results, │
│ estimate │     │ search   │     │ generate │     │ submit   │
│ density  │     │ with AI  │     │ certs    │     │ records  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘`}
      </CodeBlock>

      <h3>Phase states</h3>
      <table>
        <thead>
          <tr>
            <th>Phase</th>
            <th>Purpose</th>
            <th>AI Engine role</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><Badge variant="purple">Scout</Badge></td>
            <td>Quick sweep of a small range to estimate candidate density and cost</td>
            <td>Calibrate cost model, set expectations</td>
          </tr>
          <tr>
            <td><Badge variant="green">Search</Badge></td>
            <td>Full-range search with allocated workers. Main compute phase</td>
            <td>Dynamic resource allocation, stall detection</td>
          </tr>
          <tr>
            <td><Badge variant="orange">Verify</Badge></td>
            <td>Re-verify all probable primes, generate deterministic certificates where possible</td>
            <td>Prioritize high-value candidates for re-verification</td>
          </tr>
          <tr>
            <td><Badge>Complete</Badge></td>
            <td>Archive results, submit records to databases, write summary</td>
            <td>Report generation, strategy learning</td>
          </tr>
        </tbody>
      </table>

      <h2>Configuration</h2>
      <p>
        Projects are defined via TOML configuration or the dashboard:
      </p>
      <CodeBlock language="toml">
        {`[project]
name = "Factorial 200K+"
description = "Search for factorial primes above 200,000"
form = "factorial"

[target]
start = 200000
end = 300000
goal = "world_record"    # or "coverage", "count"

[budget]
max_compute_hours = 5000
max_cost_usd = 250.0
daily_limit_usd = 25.0

[resources]
min_workers = 4
max_workers = 16
priority = "high"        # high, normal, low

[phases]
scout_range = [200000, 201000]  # Quick density estimate
verify_all = true               # Re-verify all PRPs`}
      </CodeBlock>

      <h2>Cost Estimation</h2>
      <p>
        Before committing resources, the project system estimates total cost
        using the AI engine&apos;s power-law cost model:
      </p>
      <CodeBlock language="text">
        {`Project: Factorial 200K+
Form:    factorial
Range:   200,000 → 300,000

Estimated candidates:     ~8,500
Sieve survival rate:      ~12%
Candidates to test:       ~1,020
Avg test time:            45 min/candidate
Total compute:            ~765 core-hours
Estimated cost:           $38.25 (at $0.05/core-hour)
Expected primes:          0-2 (based on heuristic density)`}
      </CodeBlock>

      <h2>World Record Tracking</h2>
      <p>
        Projects targeting world records automatically track the current
        standings by scraping{" "}
        <a href="https://t5k.org" target="_blank" rel="noopener noreferrer">
          t5k.org (The Prime Pages)
        </a>:
      </p>
      <ul>
        <li>Current record holder for each form</li>
        <li>Record digit count and discovery date</li>
        <li>Gap analysis — how far the project&apos;s best result is from the record</li>
        <li>Leaderboard position tracking over time</li>
      </ul>

      <h2>Orchestration</h2>
      <p>
        The project orchestrator runs a 30-second tick loop that manages phase
        transitions and resource allocation:
      </p>
      <ul>
        <li><strong>Auto-advance</strong> — Moves from Scout to Search when density estimate is confident</li>
        <li><strong>Auto-verify</strong> — Triggers Verify phase when Search reaches target coverage</li>
        <li><strong>Budget enforcement</strong> — Pauses searches if daily spend limit is reached</li>
        <li><strong>Worker scaling</strong> — Requests more workers from the AI engine when needed</li>
        <li><strong>Event logging</strong> — All phase transitions and decisions are recorded</li>
      </ul>

      <h2>Dashboard</h2>
      <p>
        Manage projects at{" "}
        <a href="https://app.darkreach.ai/projects">
          app.darkreach.ai/projects
        </a>:
      </p>
      <ul>
        <li>Create and configure new projects</li>
        <li>Monitor phase progress with timeline visualization</li>
        <li>View cost breakdown and budget burn rate</li>
        <li>See discovered primes and their verification status</li>
        <li>Compare against world records in real time</li>
      </ul>

      <h2>API Endpoints</h2>
      <table>
        <thead>
          <tr>
            <th>Method</th>
            <th>Path</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><Badge variant="green">GET</Badge></td>
            <td><code>/api/projects</code></td>
            <td>List all projects with status</td>
          </tr>
          <tr>
            <td><Badge variant="purple">POST</Badge></td>
            <td><code>/api/projects</code></td>
            <td>Create a new project</td>
          </tr>
          <tr>
            <td><Badge variant="green">GET</Badge></td>
            <td><code>/api/projects/:id</code></td>
            <td>Get project details with phases and events</td>
          </tr>
          <tr>
            <td><Badge variant="purple">PUT</Badge></td>
            <td><code>/api/projects/:id</code></td>
            <td>Update project configuration</td>
          </tr>
          <tr>
            <td><Badge variant="purple">POST</Badge></td>
            <td><code>/api/projects/:id/advance</code></td>
            <td>Manually advance to next phase</td>
          </tr>
        </tbody>
      </table>

      <h2>Example: Creating a Campaign</h2>
      <CodeBlock language="bash">
        {`# Create a new factorial prime project via the API
curl -X POST https://api.darkreach.ai/api/projects \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "Factorial 200K+",
    "form": "factorial",
    "config": {
      "target": { "start": 200000, "end": 300000 },
      "budget": { "max_cost_usd": 250.0 },
      "resources": { "min_workers": 4 }
    }
  }'`}
      </CodeBlock>
    </div>
  );
}
