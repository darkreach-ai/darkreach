"use client";

import { CodeBlock } from "@/components/ui/code-block";
import { Badge } from "@/components/ui/badge";

interface Endpoint {
  method: string;
  path: string;
  description: string;
  response?: string;
}

interface EndpointGroup {
  title: string;
  description: string;
  endpoints: Endpoint[];
}

const groups: EndpointGroup[] = [
  {
    title: "Primes",
    description: "Query, filter, and inspect discovered primes.",
    endpoints: [
      {
        method: "GET",
        path: "/api/primes",
        description: "List discovered primes with pagination and filtering.",
        response: `{
  "primes": [
    {
      "id": 1,
      "form": "factorial",
      "expression": "147855! + 1",
      "digits": 636919,
      "proof_type": "pocklington",
      "certificate": { ... },
      "discovered_at": "2026-02-14T12:00:00Z"
    }
  ],
  "total": 2847,
  "page": 1,
  "per_page": 50
}`,
      },
      {
        method: "GET",
        path: "/api/primes/:id",
        description: "Get a single prime with full certificate data.",
      },
      {
        method: "GET",
        path: "/api/stats",
        description: "Aggregate statistics: total primes, candidates tested, active workers.",
        response: `{
  "total_primes": 2847,
  "candidates_tested": 14200000000,
  "active_workers": 38,
  "compute_hours": 127000,
  "search_forms": 12
}`,
      },
    ],
  },
  {
    title: "Verification",
    description: "Re-verify primes and inspect certificates.",
    endpoints: [
      {
        method: "POST",
        path: "/api/verify",
        description:
          "Re-verify a prime through the 3-tier pipeline. Returns updated proof status.",
        response: `{
  "prime_id": 42,
  "result": "verified",
  "proof_type": "pocklington",
  "certificate": { ... }
}`,
      },
    ],
  },
  {
    title: "Workers & Nodes",
    description: "Worker registration, heartbeat, and node management.",
    endpoints: [
      {
        method: "GET",
        path: "/api/workers",
        description: "List all registered workers with status and last heartbeat.",
      },
      {
        method: "POST",
        path: "/api/workers/register",
        description: "Register a new worker with the coordinator.",
      },
      {
        method: "POST",
        path: "/api/workers/heartbeat",
        description: "Worker heartbeat with progress and status update.",
      },
      {
        method: "GET",
        path: "/api/fleet",
        description: "Fleet overview: all nodes, active searches, aggregate stats.",
      },
    ],
  },
  {
    title: "Operators",
    description: "Operator accounts and node management (v1 API).",
    endpoints: [
      {
        method: "GET",
        path: "/api/v1/operators",
        description: "List operators with node counts, compute hours, and prime counts.",
        response: `{
  "operators": [
    {
      "id": "op-abc",
      "name": "alice",
      "contact_email": "alice@example.com",
      "node_count": 4,
      "total_compute_hours": 1200,
      "primes_found": 23
    }
  ]
}`,
      },
      {
        method: "POST",
        path: "/api/v1/operators",
        description: "Register a new operator account.",
      },
      {
        method: "GET",
        path: "/api/v1/nodes",
        description: "List all nodes across all operators with status.",
      },
      {
        method: "POST",
        path: "/api/v1/nodes/register",
        description: "Register a new node under an operator.",
      },
      {
        method: "POST",
        path: "/api/v1/nodes/heartbeat",
        description: "Node heartbeat with system metrics and progress.",
      },
    ],
  },
  {
    title: "Search Jobs",
    description: "Job lifecycle, work blocks, and search management.",
    endpoints: [
      {
        method: "GET",
        path: "/api/search_jobs",
        description: "List search jobs with status (running, completed, paused).",
      },
      {
        method: "POST",
        path: "/api/search_jobs",
        description: "Create a new search job.",
      },
      {
        method: "POST",
        path: "/api/search_jobs/claim",
        description: "Claim the next available work block (FOR UPDATE SKIP LOCKED).",
      },
      {
        method: "POST",
        path: "/api/search_jobs/:id/complete",
        description: "Mark a work block as completed with results.",
      },
      {
        method: "GET",
        path: "/api/searches",
        description: "List active searches with progress and rate metrics.",
      },
      {
        method: "POST",
        path: "/api/searches",
        description: "Start a new search with form and range parameters.",
      },
    ],
  },
  {
    title: "AI Agents",
    description: "Agent task management, budgets, memory, roles, and schedules.",
    endpoints: [
      {
        method: "GET",
        path: "/api/agents/tasks",
        description: "List agent tasks with status and assigned agent.",
        response: `{
  "tasks": [
    {
      "id": "task-123",
      "title": "Optimize factorial sieve depth",
      "status": "running",
      "agent": "strategy-agent",
      "created_at": "2026-02-20T10:00:00Z"
    }
  ]
}`,
      },
      {
        method: "POST",
        path: "/api/agents/tasks",
        description: "Create a new agent task.",
      },
      {
        method: "PUT",
        path: "/api/agents/tasks/:id",
        description: "Update task status or assignment.",
      },
      {
        method: "GET",
        path: "/api/agents/budgets",
        description: "Get agent compute budgets and spend tracking.",
      },
      {
        method: "POST",
        path: "/api/agents/budgets",
        description: "Set or update an agent budget.",
      },
      {
        method: "GET",
        path: "/api/agents/memory",
        description: "Read agent memory key-value store.",
      },
      {
        method: "POST",
        path: "/api/agents/memory",
        description: "Write to agent memory.",
      },
      {
        method: "GET",
        path: "/api/agents/schedules",
        description: "List agent automation schedules.",
      },
      {
        method: "POST",
        path: "/api/agents/schedules",
        description: "Create or update a schedule.",
      },
    ],
  },
  {
    title: "Projects",
    description: "Multi-phase campaign management.",
    endpoints: [
      {
        method: "GET",
        path: "/api/projects",
        description: "List all projects with phase status and progress.",
      },
      {
        method: "POST",
        path: "/api/projects",
        description: "Create a new project campaign.",
      },
      {
        method: "GET",
        path: "/api/projects/:id",
        description: "Get project details with phases, events, and cost breakdown.",
      },
      {
        method: "PUT",
        path: "/api/projects/:id",
        description: "Update project configuration or budget.",
      },
      {
        method: "POST",
        path: "/api/projects/:id/advance",
        description: "Manually advance to the next phase.",
      },
    ],
  },
  {
    title: "Observability",
    description: "Metrics, logs, and performance monitoring.",
    endpoints: [
      {
        method: "GET",
        path: "/api/observability/metrics",
        description: "System metrics: CPU, memory, disk, and worker rates.",
      },
      {
        method: "GET",
        path: "/api/observability/logs",
        description: "Recent system logs with filtering and pagination.",
      },
      {
        method: "GET",
        path: "/api/observability/charts",
        description: "Time-series data for performance charts.",
      },
    ],
  },
  {
    title: "Releases",
    description: "Worker release channels and canary rollout control.",
    endpoints: [
      {
        method: "GET",
        path: "/api/releases/worker/latest?channel=stable",
        description: "Get latest worker release metadata for a channel.",
        response: `{
  "channel": "stable",
  "version": "0.1.0",
  "published_at": "2026-02-20T00:00:00Z",
  "notes": "Initial public release",
  "artifacts": [
    {
      "os": "linux",
      "arch": "x86_64",
      "url": "https://downloads.darkreach.ai/...",
      "sha256": "..."
    }
  ]
}`,
      },
      {
        method: "POST",
        path: "/api/releases/worker",
        description: "Upsert a worker release record.",
      },
      {
        method: "POST",
        path: "/api/releases/rollout",
        description: "Set channel target version and rollout percentage.",
      },
      {
        method: "POST",
        path: "/api/releases/rollback",
        description: "Rollback a channel to the previous version.",
      },
      {
        method: "GET",
        path: "/api/releases/events?channel=stable&limit=100",
        description: "Audit trail of rollout and rollback events.",
      },
      {
        method: "GET",
        path: "/api/releases/health?active_hours=24",
        description: "Release adoption summary by worker version.",
      },
    ],
  },
  {
    title: "Documentation",
    description: "Documentation search and content serving.",
    endpoints: [
      {
        method: "GET",
        path: "/api/docs",
        description: "List all documentation files with titles and categories.",
      },
      {
        method: "GET",
        path: "/api/docs/:slug",
        description: "Get a documentation file by slug.",
      },
      {
        method: "GET",
        path: "/api/docs/search?q=query",
        description: "Multi-word search across all docs with relevance ranking.",
        response: `{
  "results": [
    {
      "slug": "sophie-germain",
      "title": "Sophie Germain Primes",
      "snippets": [
        { "text": "...matching line...", "line": 42 }
      ],
      "category": "roadmaps",
      "score": 35
    }
  ]
}`,
      },
      {
        method: "GET",
        path: "/api/docs/roadmaps/:slug",
        description: "Get a roadmap document.",
      },
      {
        method: "GET",
        path: "/api/docs/agent/:slug",
        description: "Get a CLAUDE.md agent file.",
      },
    ],
  },
  {
    title: "Notifications",
    description: "Push notification management.",
    endpoints: [
      {
        method: "GET",
        path: "/api/notifications",
        description: "List notification subscriptions.",
      },
      {
        method: "POST",
        path: "/api/notifications/subscribe",
        description: "Subscribe to push notifications for prime discoveries.",
      },
    ],
  },
  {
    title: "Health & Status",
    description: "Service health checks and coordinator status.",
    endpoints: [
      {
        method: "GET",
        path: "/api/health",
        description: "Health check and readiness probe.",
      },
      {
        method: "GET",
        path: "/api/status",
        description: "Coordinator status with uptime, version, and worker count.",
        response: `{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "database": "connected",
  "workers_online": 38,
  "active_searches": 5,
  "ai_engine": "running"
}`,
      },
    ],
  },
];

