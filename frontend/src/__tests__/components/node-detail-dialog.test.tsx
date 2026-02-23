/**
 * @file Tests for the NodeDetailDialog component
 * @module __tests__/components/node-detail-dialog
 *
 * Validates the modal dialog that shows detailed information about a single
 * network node. When opened without a node (null), it renders a placeholder
 * "Node" title. When a node is provided, it displays the hostname, health
 * status dot (green/yellow/red based on heartbeat age), throughput, hardware
 * metrics bars, search parameters, and checkpoint data.
 *
 * All external UI components (Dialog, MetricsBar, JsonBlock) are mocked to
 * simple div elements to isolate the rendering logic under test.
 *
 * @see {@link ../../components/node-detail-dialog} NodeDetailDialog source
 * @see {@link ../../hooks/use-websocket} NodeStatus, HardwareMetrics types
 * @see {@link ../../__mocks__/test-wrappers} makeNodeStatus factory
 */
import { vi, describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock Dialog components — render conditionally based on `open` prop.
vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: React.ReactNode; open: boolean }) =>
    open ? <div data-testid="dialog">{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dialog-content">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="dialog-header">{children}</div>
  ),
  DialogTitle: ({ children, ...props }: { children: React.ReactNode; [k: string]: unknown }) => (
    <h2 data-testid="dialog-title" {...props}>{children}</h2>
  ),
}));

// Mock Badge component.
vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children }: { children: React.ReactNode }) => (
    <span data-testid="badge">{children}</span>
  ),
}));

// Mock MetricsBar — renders label and percent as plain text for assertion.
vi.mock("@/components/metrics-bar", () => ({
  MetricsBar: ({ label, percent }: { label: string; percent: number }) => (
    <div data-testid={`metrics-${label}`}>{percent}%</div>
  ),
}));

// Mock JsonBlock — renders a testid for presence detection.
vi.mock("@/components/json-block", () => ({
  JsonBlock: ({ label }: { label: string }) => (
    <div data-testid={`json-${label}`} />
  ),
}));

// Mock format utilities.
vi.mock("@/lib/format", () => ({
  numberWithCommas: (x: number) =>
    x.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ","),
  formatUptime: (secs: number) => {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  },
}));

import { NodeDetailDialog } from "@/components/node-detail-dialog";
import { makeNodeStatus } from "@/__mocks__/test-wrappers";
import type { HardwareMetrics } from "@/hooks/use-websocket";

/** Factory for HardwareMetrics with sensible defaults. */
function makeMetrics(overrides: Partial<HardwareMetrics> = {}): HardwareMetrics {
  return {
    cpu_usage_percent: 45.0,
    memory_used_gb: 8.0,
    memory_total_gb: 16.0,
    memory_usage_percent: 50.0,
    disk_used_gb: 100.0,
    disk_total_gb: 500.0,
    disk_usage_percent: 20.0,
    load_avg_1m: 2.0,
    load_avg_5m: 1.8,
    load_avg_15m: 1.5,
    ...overrides,
  };
}

const noop = () => {};

describe("NodeDetailDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders "Node" title when node is null', () => {
    render(<NodeDetailDialog node={null} open={true} onOpenChange={noop} />);
    expect(screen.getByText("Node")).toBeInTheDocument();
  });

  it("renders hostname in title when node provided", () => {
    const node = makeNodeStatus({ hostname: "compute-gamma" }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    expect(screen.getByText("compute-gamma")).toBeInTheDocument();
  });

  it("shows green dot for heartbeat < 30s", () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 5 }) as any;
    const { container } = render(
      <NodeDetailDialog node={node} open={true} onOpenChange={noop} />
    );
    const dot = container.querySelector(".bg-green-500");
    expect(dot).toBeInTheDocument();
  });

  it("shows yellow dot for heartbeat 30-59s", () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 45 }) as any;
    const { container } = render(
      <NodeDetailDialog node={node} open={true} onOpenChange={noop} />
    );
    const dot = container.querySelector(".bg-yellow-500");
    expect(dot).toBeInTheDocument();
  });

  it("shows red dot for heartbeat >= 60s", () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 120 }) as any;
    const { container } = render(
      <NodeDetailDialog node={node} open={true} onOpenChange={noop} />
    );
    const dot = container.querySelector(".bg-red-500");
    expect(dot).toBeInTheDocument();
  });

  it("calculates throughput (tested / uptime_secs)", () => {
    const node = makeNodeStatus({ tested: 7200, uptime_secs: 3600 }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    // 7200 / 3600 = 2.0
    expect(screen.getByText("2.0 candidates/sec")).toBeInTheDocument();
  });

  it('shows "0.0" throughput for 0 uptime', () => {
    const node = makeNodeStatus({ tested: 500, uptime_secs: 0 }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    expect(screen.getByText("0.0 candidates/sec")).toBeInTheDocument();
  });

  it('shows "just now" for heartbeat < 5s', () => {
    const node = makeNodeStatus({ last_heartbeat_secs_ago: 2 }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    expect(screen.getByText("just now")).toBeInTheDocument();
  });

  it("shows metrics bars when node.metrics is present", () => {
    const node = makeNodeStatus({ metrics: makeMetrics() }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    expect(screen.getByTestId("metrics-CPU")).toBeInTheDocument();
    expect(screen.getByTestId("metrics-Memory")).toBeInTheDocument();
    expect(screen.getByTestId("metrics-Disk")).toBeInTheDocument();
  });

  it("renders JsonBlock for search params", () => {
    const node = makeNodeStatus({
      search_params: JSON.stringify({ k: 3, base: 2 }),
    }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    expect(screen.getByTestId("json-Search parameters")).toBeInTheDocument();
  });

  it("renders JsonBlock for checkpoint when present", () => {
    const node = makeNodeStatus({
      checkpoint: JSON.stringify({ progress: 0.5 }),
    }) as any;
    render(<NodeDetailDialog node={node} open={true} onOpenChange={noop} />);
    expect(screen.getByTestId("json-Checkpoint")).toBeInTheDocument();
  });
});
