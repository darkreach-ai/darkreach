/**
 * @file Tests for the My Nodes page
 * @module __tests__/pages/my-nodes
 *
 * Validates the My Nodes page at `/my-nodes`, which is an auth-guarded page
 * showing the operator's registered compute nodes. The page fetches node data
 * from `/api/v1/operators/me/nodes` using the Supabase JWT. Tests verify
 * page heading, subtitle, stat cards (Total Nodes, Total Cores, Online Nodes),
 * loading/error/empty states, node table with hostname and worker_id columns,
 * Online/Offline badges based on heartbeat recency, and total core calculation.
 *
 * @see {@link ../../app/my-nodes/page} Source page
 * @see {@link ../../contexts/auth-context} Auth context (Supabase session)
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

// ── Mutable mock state ──────────────────────────────────────────

let mockSession: { access_token: string } | null = {
  access_token: "test-token",
};

// ── Mock hooks and components ───────────────────────────────────

vi.mock("@/contexts/auth-context", () => ({
  useAuth: () => ({ session: mockSession }),
}));

vi.mock("@/components/view-header", () => ({
  ViewHeader: ({
    title,
    subtitle,
  }: {
    title: string;
    subtitle?: string;
    className?: string;
  }) => (
    <div data-testid="view-header">
      <h1>{title}</h1>
      {subtitle && <p>{subtitle}</p>}
    </div>
  ),
}));

vi.mock("@/components/stat-card", () => ({
  StatCard: ({
    label,
    value,
  }: {
    label: string;
    value: React.ReactNode;
    icon?: React.ReactNode;
  }) => (
    <div data-testid={`stat-${label.toLowerCase().replace(/\s+/g, "-")}`}>
      {label}: {value}
    </div>
  ),
}));

vi.mock("@/components/empty-state", () => ({
  EmptyState: ({ message }: { message: string }) => (
    <div data-testid="empty-state">{message}</div>
  ),
}));

vi.mock("lucide-react", () => ({
  Server: () => <span data-testid="icon-server" />,
  Cpu: () => <span data-testid="icon-cpu" />,
  HardDrive: () => <span data-testid="icon-harddrive" />,
  Wifi: () => <span data-testid="icon-wifi" />,
  WifiOff: () => <span data-testid="icon-wifioff" />,
}));

vi.mock("@/lib/format", () => ({
  API_BASE: "http://localhost:3000",
  relativeTime: (iso: string) => "1m ago",
}));

const mockFetch = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  global.fetch = mockFetch;
  mockSession = { access_token: "test-token" };
});

import MyNodesPage from "@/app/my-nodes/page";

// ── Test data ───────────────────────────────────────────────────

const recentHeartbeat = new Date(Date.now() - 30_000).toISOString(); // 30s ago (online)
const staleHeartbeat = new Date(Date.now() - 300_000).toISOString(); // 5 min ago (offline)

const mockNodes = [
  {
    worker_id: "node-abc-123",
    hostname: "compute-01",
    cores: 8,
    cpu_model: "AMD Ryzen 9 5950X",
    os: "Linux",
    arch: "x86_64",
    ram_gb: 64,
    has_gpu: false,
    gpu_model: null,
    worker_version: "0.8.0",
    registered_at: "2026-01-01T00:00:00Z",
    last_heartbeat: recentHeartbeat,
  },
  {
    worker_id: "node-def-456",
    hostname: "compute-02",
    cores: 4,
    cpu_model: "Intel i7-12700K",
    os: "Linux",
    arch: "x86_64",
    ram_gb: 32,
    has_gpu: true,
    gpu_model: "RTX 3080",
    worker_version: "0.7.5",
    registered_at: "2026-01-15T00:00:00Z",
    last_heartbeat: staleHeartbeat,
  },
];

// ── Tests ───────────────────────────────────────────────────────

describe("MyNodesPage", () => {
  it("renders 'My Nodes' title", () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });
    render(<MyNodesPage />);
    expect(screen.getByText("My Nodes")).toBeInTheDocument();
  });

  it("renders stat cards (Total Nodes, Total Cores, Online Nodes)", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockNodes),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByTestId("stat-total-nodes")).toBeInTheDocument();
      expect(screen.getByTestId("stat-total-cores")).toBeInTheDocument();
      expect(screen.getByTestId("stat-online-nodes")).toBeInTheDocument();
    });
  });

  it("shows loading state", () => {
    mockFetch.mockReturnValue(new Promise(() => {})); // Never resolves
    render(<MyNodesPage />);
    expect(screen.getByText("Loading nodes...")).toBeInTheDocument();
  });

  it("shows error state", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
      json: () => Promise.resolve({ error: "Internal error" }),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByText("Internal error")).toBeInTheDocument();
    });
  });

  it("shows empty state when no nodes", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByTestId("empty-state")).toBeInTheDocument();
    });
  });

  it("renders node table with hostname", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockNodes),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByText("compute-01")).toBeInTheDocument();
      expect(screen.getByText("compute-02")).toBeInTheDocument();
    });
  });

  it("renders node table with worker_id", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockNodes),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByText("node-abc-123")).toBeInTheDocument();
      expect(screen.getByText("node-def-456")).toBeInTheDocument();
    });
  });

  it("renders Online badge for recent heartbeat (< 2 min)", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockNodes),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByText("Online")).toBeInTheDocument();
    });
  });

  it("renders Offline badge for stale heartbeat (> 2 min)", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockNodes),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      expect(screen.getByText("Offline")).toBeInTheDocument();
    });
  });

  it("calculates total cores correctly", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockNodes),
    });
    render(<MyNodesPage />);
    await waitFor(() => {
      // Total cores = 8 + 4 = 12
      expect(screen.getByTestId("stat-total-cores")).toHaveTextContent("12");
    });
  });
});
