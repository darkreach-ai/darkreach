/**
 * @file Tests for the Browse page
 * @module __tests__/pages/browse
 *
 * Validates the Browse page at `/browse`, which provides a sortable,
 * infinite-scroll table of all discovered primes. Tests verify page heading,
 * subtitle with total prime count, filter controls, table rendering with
 * prime data, sortable column headers, and form badges.
 *
 * @see {@link ../../app/browse/page} Source page
 * @see {@link ../../hooks/use-primes} usePrimes hook (data provider)
 * @see {@link ../../hooks/use-stats} useStats hook (total count)
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

const mockFetchPrimeDetail = vi.fn();
const mockClearSelectedPrime = vi.fn();
const mockResetAndFetch = vi.fn();
const mockFetchNextPage = vi.fn();

vi.mock("@/hooks/use-primes", () => ({
  usePrimes: () => ({
    primes: {
      primes: [
        {
          id: 1,
          form: "factorial",
          expression: "5!+1",
          digits: 3,
          found_at: "2026-01-01T00:00:00Z",
          proof_method: "deterministic",
          verified: true,
          verified_at: null,
          verification_method: null,
          verification_tier: null,
        },
        {
          id: 2,
          form: "kbn",
          expression: "3*2^10+1",
          digits: 4,
          found_at: "2026-01-02T00:00:00Z",
          proof_method: "probabilistic",
          verified: false,
          verified_at: null,
          verification_method: null,
          verification_tier: null,
        },
      ],
      total: 2,
      offset: 0,
      limit: 50,
    },
    fetchPrimeDetail: mockFetchPrimeDetail,
    selectedPrime: null,
    clearSelectedPrime: mockClearSelectedPrime,
    resetAndFetch: mockResetAndFetch,
    fetchNextPage: mockFetchNextPage,
    hasMore: false,
    isLoadingMore: false,
    isInitialLoading: false,
  }),
}));

vi.mock("@/hooks/use-stats", () => ({
  useStats: () => ({
    stats: {
      total: 42,
      by_form: [
        { form: "factorial", count: 30 },
        { form: "kbn", count: 12 },
      ],
      largest_digits: 1000,
      largest_expression: "100!+1",
    },
  }),
}));

vi.mock("@/components/view-header", () => ({
  ViewHeader: ({
    title,
    subtitle,
  }: {
    title: string;
    subtitle: string;
  }) => (
    <div data-testid="view-header">
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
  ),
}));

vi.mock("@/components/prime-detail-dialog", () => ({
  PrimeDetailDialog: () => null,
}));

vi.mock("next/link", () => ({
  default: ({
    children,
    href,
  }: {
    children: React.ReactNode;
    href: string;
  }) => <a href={href}>{children}</a>,
}));

vi.mock("@/lib/format", () => ({
  API_BASE: "http://localhost:3000",
  numberWithCommas: (x: number) =>
    x.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ","),
  formatTime: (t: string) => t,
  formToSlug: (f: string) => f.toLowerCase(),
  formLabels: {
    factorial: "Factorial",
    kbn: "k\u00b7b^n",
  },
  relativeTime: (t: string) => t,
}));

import BrowsePage from "@/app/browse/page";

describe("BrowsePage", () => {
  it("renders without crashing", () => {
    render(<BrowsePage />);
    expect(screen.getByText("Browse")).toBeInTheDocument();
  });

  it("shows total prime count in subtitle", () => {
    render(<BrowsePage />);
    const header = screen.getByTestId("view-header");
    expect(header).toHaveTextContent("2 primes");
  });

  it("renders filter controls", () => {
    render(<BrowsePage />);
    expect(screen.getByPlaceholderText("Search expressions...")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Min digits")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Max digits")).toBeInTheDocument();
  });

  it("renders table with prime expressions", () => {
    render(<BrowsePage />);
    expect(screen.getByText("5!+1")).toBeInTheDocument();
    expect(screen.getByText("3*2^10+1")).toBeInTheDocument();
  });

  it("renders sortable column headers", () => {
    render(<BrowsePage />);
    expect(screen.getByText("Expression")).toBeInTheDocument();
    expect(screen.getByText("Digits")).toBeInTheDocument();
    expect(screen.getByText("Found")).toBeInTheDocument();
  });

  it("renders form badges on rows", () => {
    render(<BrowsePage />);
    expect(screen.getByText("Factorial")).toBeInTheDocument();
    expect(screen.getByText("k\u00b7b^n")).toBeInTheDocument();
  });
});
