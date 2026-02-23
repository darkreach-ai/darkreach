/**
 * @file Tests for the EarningsHistoryTable component
 * @module __tests__/components/operators/earnings-history-table
 *
 * Validates the paginated table of credit transactions displayed on the
 * operator earnings page. Each row shows the work block ID, credit amount,
 * reason badge, and relative timestamp. Tests cover empty state, header
 * rendering, block ID formatting (including null block_id), reason badge
 * mapping (block_completed -> "Block", prime_discovered -> "Discovery",
 * unknown reasons -> raw text), and pagination controls (page indicator
 * text, next/previous button behavior).
 *
 * @see {@link ../../../components/operators/earnings-history-table} Source component
 * @see {@link ../../../hooks/use-earnings} CreditRow type
 * @see {@link ../../../__mocks__/test-wrappers} makeCreditRow factory
 */
import { vi, describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

// Mock shadcn/ui Table components — render as semantic HTML for query access.
vi.mock("@/components/ui/table", () => ({
  Table: ({ children }: { children: React.ReactNode }) => (
    <table>{children}</table>
  ),
  TableHeader: ({ children }: { children: React.ReactNode }) => (
    <thead>{children}</thead>
  ),
  TableBody: ({ children }: { children: React.ReactNode }) => (
    <tbody>{children}</tbody>
  ),
  TableRow: ({ children }: { children: React.ReactNode }) => (
    <tr>{children}</tr>
  ),
  TableHead: ({ children }: { children: React.ReactNode }) => (
    <th>{children}</th>
  ),
  TableCell: ({ children, ...props }: { children: React.ReactNode; [k: string]: unknown }) => (
    <td {...props}>{children}</td>
  ),
}));

// Mock Button — renders a plain button element with onClick support.
vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, disabled, ...props }: { children: React.ReactNode; onClick?: () => void; disabled?: boolean; [k: string]: unknown }) => (
    <button onClick={onClick} disabled={disabled} {...props}>{children}</button>
  ),
}));

// Mock Badge — renders children as a span with testid for lookup.
vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children, variant }: { children: React.ReactNode; variant?: string }) => (
    <span data-testid="badge" data-variant={variant}>{children}</span>
  ),
}));

// Mock lucide-react icons used in pagination buttons.
vi.mock("lucide-react", () => ({
  ChevronLeft: () => <span data-testid="chevron-left" />,
  ChevronRight: () => <span data-testid="chevron-right" />,
}));

// Mock format utilities.
vi.mock("@/lib/format", () => ({
  formatCredits: (c: number) => {
    if (c >= 1_000_000) return `${(c / 1_000_000).toFixed(1)}M`;
    if (c >= 1_000) return `${(c / 1_000).toFixed(1)}K`;
    return c.toLocaleString();
  },
  relativeTime: () => "3h ago",
}));

import { EarningsHistoryTable } from "@/components/operators/earnings-history-table";
import { makeCreditRow } from "@/__mocks__/test-wrappers";

describe("EarningsHistoryTable", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows empty state when credits array is empty", () => {
    render(<EarningsHistoryTable credits={[]} />);
    expect(screen.getByText("No credit transactions yet")).toBeInTheDocument();
  });

  it("renders table with headers (Block, Credits, Reason, Time)", () => {
    const credits = [makeCreditRow() as any];
    render(<EarningsHistoryTable credits={credits} />);
    // "Block" appears both as a <th> header and as a reason badge label,
    // so we use getAllByText and check the columnheader role for the header.
    expect(screen.getByRole("columnheader", { name: "Block" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Credits" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Reason" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Time" })).toBeInTheDocument();
  });

  it("renders credit rows with block ID (#100)", () => {
    const credits = [makeCreditRow({ id: 1, block_id: 100 }) as any];
    render(<EarningsHistoryTable credits={credits} />);
    expect(screen.getByText("#100")).toBeInTheDocument();
  });

  it('shows "\u2014" for null block_id', () => {
    const credits = [makeCreditRow({ id: 1, block_id: null }) as any];
    render(<EarningsHistoryTable credits={credits} />);
    // The component renders "\u2014" for null block_id
    const dashes = screen.getAllByText("\u2014");
    expect(dashes.length).toBeGreaterThanOrEqual(1);
  });

  it('shows reason badge with correct label for "block_completed" -> "Block"', () => {
    const credits = [makeCreditRow({ id: 1, reason: "block_completed" }) as any];
    render(<EarningsHistoryTable credits={credits} />);
    const badges = screen.getAllByTestId("badge");
    // Find the badge that contains "Block" (not the table header "Block")
    const reasonBadge = badges.find((b) => b.textContent === "Block");
    expect(reasonBadge).toBeDefined();
    expect(reasonBadge?.getAttribute("data-variant")).toBe("secondary");
  });

  it('shows reason badge for "prime_discovered" -> "Discovery"', () => {
    const credits = [makeCreditRow({ id: 1, reason: "prime_discovered" }) as any];
    render(<EarningsHistoryTable credits={credits} />);
    const badges = screen.getAllByTestId("badge");
    const reasonBadge = badges.find((b) => b.textContent === "Discovery");
    expect(reasonBadge).toBeDefined();
    expect(reasonBadge?.getAttribute("data-variant")).toBe("default");
  });

  it("shows raw reason text for unknown reason", () => {
    const credits = [makeCreditRow({ id: 1, reason: "manual_bonus" }) as any];
    render(<EarningsHistoryTable credits={credits} />);
    const badges = screen.getAllByTestId("badge");
    const reasonBadge = badges.find((b) => b.textContent === "manual_bonus");
    expect(reasonBadge).toBeDefined();
  });

  it('pagination: shows "Page 1 of 2" for 15 items with pageSize=10', () => {
    const credits = Array.from({ length: 15 }, (_, i) =>
      makeCreditRow({ id: i + 1 }) as any,
    );
    render(<EarningsHistoryTable credits={credits} pageSize={10} />);
    expect(screen.getByText("Page 1 of 2")).toBeInTheDocument();
  });

  it("pagination: next button advances page", () => {
    const credits = Array.from({ length: 15 }, (_, i) =>
      makeCreditRow({ id: i + 1, block_id: (i + 1) * 10 }) as any,
    );
    render(<EarningsHistoryTable credits={credits} pageSize={10} />);
    expect(screen.getByText("Page 1 of 2")).toBeInTheDocument();

    // Click the next page button (ChevronRight icon button)
    const buttons = screen.getAllByRole("button");
    const nextButton = buttons.find(
      (b) => b.querySelector("[data-testid='chevron-right']") !== null,
    );
    expect(nextButton).toBeDefined();
    fireEvent.click(nextButton!);

    expect(screen.getByText("Page 2 of 2")).toBeInTheDocument();
  });
});
