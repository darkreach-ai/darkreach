/**
 * @file Tests for the Marketplace page
 * @module __tests__/pages/marketplace
 *
 * Validates the Compute Marketplace page at `/marketplace`, a public page
 * that shows active search forms available for operator contribution and
 * credit conversion rates. No authentication is required. Data is fetched
 * from `/api/v1/marketplace/forms` and `/api/resources/rates`.
 *
 * Tests verify: page heading, subtitle text, loading skeleton state,
 * empty state when no forms are available, FormShowcaseCard rendering
 * for each form, and RateTable rendering with credit rates.
 *
 * @see {@link ../../app/marketplace/page} Source page
 * @see {@link ../../hooks/use-marketplace} Marketplace data hook
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// ── Mutable mock return values ──────────────────────────────────

let mockLoading = false;
let mockForms: Array<{
  form: string;
  job_count: number;
  total_blocks: number;
  completed_blocks: number;
}> = [
  { form: "factorial", job_count: 3, total_blocks: 100, completed_blocks: 45 },
  { form: "kbn", job_count: 1, total_blocks: 50, completed_blocks: 10 },
];
let mockRates: Array<{
  resource_type: string;
  credits_per_unit: number;
  unit_label: string;
  updated_at: string;
}> = [
  {
    resource_type: "cpu_core_hours",
    credits_per_unit: 10,
    unit_label: "core-hour",
    updated_at: "2026-02-20",
  },
];

// ── Mock hooks ──────────────────────────────────────────────────

vi.mock("@/hooks/use-marketplace", () => ({
  useMarketplace: () => ({
    forms: mockForms,
    rates: mockRates,
    loading: mockLoading,
  }),
}));

// ── Mock child components ───────────────────────────────────────

vi.mock("@/components/operators/form-showcase-card", () => ({
  FormShowcaseCard: ({
    stat,
    creditRate,
  }: {
    stat: { form: string };
    creditRate?: number;
  }) => (
    <div data-testid={`form-card-${stat.form}`}>
      FormShowcaseCard: {stat.form}
      {creditRate != null && ` (${creditRate} credits)`}
    </div>
  ),
}));

vi.mock("@/components/operators/rate-table", () => ({
  RateTable: ({ rates }: { rates: unknown[] }) => (
    <div data-testid="rate-table">RateTable ({rates.length} rates)</div>
  ),
}));

vi.mock("@/components/view-header", () => ({
  ViewHeader: ({
    title,
    subtitle,
  }: {
    title: string;
    subtitle?: string;
  }) => (
    <div data-testid="view-header">
      <h1>{title}</h1>
      {subtitle && <p>{subtitle}</p>}
    </div>
  ),
}));

vi.mock("lucide-react", () => ({
  Store: () => <span data-testid="icon-store" />,
  Zap: () => <span data-testid="icon-zap" />,
}));

import MarketplacePage from "@/app/marketplace/page";

// ── Tests ───────────────────────────────────────────────────────

describe("MarketplacePage", () => {
  it("renders 'Compute Marketplace' title", () => {
    render(<MarketplacePage />);
    expect(screen.getByText("Compute Marketplace")).toBeInTheDocument();
  });

  it("renders subtitle text", () => {
    render(<MarketplacePage />);
    expect(
      screen.getByText(
        "Browse active search forms and earn credits by contributing compute power"
      )
    ).toBeInTheDocument();
  });

  it("shows loading skeleton when loading", () => {
    mockLoading = true;
    const { container, unmount } = render(<MarketplacePage />);
    const pulses = container.querySelectorAll(".animate-pulse");
    expect(pulses.length).toBeGreaterThanOrEqual(1);
    unmount();
    mockLoading = false;
  });

  it("shows empty state when no forms", () => {
    const savedForms = mockForms;
    mockForms = [];
    const { unmount } = render(<MarketplacePage />);
    expect(
      screen.getByText("No active search forms at this time")
    ).toBeInTheDocument();
    unmount();
    mockForms = savedForms;
  });

  it("renders FormShowcaseCard for each form", () => {
    render(<MarketplacePage />);
    expect(screen.getByTestId("form-card-factorial")).toBeInTheDocument();
    expect(screen.getByTestId("form-card-kbn")).toBeInTheDocument();
  });

  it("renders RateTable with rates", () => {
    render(<MarketplacePage />);
    expect(screen.getByTestId("rate-table")).toBeInTheDocument();
    expect(
      screen.getByText("RateTable (1 rates)")
    ).toBeInTheDocument();
  });
});
