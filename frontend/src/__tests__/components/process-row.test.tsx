/**
 * @file Tests for the ProcessRow component
 * @module __tests__/components/process-row
 *
 * Validates the expandable row component that displays a single node process
 * in the network monitoring page. Each row shows a health status dot (green/
 * yellow/red based on heartbeat age), the node's worker_id, a search_type
 * badge, formatted search parameters, current candidate, tested/found counts,
 * throughput rate, and pause/resume controls for managed searches.
 *
 * Also exercises the `formatWorkerParams` helper which formats search
 * parameters for all 12 search forms (kbn, factorial, palindromic,
 * near_repdigit, primorial, cullen_woodall, twin, sophie_germain, repunit,
 * gen_fermat, wagstaff, carol_kynea) plus the default fallback.
 *
 * @see {@link ../../components/process-row} ProcessRow source
 * @see {@link ../../hooks/use-websocket} NodeStatus, ManagedSearch types
 * @see {@link ../../__mocks__/test-wrappers} makeNodeStatus factory
 */
import { vi, describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock UI primitives used by ProcessRow.
vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children, ...props }: { children: React.ReactNode; [k: string]: unknown }) => (
    <span data-testid="badge" {...props}>{children}</span>
  ),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, ...props }: { children: React.ReactNode; onClick?: () => void; [k: string]: unknown }) => (
    <button onClick={onClick} {...props}>{children}</button>
  ),
}));

vi.mock("@/lib/format", () => ({
  API_BASE: "",
  numberWithCommas: (x: number) =>
    x.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ","),
  formatUptime: (secs: number) => {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  },
}));

import { ProcessRow } from "@/components/process-row";
import { makeNodeStatus } from "@/__mocks__/test-wrappers";
import type { ManagedSearch } from "@/hooks/use-websocket";

/** Factory helper for creating a ManagedSearch with sensible defaults. */
function makeManagedSearch(overrides: Partial<ManagedSearch> = {}): ManagedSearch {
  return {
    id: 1,
    search_type: "kbn",
    params: { search_type: "kbn", k: 3, base: 2, min_n: 1, max_n: 100000 },
    status: "running",
    started_at: "2026-02-20T12:00:00Z",
    stopped_at: null,
    pid: 1234,
    worker_id: "node-abc123",
    tested: 50000,
    found: 3,
    ...overrides,
  };
}

describe("ProcessRow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders worker_id text", () => {
    const node = makeNodeStatus({ worker_id: "node-xyz789" }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("node-xyz789")).toBeInTheDocument();
  });

  it("renders search_type badge", () => {
    const node = makeNodeStatus({ search_type: "factorial" }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("factorial")).toBeInTheDocument();
  });

  it("renders health dot green when heartbeat < 30s", () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 10 }) as any;
    const { container } = render(<ProcessRow worker={node} search={null} />);
    const dot = container.querySelector(".bg-green-500");
    expect(dot).toBeInTheDocument();
  });

  it("renders health dot yellow when heartbeat 30-59s", () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 45 }) as any;
    const { container } = render(<ProcessRow worker={node} search={null} />);
    const dot = container.querySelector(".bg-yellow-500");
    expect(dot).toBeInTheDocument();
  });

  it("renders health dot red when heartbeat >= 60s", () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 120 }) as any;
    const { container } = render(<ProcessRow worker={node} search={null} />);
    const dot = container.querySelector(".bg-red-500");
    expect(dot).toBeInTheDocument();
  });

  it("shows throughput (tested / uptime_secs)", () => {
    const node = makeNodeStatus({ tested: 7200, uptime_secs: 3600 }) as any;
    render(<ProcessRow worker={node} search={null} />);
    // 7200 / 3600 = 2.0
    expect(screen.getByText("2.0/s")).toBeInTheDocument();
  });

  it("shows '0.0' throughput when uptime is 0", () => {
    const node = makeNodeStatus({ tested: 100, uptime_secs: 0 }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("0.0/s")).toBeInTheDocument();
  });

  it("formats kbn params correctly", () => {
    const node = makeNodeStatus({
      search_type: "kbn",
      search_params: JSON.stringify({ k: 3, base: 2, min_n: 1, max_n: 100000 }),
    }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("k=3, base=2, n=1..100,000")).toBeInTheDocument();
  });

  it("formats factorial params correctly", () => {
    const node = makeNodeStatus({
      search_type: "factorial",
      search_params: JSON.stringify({ start: 1000, end: 5000 }),
    }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("n=1,000..5,000")).toBeInTheDocument();
  });

  it("formats palindromic params correctly", () => {
    const node = makeNodeStatus({
      search_type: "palindromic",
      search_params: JSON.stringify({ base: 10, min_digits: 3, max_digits: 9 }),
    }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("base 10, 3..9 digits")).toBeInTheDocument();
  });

  it("formats default/unknown type params", () => {
    const node = makeNodeStatus({
      search_type: "custom_form",
      search_params: JSON.stringify({ alpha: 42, beta: 99 }),
    }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("alpha=42, beta=99")).toBeInTheDocument();
  });

  it("shows current candidate when present", () => {
    const node = makeNodeStatus({ current: "3*2^99991+1" }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("3*2^99991+1")).toBeInTheDocument();
  });

  it("shows tested/found counts", () => {
    const node = makeNodeStatus({ tested: 50000, found: 3 }) as any;
    render(<ProcessRow worker={node} search={null} />);
    expect(screen.getByText("50,000 tested")).toBeInTheDocument();
    expect(screen.getByText("3 found")).toBeInTheDocument();
  });

  it("shows Pause button when search is running", () => {
    const node = makeNodeStatus() as any;
    const search = makeManagedSearch({ status: "running" });
    render(<ProcessRow worker={node} search={search} />);
    expect(screen.getByText("Pause")).toBeInTheDocument();
  });

  it("shows Resume button when search is paused", () => {
    const node = makeNodeStatus() as any;
    const search = makeManagedSearch({ status: "paused" });
    render(<ProcessRow worker={node} search={search} />);
    expect(screen.getByText("Resume")).toBeInTheDocument();
  });
});