const wsEvents = [
  {
    event: "prime_discovered",
    direction: "server → client",
    description: "Broadcast when a new prime is found by any node.",
    payload: `{ "form": "factorial", "expression": "147855! + 1", "digits": 636919, "proof_type": "pocklington" }`,
  },
  {
    event: "worker_status",
    direction: "server → client",
    description: "Fleet status update pushed every 2 seconds.",
    payload: `{ "workers": [...], "active_searches": [...], "total_rate": 150000 }`,
  },
  {
    event: "search_progress",
    direction: "server → client",
    description: "Search progress update with candidates tested and rate.",
    payload: `{ "job_id": 1, "progress": 0.42, "candidates_per_second": 15000 }`,
  },
  {
    event: "ai_decision",
    direction: "server → client",
    description: "AI engine decision notification with reasoning.",
    payload: `{ "action": "start_search", "form": "kbn", "confidence": 0.87, "reasoning": "..." }`,
  },
  {
    event: "project_event",
    direction: "server → client",
    description: "Project phase transition or milestone.",
    payload: `{ "project_id": "proj-1", "event": "phase_advance", "from": "scout", "to": "search" }`,
  },
  {
    event: "subscribe",
    direction: "client → server",
    description: "Subscribe to specific event channels.",
    payload: `{ "channels": ["primes", "fleet", "searches", "ai", "projects"] }`,
  },
];

