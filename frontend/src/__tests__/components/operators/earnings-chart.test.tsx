/**
 * @file Tests for the EarningsChart component
 * @module __tests__/components/operators/earnings-chart
 *
 * Validates the Recharts BarChart that visualizes monthly credit earnings
 * on the operator earnings page. Shows an empty state message when the
 * earnings array is empty, and a bar chart with monthly data when populated.
 *
 * Recharts is mocked to simple div elements since jsdom cannot render SVG.
 * Tests focus on conditional rendering (empty vs populated), correct data
 * count passed to BarChart, null-safe handling of total_credits and
 * block_count fields, and the Bar element's dataKey configuration.
 *
 * @see {@link ../../../components/operators/earnings-chart} EarningsChart source
 * @see {@link ../../../hooks/use-earnings} MonthlyEarning type
 * @see {@link ../../../__mocks__/test-wrappers} makeMonthlyEarning factory
 */
import { vi, describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock Recharts — replace chart components with divs carrying data attributes
// for assertion. BarChart receives `data` prop which we expose via data-count.
vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  BarChart: ({ children, data }: { children: React.ReactNode; data?: unknown[] }) => (
    <div data-testid="bar-chart" data-count={data?.length}>{children}</div>
  ),
  Bar: (props: { dataKey?: string; [k: string]: unknown }) => (
    <div data-testid="bar" data-datakey={props.dataKey} />
  ),
  XAxis: (props: { dataKey?: string; [k: string]: unknown }) => (
    <div data-testid="x-axis" data-datakey={props.dataKey} />
  ),
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
}));

// Mock format utility used by the YAxis tick formatter.
vi.mock("@/lib/format", () => ({
  formatCredits: (c: number) => {
    if (c >= 1_000_000) return `${(c / 1_000_000).toFixed(1)}M`;
    if (c >= 1_000) return `${(c / 1_000).toFixed(1)}K`;
    return c.toLocaleString();
  },
}));

import { EarningsChart } from "@/components/operators/earnings-chart";
import { makeMonthlyEarning } from "@/__mocks__/test-wrappers";

describe("EarningsChart", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows empty state when earnings array is empty", () => {
    render(<EarningsChart earnings={[]} />);
    expect(screen.getByText("No earnings data yet")).toBeInTheDocument();
    expect(screen.queryByTestId("bar-chart")).not.toBeInTheDocument();
  });

  it("renders BarChart with correct data count", () => {
    const earnings = [
      makeMonthlyEarning({ month: "2026-01-01", total_credits: 500, block_count: 25 }),
      makeMonthlyEarning({ month: "2026-02-01", total_credits: 750, block_count: 30 }),
      makeMonthlyEarning({ month: "2026-03-01", total_credits: 300, block_count: 15 }),
    ] as any[];
    render(<EarningsChart earnings={earnings} />);

    const chart = screen.getByTestId("bar-chart");
    expect(chart).toBeInTheDocument();
    expect(chart.getAttribute("data-count")).toBe("3");
  });

  it("handles null total_credits (defaults to 0)", () => {
    const earnings = [
      makeMonthlyEarning({ month: "2026-01-01", total_credits: null }),
    ] as any[];
    // Should not throw — the component coalesces null to 0.
    render(<EarningsChart earnings={earnings} />);
    expect(screen.getByTestId("bar-chart")).toBeInTheDocument();
  });

  it("handles null block_count (defaults to 0)", () => {
    const earnings = [
      makeMonthlyEarning({ month: "2026-01-01", block_count: null }),
    ] as any[];
    // Should not throw — the component coalesces null to 0.
    render(<EarningsChart earnings={earnings} />);
    expect(screen.getByTestId("bar-chart")).toBeInTheDocument();
  });

  it('renders Bar with dataKey="credits"', () => {
    const earnings = [
      makeMonthlyEarning({ month: "2026-01-01" }),
    ] as any[];
    render(<EarningsChart earnings={earnings} />);

    const bar = screen.getByTestId("bar");
    expect(bar.getAttribute("data-datakey")).toBe("credits");
  });
});
