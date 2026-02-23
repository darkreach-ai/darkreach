/**
 * @file Tests for the TagDistribution chart component
 * @module __tests__/components/charts/tag-distribution
 *
 * Validates the horizontal bar chart that shows tag frequency across all
 * discovered primes. Tags are color-coded by category (structural, proof,
 * property, verification, record) via the tagCategoryColor helper.
 *
 * Recharts is mocked to simple div elements since jsdom cannot render SVG.
 * Tests focus on the empty state, chart rendering, top-20 limit, and
 * category-based coloring.
 *
 * @see {@link ../../../components/charts/tag-distribution} Source component
 * @see {@link ../../../components/tag-chip} tagCategoryColor helper
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock Recharts to avoid SVG rendering in jsdom.
vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  BarChart: ({ children, data }: { children: React.ReactNode; data: any[] }) => (
    <div data-testid="bar-chart" data-count={data.length}>
      {children}
    </div>
  ),
  Bar: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="bar">{children}</div>
  ),
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  Cell: ({ fill }: { fill: string }) => (
    <div data-testid="cell" data-fill={fill} />
  ),
}));

// Mock Card components from shadcn/ui.
vi.mock("@/components/ui/card", () => ({
  Card: ({ children }: any) => <div data-testid="card">{children}</div>,
  CardContent: ({ children }: any) => <div>{children}</div>,
  CardHeader: ({ children }: any) => <div>{children}</div>,
  CardTitle: ({ children }: any) => <div>{children}</div>,
}));

// Mock tagCategoryColor to return predictable colors for testing.
vi.mock("@/components/tag-chip", () => ({
  tagCategoryColor: (tag: string) => {
    const colors: Record<string, string> = {
      factorial: "#64748b",
      deterministic: "#eab308",
      "safe-prime": "#6366f1",
      "verified-t3": "#10b981",
      "world-record": "#f59e0b",
    };
    return colors[tag] || "#64748b";
  },
}));

import { TagDistribution } from "@/components/charts/tag-distribution";

describe("TagDistribution", () => {
  /** Verifies the component renders nothing when there are no tags. */
  it("renders nothing when no tags", () => {
    const { container } = render(<TagDistribution data={[]} />);
    expect(container.innerHTML).toBe("");
  });

  /** Verifies the chart renders with tag data and title. */
  it("renders chart with tag data", () => {
    const data = [
      { tag: "factorial", count: 42 },
      { tag: "kbn", count: 30 },
      { tag: "deterministic", count: 25 },
    ];
    render(<TagDistribution data={data} />);
    expect(screen.getByText("Tag distribution")).toBeInTheDocument();
    expect(screen.getByTestId("bar-chart")).toBeInTheDocument();
  });

  /** Verifies only the top 20 tags are passed to the chart when data exceeds 20. */
  it("limits to top 20 tags", () => {
    const data = Array.from({ length: 30 }, (_, i) => ({
      tag: `tag-${i}`,
      count: 30 - i,
    }));
    render(<TagDistribution data={data} />);
    const barChart = screen.getByTestId("bar-chart");
    expect(barChart.getAttribute("data-count")).toBe("20");
  });

  /** Verifies each tag cell receives a fill color from tagCategoryColor. */
  it("uses category colors for cells", () => {
    const data = [
      { tag: "factorial", count: 10 },
      { tag: "deterministic", count: 5 },
      { tag: "world-record", count: 3 },
    ];
    render(<TagDistribution data={data} />);
    const cells = screen.getAllByTestId("cell");
    expect(cells).toHaveLength(3);
    // structural tag "factorial" gets #64748b
    expect(cells[0].getAttribute("data-fill")).toBe("#64748b");
    // proof tag "deterministic" gets #eab308
    expect(cells[1].getAttribute("data-fill")).toBe("#eab308");
    // record tag "world-record" gets #f59e0b
    expect(cells[2].getAttribute("data-fill")).toBe("#f59e0b");
  });
});
