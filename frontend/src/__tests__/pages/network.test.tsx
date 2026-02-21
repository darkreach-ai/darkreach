/**
 * @file Tests for the Network page
 * @module __tests__/pages/network
 *
 * Validates the Network page at `/network`, which shows compute machines
 * as collapsible sections with node tables. Tests verify page heading,
 * subtitle, stat cards (Machines, Nodes, Cores, Throughput), filters,
 * and empty state.
 *
 * @see {@link ../../app/network/page} Source page
 * @see {@link ../../contexts/websocket-context} WebSocket data provider
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("@/contexts/websocket-context", () => ({
  useWs: () => ({
    fleet: {
      workers: [],
      servers: [],
      total_workers: 0,
      total_cores: 0,
      total_tested: 0,
      total_found: 0,
    },
    connected: true,
  }),
}));

vi.mock("@/components/view-header", () => ({
  ViewHeader: ({
    title,
    subtitle,
  }: {
    title: string;
    subtitle: string;
    actions?: React.ReactNode;
  }) => (
    <div data-testid="view-header">
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
  ),
}));

vi.mock("@/components/worker-detail-dialog", () => ({
  WorkerDetailDialog: () => null,
}));

vi.mock("@/components/stat-card", () => ({
  StatCard: ({ label, value }: { label: string; value: React.ReactNode }) => (
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

vi.mock("@/components/ui/collapsible", () => ({
  Collapsible: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  CollapsibleTrigger: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  CollapsibleContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("@/lib/format", () => ({
  API_BASE: "http://localhost:3000",
  numberWithCommas: (x: number) => String(x),
}));

import NetworkPage from "@/app/network/page";

describe("NetworkPage", () => {
  it("renders without crashing", () => {
    render(<NetworkPage />);
    expect(screen.getByText("Network")).toBeInTheDocument();
  });

  it("shows network subtitle", () => {
    render(<NetworkPage />);
    expect(
      screen.getByText(
        "Compute nodes powering the distributed search network."
      )
    ).toBeInTheDocument();
  });

  it("renders stat cards", () => {
    render(<NetworkPage />);
    expect(screen.getByTestId("stat-machines")).toBeInTheDocument();
    expect(screen.getByTestId("stat-nodes")).toBeInTheDocument();
    expect(screen.getByTestId("stat-cores")).toBeInTheDocument();
    expect(screen.getByTestId("stat-throughput")).toBeInTheDocument();
  });

  it("shows empty state when no compute machines", () => {
    render(<NetworkPage />);
    expect(screen.getByText("No compute machines online.")).toBeInTheDocument();
  });

  it("does not render Add Server or New Search buttons", () => {
    render(<NetworkPage />);
    expect(screen.queryByText("Add Server")).not.toBeInTheDocument();
    expect(screen.queryByText("New Search")).not.toBeInTheDocument();
  });

  it("does not render Deployments or Search Queue sections", () => {
    render(<NetworkPage />);
    expect(screen.queryByText("Deployments")).not.toBeInTheDocument();
    expect(screen.queryByText("Search Queue")).not.toBeInTheDocument();
  });
});
