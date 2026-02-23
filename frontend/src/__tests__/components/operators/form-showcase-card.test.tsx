/**
 * @file Tests for the FormShowcaseCard marketplace component
 * @module __tests__/components/operators/form-showcase-card
 *
 * Validates the FormShowcaseCard component that displays search form statistics
 * in the marketplace grid. Tests cover form label resolution (from formLabels
 * map with raw-name fallback), job count with singular/plural handling, block
 * progress percentage computation, division-by-zero protection for zero
 * total_blocks, null field handling, and conditional credit rate rendering.
 *
 * Mocks Card/Badge UI primitives and the format utility module to isolate
 * the component's logic from presentational wrappers and locale formatting.
 *
 * @see {@link ../../../components/operators/form-showcase-card} Source component
 * @see {@link ../../../hooks/use-marketplace} ActiveFormStat type definition
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock shadcn/ui Card and Badge to simple DOM elements for querying.
vi.mock("@/components/ui/card", () => ({
  Card: ({ children, ...props }: any) => (
    <div data-testid="card" {...props}>
      {children}
    </div>
  ),
  CardContent: ({ children, ...props }: any) => (
    <div {...props}>{children}</div>
  ),
  CardHeader: ({ children, ...props }: any) => (
    <div {...props}>{children}</div>
  ),
  CardTitle: ({ children, ...props }: any) => (
    <div {...props}>{children}</div>
  ),
}));

vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children, ...props }: any) => <span {...props}>{children}</span>,
}));

// Mock format utilities with a subset of formLabels and locale-based numberWithCommas.
vi.mock("@/lib/format", () => ({
  formLabels: { factorial: "Factorial", kbn: "k*b^n" } as Record<
    string,
    string
  >,
  numberWithCommas: (n: number) => n.toLocaleString(),
}));

import { FormShowcaseCard } from "@/components/operators/form-showcase-card";
import { makeActiveFormStat } from "@/__mocks__/test-wrappers";

describe("FormShowcaseCard", () => {
  /** Verifies the human-readable label from formLabels is displayed. */
  it("renders form label from formLabels map", () => {
    const stat = makeActiveFormStat({ form: "factorial" });
    render(<FormShowcaseCard stat={stat as any} />);
    expect(screen.getByText("Factorial")).toBeInTheDocument();
  });

  /** Verifies the raw form name is used when it is not in formLabels. */
  it("falls back to raw form name when not in formLabels", () => {
    const stat = makeActiveFormStat({ form: "wagstaff" });
    render(<FormShowcaseCard stat={stat as any} />);
    expect(screen.getByText("wagstaff")).toBeInTheDocument();
  });

  /** Verifies job count badge shows correct singular/plural form. */
  it("shows job count with correct singular/plural", () => {
    const statSingular = makeActiveFormStat({ job_count: 1 });
    const { unmount } = render(<FormShowcaseCard stat={statSingular as any} />);
    expect(screen.getByText("1 job")).toBeInTheDocument();
    unmount();

    const statPlural = makeActiveFormStat({ job_count: 5 });
    render(<FormShowcaseCard stat={statPlural as any} />);
    expect(screen.getByText("5 jobs")).toBeInTheDocument();
  });

  /** Verifies the progress percentage is computed as (completed/total * 100), rounded. */
  it("shows progress percentage", () => {
    const stat = makeActiveFormStat({
      total_blocks: 200,
      completed_blocks: 150,
    });
    render(<FormShowcaseCard stat={stat as any} />);
    // 150/200 = 75%
    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  /** Verifies zero total_blocks does not cause division by zero — shows 0%. */
  it("handles zero total_blocks without division by zero", () => {
    const stat = makeActiveFormStat({
      total_blocks: 0,
      completed_blocks: 0,
    });
    render(<FormShowcaseCard stat={stat as any} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  /** Verifies null fields (job_count, total_blocks, completed_blocks) default to 0. */
  it("handles null fields gracefully", () => {
    const stat = makeActiveFormStat({
      job_count: null,
      total_blocks: null,
      completed_blocks: null,
    });
    render(<FormShowcaseCard stat={stat as any} />);
    // job_count null defaults to 0 → "0 jobs"
    expect(screen.getByText("0 jobs")).toBeInTheDocument();
    // percentage should be 0% since total is 0
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  /** Verifies the credit rate section appears when creditRate prop is provided. */
  it("shows credit rate when creditRate prop is provided", () => {
    const stat = makeActiveFormStat();
    render(<FormShowcaseCard stat={stat as any} creditRate={25} />);
    expect(screen.getByText("25")).toBeInTheDocument();
    expect(screen.getByText(/credits\/core-hour/)).toBeInTheDocument();
  });

  /** Verifies the credit rate section is absent when creditRate is undefined. */
  it("hides credit rate section when creditRate is undefined", () => {
    const stat = makeActiveFormStat();
    render(<FormShowcaseCard stat={stat as any} />);
    expect(screen.queryByText(/credits\/core-hour/)).not.toBeInTheDocument();
  });
});
