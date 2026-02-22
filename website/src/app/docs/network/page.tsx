"use client";

import { CodeBlock } from "@/components/ui/code-block";
import { Badge } from "@/components/ui/badge";

export default function NetworkPage() {
  return (
    <div className="prose-docs">
      <h1>Network &amp; Operators</h1>
      <p>
        darkreach uses a distributed network of <strong>operators</strong> and{" "}
        <strong>nodes</strong> to parallelize prime searches across many
        machines. Operators manage one or more compute nodes, while the
        coordinator distributes work and aggregates results.
      </p>

      <h2>Architecture</h2>
      <CodeBlock>
        {`┌─────────────────────────────────────────────────┐
│              Coordinator (api.darkreach.ai)       │
│  Work generation · Block distribution · Results   │
└────────┬───────────────┬───────────────┬──────────┘
         │               │               │
    ┌────▼────┐    ┌─────▼─────┐   ┌─────▼─────┐
    │Operator │    │ Operator  │   │ Operator  │
    │  Alice  │    │   Bob     │   │  Charlie  │
    │ 4 nodes │    │  2 nodes  │   │  1 node   │
    └─────────┘    └───────────┘   └───────────┘`}
      </CodeBlock>

      <h2>Terminology</h2>
      <table>
        <thead>
          <tr>
            <th>Term</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><strong>Coordinator</strong></td>
            <td>Central server that generates work blocks, distributes them to nodes, and collects results</td>
          </tr>
          <tr>
            <td><strong>Operator</strong></td>
            <td>A person or organization that contributes compute resources. Has an account and manages nodes</td>
          </tr>
          <tr>
            <td><strong>Node</strong></td>
            <td>A single compute machine running the darkreach CLI in worker mode</td>
          </tr>
          <tr>
            <td><strong>Work block</strong></td>
            <td>A range of candidates assigned to a node for sieving and testing</td>
          </tr>
        </tbody>
      </table>
      <p>
        <em>
          Note: The codebase is migrating from &ldquo;volunteer/worker/fleet&rdquo;
          to &ldquo;operator/node/network&rdquo; terminology. Both are supported
          in the API.
        </em>
      </p>

      <h2>Joining the Network</h2>
      <h3>1. Register as an operator</h3>
      <p>
        Create an operator account via the dashboard or API:
      </p>
      <CodeBlock language="bash">
        {`curl -X POST https://api.darkreach.ai/api/v1/operators \\
  -H "Content-Type: application/json" \\
  -d '{"name": "alice", "contact_email": "alice@example.com"}'`}
      </CodeBlock>

      <h3>2. Configure your node</h3>
      <p>
        Create a configuration file at <code>~/.darkreach/config.toml</code>:
      </p>
      <CodeBlock language="toml">
        {`[node]
coordinator_url = "https://api.darkreach.ai"
operator_id = "your-operator-id"

[compute]
threads = 8          # Number of search threads (default: all cores)
memory_limit = "4G"  # Maximum memory for sieve buffers

[heartbeat]
interval = 10        # Heartbeat interval in seconds`}
      </CodeBlock>

      <h3>3. Start the worker</h3>
      <CodeBlock language="bash">
        {`# Connect to the coordinator and start claiming work
darkreach work --coordinator https://api.darkreach.ai

# Or with a direct database connection
darkreach work --database-url postgres://...`}
      </CodeBlock>
      <p>
        The node will register with the coordinator, begin receiving work
        blocks, and report results automatically.
      </p>

      <h2>Work Distribution</h2>
      <p>
        Work claiming uses PostgreSQL row-level locking for fairness and
        crash safety:
      </p>
      <CodeBlock language="sql">
        {`-- Each node claims one block at a time
SELECT * FROM work_blocks
WHERE status = 'pending'
ORDER BY priority DESC, created_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED`}
      </CodeBlock>
      <ul>
        <li><Badge variant="green">Fair</Badge> — <code>SKIP LOCKED</code> prevents contention between nodes</li>
        <li><Badge variant="green">Crash-safe</Badge> — Uncompleted blocks return to the pool on transaction rollback</li>
        <li><Badge variant="green">Priority-aware</Badge> — High-priority campaigns get blocks claimed first</li>
      </ul>

      <h2>Heartbeat Protocol</h2>
      <p>
        Nodes send heartbeats every 10 seconds with progress data:
      </p>
      <CodeBlock language="json">
        {`{
  "worker_id": "node-abc123",
  "operator_id": "op-alice",
  "status": "working",
  "current_block": {
    "job_id": 42,
    "progress": 0.67,
    "candidates_tested": 150000,
    "candidates_per_second": 12500
  },
  "system": {
    "cpu_usage": 0.95,
    "memory_mb": 3200,
    "cores": 8
  }
}`}
      </CodeBlock>
      <p>
        Nodes that miss heartbeats for 60 seconds are marked stale. Their
        in-progress work blocks are reclaimed by other nodes.
      </p>

      <h2>Scaling with systemd</h2>
      <p>
        For multi-node deployments, use systemd template units:
      </p>
      <CodeBlock language="ini">
        {`# /etc/systemd/system/darkreach-worker@.service
[Unit]
Description=darkreach worker %i
After=network-online.target

[Service]
User=darkreach
ExecStart=/usr/local/bin/darkreach work \\
  --coordinator https://api.darkreach.ai
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target`}
      </CodeBlock>
      <CodeBlock language="bash">
        {`# Enable and start 4 worker instances
sudo systemctl enable darkreach-worker@{1..4}
sudo systemctl start darkreach-worker@{1..4}`}
      </CodeBlock>

      <h2>Operator Dashboard</h2>
      <p>
        Operators can monitor their nodes at{" "}
        <a href="https://app.darkreach.ai/my-nodes">app.darkreach.ai/my-nodes</a>:
      </p>
      <ul>
        <li>Node online/offline status and uptime</li>
        <li>Current work block assignments and progress</li>
        <li>Compute contribution (core-hours, primes found)</li>
        <li>Performance metrics per node</li>
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
            <td><code>/api/v1/operators</code></td>
            <td>List operators with node counts and stats</td>
          </tr>
          <tr>
            <td><Badge variant="purple">POST</Badge></td>
            <td><code>/api/v1/operators</code></td>
            <td>Register a new operator</td>
          </tr>
          <tr>
            <td><Badge variant="green">GET</Badge></td>
            <td><code>/api/v1/nodes</code></td>
            <td>List all nodes with status</td>
          </tr>
          <tr>
            <td><Badge variant="purple">POST</Badge></td>
            <td><code>/api/v1/nodes/register</code></td>
            <td>Register a new node under an operator</td>
          </tr>
          <tr>
            <td><Badge variant="purple">POST</Badge></td>
            <td><code>/api/v1/nodes/heartbeat</code></td>
            <td>Node heartbeat with progress</td>
          </tr>
          <tr>
            <td><Badge variant="green">GET</Badge></td>
            <td><code>/api/fleet</code></td>
            <td>Fleet overview (all nodes + active searches)</td>
          </tr>
        </tbody>
      </table>

      <h2>Security</h2>
      <ul>
        <li>Operator registration requires approval for production deployments</li>
        <li>Node identity is tied to operator ID — nodes cannot impersonate other operators</li>
        <li>All results are re-verifiable via the verification pipeline</li>
        <li>Work block results include timing and system metadata for anomaly detection</li>
      </ul>
    </div>
  );
}
