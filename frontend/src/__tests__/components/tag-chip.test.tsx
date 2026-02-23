/**
 * @file Tests for the TagChip component and tagCategoryColor utility
 * @module __tests__/components/tag-chip
 *
 * Validates the TagChip badge component that renders color-coded tags for
 * prime search results. Tags are categorized into structural (search forms),
 * proof (deterministic/probabilistic/prp-only), verification (verified-xxx),
 * record (world-record), and property (twin-prime, safe-prime, etc.) groups,
 * each receiving distinct Tailwind color classes.
 *
 * Also tests the tagCategoryColor utility that returns hex color codes for
 * chart rendering, matching tag categories to the same visual scheme.
 *
 * @see {@link ../../components/tag-chip} Source component
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

// Mock the shadcn Badge to a simple span for assertion on className and children.
vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children, className, onClick, ...props }: any) => (
    <span data-testid="badge" className={className} onClick={onClick} {...props}>
      {children}
    </span>
  ),
}));

// Mock cn utility to concatenate truthy class strings, matching real behavior.
vi.mock("@/lib/utils", () => ({ cn: (...args: any[]) => args.filter(Boolean).join(" ") }));

import { TagChip, tagCategoryColor } from "@/components/tag-chip";

// ── TagChip component ────────────────────────────────────────────

describe("TagChip", () => {
  /** Verifies the tag text is rendered inside the badge. */
  it("renders the tag text", () => {
    render(<TagChip tag="factorial" />);
    expect(screen.getByTestId("badge")).toHaveTextContent("factorial");
  });

  /** Verifies that structural form tags (e.g. factorial, kbn) receive the slate color style. */
  it("applies structural style to form tags like factorial", () => {
    render(<TagChip tag="factorial" />);
    expect(screen.getByTestId("badge").className).toContain("slate");
  });

  /** Verifies that the kbn structural form also receives the slate color style. */
  it("applies structural style to form tags like kbn", () => {
    render(<TagChip tag="kbn" />);
    expect(screen.getByTestId("badge").className).toContain("slate");
  });

  /** Verifies the deterministic proof tag receives the green color style. */
  it("applies green style to deterministic proof tag", () => {
    render(<TagChip tag="deterministic" />);
    const className = screen.getByTestId("badge").className;
    expect(className).toContain("green");
  });

  /** Verifies the prp-only proof tag receives the orange color style. */
  it("applies orange style to prp-only proof tag", () => {
    render(<TagChip tag="prp-only" />);
    const className = screen.getByTestId("badge").className;
    expect(className).toContain("orange");
  });

  /** Verifies the probabilistic proof tag receives the default yellow proof style. */
  it("applies yellow style to probabilistic proof tag", () => {
    render(<TagChip tag="probabilistic" />);
    const className = screen.getByTestId("badge").className;
    expect(className).toContain("yellow");
  });

  /** Verifies verification tags (verified-xxx) receive the emerald color style. */
  it("applies verification style to verified-xxx tags", () => {
    render(<TagChip tag="verified-bpsw" />);
    const className = screen.getByTestId("badge").className;
    expect(className).toContain("emerald");
  });

  /** Verifies record tags (world-record) receive the amber color style. */
  it("applies record style to world-record tag", () => {
    render(<TagChip tag="world-record" />);
    const className = screen.getByTestId("badge").className;
    expect(className).toContain("amber");
  });

  /** Verifies property tags (twin-prime) receive the indigo color style. */
  it("applies property style to twin-prime tag", () => {
    render(<TagChip tag="twin-prime" />);
    const className = screen.getByTestId("badge").className;
    expect(className).toContain("indigo");
  });

  /** Verifies the onClick handler fires when a clickable TagChip is clicked. */
  it("fires onClick handler when provided", () => {
    const handler = vi.fn();
    render(<TagChip tag="factorial" onClick={handler} />);
    fireEvent.click(screen.getByTestId("badge"));
    expect(handler).toHaveBeenCalledOnce();
  });

  /** Verifies cursor-pointer class is added when onClick is provided. */
  it("adds cursor-pointer class when onClick is provided", () => {
    render(<TagChip tag="factorial" onClick={() => {}} />);
    expect(screen.getByTestId("badge").className).toContain("cursor-pointer");
  });

  /** Verifies cursor-pointer class is absent when no onClick handler is given. */
  it("does not add cursor-pointer class when onClick is absent", () => {
    render(<TagChip tag="factorial" />);
    expect(screen.getByTestId("badge").className).not.toContain("cursor-pointer");
  });
});

// ── tagCategoryColor utility ─────────────────────────────────────

describe("tagCategoryColor", () => {
  /** Verifies structural tags return the slate hex color. */
  it("returns slate hex for structural tags", () => {
    expect(tagCategoryColor("factorial")).toBe("#64748b");
  });

  /** Verifies proof tags return the yellow hex color. */
  it("returns yellow hex for proof tags", () => {
    expect(tagCategoryColor("deterministic")).toBe("#eab308");
  });

  /** Verifies property tags return the indigo hex color. */
  it("returns indigo hex for property tags", () => {
    expect(tagCategoryColor("twin-prime")).toBe("#6366f1");
  });

  /** Verifies verification tags return the emerald hex color. */
  it("returns emerald hex for verification tags", () => {
    expect(tagCategoryColor("verified-bpsw")).toBe("#10b981");
  });

  /** Verifies record tags return the amber hex color. */
  it("returns amber hex for record tags", () => {
    expect(tagCategoryColor("world-record")).toBe("#f59e0b");
  });

  /** Verifies unknown tags fall back to the structural (slate) color. */
  it("returns structural color for unknown tags", () => {
    expect(tagCategoryColor("unknown-tag")).toBe("#64748b");
  });
});
