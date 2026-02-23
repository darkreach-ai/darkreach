/**
 * @file Tests for the Earnings page
 * @module __tests__/pages/earnings
 *
 * Validates the Earnings dashboard page at `/earnings`, which shows an
 * operator's monthly earnings chart, summary stat cards (Total Credits,
 * Blocks Completed, Rank), trust level progress bar, and paginated credit
 * transaction history. Data is fetched from the JWT-authed
 * `/api/v1/operators/me/credits` and `/api/v1/operators/me/earnings` endpoints.
 *
 * Tests verify: page heading, loading skeleton state, stat card rendering
 * (credits, blocks, rank), null rank handling, TrustProgress component
 * presence, and EarningsHistoryTable component presence.
 *
 * @see {@link ../../app/earnings/page} Source page
 * @see {@link ../../hooks/use-earnings} Earnings data hook
 * @see {@link ../../hooks/use-operator-resources} Operator stats hook
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// ── Mutable mock return values ──────────────────────────────────

let mockEarningsLoading = false;
let mockStatsLoading = false;
let mockStats: {
  username: string;
  credit: number;
  primes_found: number;
  trust_level: number | null;
  rank: number | null;
} | null = {
  username: "test",
  credit: 1500,
  primes_found: 42,
  trust_level: 2,
  rank: 5,
};

// ── Mock hooks ──────────────────────────────────────────────────

vi.mock("@/hooks/use-earnings", () => ({
  useEarnings: () => ({
    credits: [],
    earnings: [
      { month: "2026-01-01", total_credits: 500, block_count: 25 },
    ],
    loading: mockEarningsLoading,
  }),
}));

vi.mock("@/hooks/use-operator-resources", () => ({
  useOperatorStats: () => ({
    stats: mockStats,
    loading: mockStatsLoading,
  }),
}));

// ── Mock child components ───────────────────────────────────────

vi.mock("@/components/operators/earnings-chart", () => ({
  EarningsChart: ({ earnings }: { earnings: unknown[] }) => (
    <div data-testid="earnings-chart">
      EarningsChart ({earnings.length} months)
    </div>
  ),
}));

vi.mock("@/components/operators/earnings-history-table", () => ({
  EarningsHistoryTable: ({ credits }: { credits: unknown[] }) => (
    <div data-testid="earnings-history-table">
      EarningsHistoryTable ({credits.length} rows)
    </div>
  ),
}));

vi.mock("@/components/operators/trust-progress", () => ({
  TrustProgress: ({ trustLevel }: { trustLevel: number }) => (
    <div data-testid="trust-progress">Trust Level: {trustLevel}</div>
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
  Coins: () => <span data-testid="icon-coins" />,
  Blocks: () => <span data-testid="icon-blocks" />,
  Shield: () => <span data-testid="icon-shield" />,
  TrendingUp: () => <span data-testid="icon-trending" />,
}));

vi.mock("@/lib/format", () => ({
  API_BASE: "http://localhost:3000",
  formatCredits: (x: number) => x.toLocaleString(),
  numberWithCommas: (x: number) =>
    x.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ","),
}));

import EarningsPage from "@/app/earnings/page";

// ── Tests ───────────────────────────────────────────────────────

describe("EarningsPage", () => {
  it("renders 'Earnings' title", () => {
    render(<EarningsPage />);
    expect(screen.getByText("Earnings")).toBeInTheDocument();
  });

  it("shows skeleton loaders when loading", () => {
    mockEarningsLoading = true;
    mockStatsLoading = true;
    const { container, unmount } = render(<EarningsPage />);
    // When loading, the page renders animated pulse divs instead of components
    const pulses = container.querySelectorAll(".animate-pulse");
    expect(pulses.length).toBeGreaterThanOrEqual(1);
    unmount();
    mockEarningsLoading = false;
    mockStatsLoading = false;
  });

  it("renders Total Credits stat card", () => {
    render(<EarningsPage />);
    expect(screen.getByText("Total Credits")).toBeInTheDocument();
  });

  it("renders Blocks Completed stat card", () => {
    render(<EarningsPage />);
    expect(screen.getByText(/Blocks Completed/)).toBeInTheDocument();
  });

  it("renders Rank stat card with '#5'", () => {
    render(<EarningsPage />);
    expect(screen.getByText("#5")).toBeInTheDocument();
  });

  it("handles null rank (shows dash)", () => {
    const originalStats = mockStats;
    mockStats = {
      username: "test",
      credit: 1500,
      primes_found: 42,
      trust_level: 2,
      rank: null,
    };
    const { unmount } = render(<EarningsPage />);
    // When rank is null, the page renders a dash character
    const rankCard = screen.getByText("Rank").closest("div")!;
    expect(rankCard.parentElement!.textContent).toContain("\u2014");
    unmount();
    mockStats = originalStats;
  });

  it("renders TrustProgress component", () => {
    render(<EarningsPage />);
    expect(screen.getByTestId("trust-progress")).toBeInTheDocument();
  });

  it("renders EarningsHistoryTable component", () => {
    render(<EarningsPage />);
    expect(
      screen.getByTestId("earnings-history-table")
    ).toBeInTheDocument();
  });
});
