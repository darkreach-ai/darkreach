/**
 * @file Tests for the CostHistoryChart component
 * @module __tests__/components/charts/cost-history
 *
 * Validates the cumulative cost time-series chart used on project pages.
 * The component fetches cost history from `/api/projects/:slug/cost-history`
 * and renders a Recharts AreaChart with optional budget reference line.
 *
 * Recharts is mocked to simple div elements since jsdom cannot render SVG.
 * Tests focus on empty state, successful data rendering, budget reference
 * line, and error handling.
 *
 * @see {@link ../../../components/charts/cost-history} Source component
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

// Mock Recharts to avoid SVG rendering in jsdom.
vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  AreaChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="area-chart">{children}</div>
  ),
  Area: () => <div data-testid="area" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  ReferenceLine: ({ label }: any) => (
    <div data-testid="reference-line">{label?.value}</div>
  ),
}));

import { CostHistoryChart } from "@/components/charts/cost-history";

const mockHistory = {
  history: [
    { time: "2026-02-20T10:00:00Z", core_hours: 10, cost_usd: 1.5 },
    { time: "2026-02-20T11:00:00Z", core_hours: 25, cost_usd: 3.75 },
  ],
  budget_usd: 50,
  cloud_rate: 0.15,
  total_core_hours: 25,
  total_cost_usd: 3.75,
};

describe("CostHistoryChart", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  /** Verifies the empty state message is shown when the API returns no data. */
  it("renders empty state when no data", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            history: [],
            budget_usd: null,
            cloud_rate: 0.15,
            total_core_hours: 0,
            total_cost_usd: 0,
          }),
      })
    );

    render(<CostHistoryChart slug="test-project" apiBase="" />);

    await waitFor(() => {
      expect(screen.getByText("No cost data yet")).toBeInTheDocument();
    });
  });

  /** Verifies the chart renders with area and total cost when data is available. */
  it("renders chart with data", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockHistory),
      })
    );

    render(<CostHistoryChart slug="test-project" apiBase="" />);

    await waitFor(() => {
      expect(screen.getByTestId("area-chart")).toBeInTheDocument();
    });

    expect(screen.getByText("Cumulative cost")).toBeInTheDocument();
    expect(screen.getByText("$3.75")).toBeInTheDocument();
  });

  /** Verifies the empty state when fetch fails (silent error handling). */
  it("handles fetch error gracefully", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
      })
    );

    render(<CostHistoryChart slug="test-project" apiBase="" />);

    // On fetch failure the component silently ignores — shows empty state
    // (data remains null, so empty state is rendered)
    expect(screen.getByText("No cost data yet")).toBeInTheDocument();
  });

  /** Verifies the budget reference line renders when budget_usd is set. */
  it("renders budget reference line when budget is set", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockHistory),
      })
    );

    render(<CostHistoryChart slug="test-project" apiBase="" />);

    await waitFor(() => {
      expect(screen.getByTestId("reference-line")).toBeInTheDocument();
    });
  });

  /** Verifies the correct API URL is called with slug. */
  it("fetches from correct API endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockHistory),
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<CostHistoryChart slug="my-project" apiBase="https://api.test" />);

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "https://api.test/api/projects/my-project/cost-history"
      );
    });
  });
});