function MethodBadge({ method }: { method: string }) {
  const variant =
    method === "GET" ? "green" : method === "POST" ? "purple" : method === "PUT" ? "orange" : "default";
  return <Badge variant={variant}>{method}</Badge>;
}

export default function ApiPage() {
  return (
    <div className="prose-docs">
      <h1>API Reference</h1>
      <p>
        The darkreach coordinator exposes a REST API and WebSocket endpoint for
        nodes, the dashboard, and third-party integrations. The API is organized
        into 12 endpoint groups covering all platform functionality.
      </p>
      <p>
        <strong>Base URL:</strong>{" "}
        <code>https://api.darkreach.ai</code>
      </p>

      <h2>REST Endpoints</h2>
      {groups.map((group) => (
        <div key={group.title}>
          <h3>{group.title}</h3>
          <p className="text-sm text-muted-foreground mt-0">{group.description}</p>
          <div className="space-y-4 mt-3 mb-8">
            {group.endpoints.map((ep) => (
              <div
                key={`${ep.method}-${ep.path}`}
                className="border border-border rounded-lg p-4 bg-card"
              >
                <div className="flex items-center gap-3 mb-2">
                  <MethodBadge method={ep.method} />
                  <code className="text-sm text-accent-purple">{ep.path}</code>
                </div>
                <p className="text-sm text-muted-foreground m-0">
                  {ep.description}
                </p>
                {ep.response && (
                  <div className="mt-3">
                    <CodeBlock language="json">{ep.response}</CodeBlock>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      ))}

      <h2>WebSocket</h2>
      <p>
        Connect to <code>wss://api.darkreach.ai/ws</code> for real-time
        updates. Messages are JSON-encoded with an <code>event</code> field.
        The server pushes updates every 2 seconds.
      </p>

      <div className="space-y-4 mt-4">
        {wsEvents.map((ev) => (
          <div
            key={ev.event}
            className="border border-border rounded-lg p-4 bg-card"
          >
            <div className="flex items-center gap-3 mb-2">
              <Badge
                variant={
                  ev.direction.startsWith("server") ? "green" : "purple"
                }
              >
                {ev.direction}
              </Badge>
              <code className="text-sm text-accent-purple">{ev.event}</code>
            </div>
            <p className="text-sm text-muted-foreground m-0 mb-3">
              {ev.description}
            </p>
            <CodeBlock language="json">{ev.payload}</CodeBlock>
          </div>
        ))}
      </div>

      <h2>Authentication</h2>
      <p>
        The dashboard uses Supabase Auth (email/password) for login. API
        endpoints for read operations are currently unauthenticated. Write
        operations on the operator and agent APIs require an operator ID for
        identity. Node registration uses coordinator-assigned worker IDs.
      </p>

      <h2>Rate Limits</h2>
      <p>
        The API does not currently enforce rate limits. However, heartbeat
        endpoints are designed for 10-second intervals, and the WebSocket
        connection pushes at 2-second intervals. Excessive polling is
        discouraged.
      </p>
    </div>
  );
}
