"use client";

import { CodeBlock } from "@/components/ui/code-block";
import { Badge } from "@/components/ui/badge";

export default function AiEnginePage() {
  return (
    <div className="prose-docs">
      <h1>AI Engine</h1>
      <p>
        The AI engine is an autonomous decision system that manages search
        strategy, resource allocation, and campaign orchestration. It replaces
        manual tuning with a unified <strong>OODA decision loop</strong> that
        continuously adapts to fleet state, discovery patterns, and cost
        constraints.
      </p>

      <h2>OODA Decision Loop</h2>
      <p>
        Every 30 seconds the engine executes a full Observe &rarr; Orient &rarr;
        Decide &rarr; Act cycle:
      </p>
      <CodeBlock>
        {`┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│ Observe  │────▶│  Orient  │────▶│  Decide  │────▶│   Act    │
│          │     │          │     │          │     │          │
│ Snapshot │     │  Score   │     │  Select  │     │ Execute  │
│ fleet,   │     │  forms,  │     │  best    │     │ start/   │
│ costs,   │     │  weight  │     │  action  │     │ stop/    │
│ records  │     │  drift   │     │  plan    │     │ reconfig │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
      │                                                   │
      └───────────────── Learn ◀──────────────────────────┘`}
      </CodeBlock>

      <h3>Observe: WorldSnapshot</h3>
      <p>
        A single consistent view of the entire system assembled via parallel
        database queries in ~50ms:
      </p>
      <ul>
        <li>Active workers, their capabilities, and current assignments</li>
        <li>Running searches with progress, rate, and stall detection</li>
        <li>Recent discoveries and their forms</li>
        <li>Budget velocity and remaining compute budget</li>
        <li>World record standings per form (scraped from t5k.org)</li>
        <li>Cost model coefficients from calibration data</li>
      </ul>

      <h3>Orient: Scoring Model</h3>
      <p>
        Each candidate search form is scored using a 7-component weighted model:
      </p>
      <table>
        <thead>
          <tr>
            <th>Component</th>
            <th>Weight</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><code>record_gap</code></td>
            <td>Dynamic</td>
            <td>Distance to current world record — closer means higher payoff</td>
          </tr>
          <tr>
            <td><code>yield_rate</code></td>
            <td>Dynamic</td>
            <td>Historical primes-per-core-hour for this form</td>
          </tr>
          <tr>
            <td><code>cost_efficiency</code></td>
            <td>Dynamic</td>
            <td>Expected cost per discovery using the power-law cost model</td>
          </tr>
          <tr>
            <td><code>opportunity_density</code></td>
            <td>Dynamic</td>
            <td>Untested candidate density in the target range</td>
          </tr>
          <tr>
            <td><code>fleet_fit</code></td>
            <td>Dynamic</td>
            <td>How well the form matches available worker hardware</td>
          </tr>
          <tr>
            <td><code>momentum</code></td>
            <td>Dynamic</td>
            <td>Recent discovery trend — reward hot streaks</td>
          </tr>
          <tr>
            <td><code>competition</code></td>
            <td>Dynamic</td>
            <td>External search activity on competing platforms</td>
          </tr>
        </tbody>
      </table>
      <p>
        Weights are learned via <strong>online gradient descent</strong>,
        comparing predicted outcomes against actual discovery data. The learned
        weights are persisted in the <code>ai_engine_state</code> database
        table.
      </p>

      <h3>Decide &amp; Act</h3>
      <p>
        The decision phase selects from a set of possible actions:
      </p>
      <ul>
        <li><strong>Start search</strong> — Launch a new search on the highest-scored form</li>
        <li><strong>Stop search</strong> — Terminate a stalled or low-yield search</li>
        <li><strong>Reconfigure</strong> — Adjust sieve depth, worker count, or range parameters</li>
        <li><strong>Scale</strong> — Request more workers or release idle ones</li>
        <li><strong>Hold</strong> — No action needed (system is performing well)</li>
      </ul>

      <h3>Learn: Outcome Tracking</h3>
      <p>
        Every decision is recorded in the <code>ai_engine_decisions</code> table
        with reasoning text, confidence score, and eventual outcome. This audit
        trail enables:
      </p>
      <ul>
        <li>Weight updates via gradient descent on prediction error</li>
        <li>Post-hoc analysis of strategy effectiveness</li>
        <li>Debugging poor decisions with full context replay</li>
      </ul>

      <h2>Cost Model</h2>
      <p>
        The cost model predicts compute time for a work block using a{" "}
        <strong>power-law regression</strong> fitted to historical data:
      </p>
      <CodeBlock language="text">
        {`cost(digits) = a * digits^b

Where:
  a, b  = OLS-fitted coefficients on log-log work block data
  digits = candidate digit count

Fallback defaults (when insufficient data):
  factorial:    a=1e-6,  b=2.5
  kbn:          a=1e-7,  b=2.0
  palindromic:  a=1e-5,  b=2.2
  ...per form`}
      </CodeBlock>
      <p>
        Coefficients are recalibrated automatically as new work block
        completions arrive. The model is stored in
        the <code>calibrations</code> database table.
      </p>

      <h2>Drift Detection</h2>
      <p>
        The engine compares consecutive WorldSnapshots to detect significant
        changes that require immediate attention:
      </p>
      <ul>
        <li><Badge variant="green">Worker change</Badge> — Workers joining or leaving the fleet</li>
        <li><Badge variant="purple">Discovery</Badge> — New prime found, potentially shifting strategy</li>
        <li><Badge variant="orange">Stall</Badge> — Search making no progress for extended period</li>
        <li><Badge variant="red">Budget alert</Badge> — Spend rate exceeding budget velocity target</li>
      </ul>

      <h2>Safety Checks</h2>
      <p>
        Before any action is executed, safety gates are evaluated:
      </p>
      <ul>
        <li><strong>Budget gate</strong> — Cannot start new searches if remaining budget is below threshold</li>
        <li><strong>Concurrency limit</strong> — Maximum simultaneous searches per form</li>
        <li><strong>Stall penalty</strong> — Penalize forms that have recently stalled</li>
        <li><strong>Cooldown</strong> — Minimum interval between actions to prevent thrashing</li>
      </ul>

      <h2>Dashboard Integration</h2>
      <p>
        The AI engine state is visible in the dashboard at{" "}
        <a href="https://app.darkreach.ai/strategy">app.darkreach.ai/strategy</a>:
      </p>
      <ul>
        <li>Current scoring weights and form rankings</li>
        <li>Decision history with reasoning and outcomes</li>
        <li>Cost model curves per form</li>
        <li>Drift event timeline</li>
      </ul>

      <h2>Configuration</h2>
      <p>
        The AI engine runs automatically when the coordinator starts with a
        database connection. Key configuration is via environment variables and
        the <code>ai_engine_state</code> table:
      </p>
      <CodeBlock language="bash">
        {`# Tick interval (default: 30s)
AI_ENGINE_TICK_INTERVAL=30

# Budget limit (USD per day)
AI_ENGINE_DAILY_BUDGET=50.0

# Maximum concurrent searches
AI_ENGINE_MAX_CONCURRENT=8`}
      </CodeBlock>
    </div>
  );
}
