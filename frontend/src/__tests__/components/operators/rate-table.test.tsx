/**
 * @file Tests for the RateTable credit rates component
 * @module __tests__/components/operators/rate-table
 *
 * Validates the RateTable component that displays credit conversion rates
 * for each resource type. Tests cover the empty state message, table header
 * labels, resource type formatting (underscores replaced with spaces),
 * locale-formatted credits_per_unit values, and unit label rendering.
 *
 * UI table primitives from shadcn/ui are mocked to simple HTML table elements
 * so tests can query standard DOM structure without the component library.
 *
 * @see {@link ../../../components/operators/rate-table} Source component
 * @see {@link ../../../hooks/use-marketplace} CreditRate type definition
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock shadcn/ui table components to plain HTML table elements.
vi.mock("@/components/ui/table", () => ({
  Table: ({ children }: any) => <table>{children}</table>,
  TableHeader: ({ children }: any) => <thead>{children}</thead>,
  TableBody: ({ children }: any) => <tbody>{children}</tbody>,
  TableRow: ({ children }: any) => <tr>{children}</tr>,
  TableHead: ({ children }: any) => <th>{children}</th>,
  TableCell: ({ children, ...props }: any) => <td {...props}>{children}</td>,
}));

import { RateTable } from "@/components/operators/rate-table";
import { makeCreditRate } from "@/__mocks__/test-wrappers";

describe("RateTable", () => {
  /** Verifies the empty state message when no rates are configured. */
  it("renders empty state message when rates array is empty", () => {
    render(<RateTable rates={[]} />);
    expect(
      screen.getByText("No credit rates configured yet")
    ).toBeInTheDocument();
  });

  /** Verifies all three table column headers are rendered. */
  it("renders table headers", () => {
    const rates = [makeCreditRate()];
    render(<RateTable rates={rates as any} />);
    expect(screen.getByText("Resource Type")).toBeInTheDocument();
    expect(screen.getByText("Credits per Unit")).toBeInTheDocument();
    expect(screen.getByText("Unit")).toBeInTheDocument();
  });

  /** Verifies resource_type underscores are replaced with spaces for display. */
  it("renders rate rows with formatted resource type", () => {
    const rates = [
      makeCreditRate({ resource_type: "cpu_core_hours" }),
      makeCreditRate({ resource_type: "gpu_hours" }),
    ];
    render(<RateTable rates={rates as any} />);
    expect(screen.getByText("cpu core hours")).toBeInTheDocument();
    expect(screen.getByText("gpu hours")).toBeInTheDocument();
  });

  /** Verifies credits_per_unit values use toLocaleString formatting. */
  it("formats credits_per_unit with toLocaleString", () => {
    const rates = [makeCreditRate({ credits_per_unit: 1500 })];
    render(<RateTable rates={rates as any} />);
    // toLocaleString() formats 1500 — result depends on locale but should contain the digits
    expect(screen.getByText((1500).toLocaleString())).toBeInTheDocument();
  });

  /** Verifies unit_label text is rendered for each rate row. */
  it("renders unit_label for each rate", () => {
    const rates = [
      makeCreditRate({ unit_label: "core-hour" }),
      makeCreditRate({
        resource_type: "gpu_hours",
        unit_label: "GPU-hour",
      }),
    ];
    render(<RateTable rates={rates as any} />);
    expect(screen.getByText("core-hour")).toBeInTheDocument();
    expect(screen.getByText("GPU-hour")).toBeInTheDocument();
  });
});
