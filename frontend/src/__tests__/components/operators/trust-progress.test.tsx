/**
 * @file Tests for the TrustProgress operator trust level component
 * @module __tests__/components/operators/trust-progress
 *
 * Validates the TrustProgress visual progress bar that displays operator
 * trust advancement from level 1 (Newcomer) through level 4 (Core).
 * Tests cover label rendering, trust level name mapping, level fraction
 * display, and correct filling of progress bar segments.
 *
 * Trust levels:
 * - 1 = Newcomer (zinc)
 * - 2 = Proven (blue)
 * - 3 = Trusted (indigo)
 * - 4 = Core (amber)
 *
 * @see {@link ../../../components/operators/trust-progress} Source component
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock cn utility — concatenates truthy class name arguments with spaces.
vi.mock("@/lib/utils", () => ({
  cn: (...args: any[]) => args.filter(Boolean).join(" "),
}));

import { TrustProgress } from "@/components/operators/trust-progress";

describe("TrustProgress", () => {
  /** Verifies the "Trust Level:" label is rendered. */
  it('renders "Trust Level:" label', () => {
    render(<TrustProgress trustLevel={1} />);
    expect(screen.getByText(/Trust Level:/)).toBeInTheDocument();
  });

  /** Verifies trust level 1 shows the "Newcomer" label in the header. */
  it('shows "Newcomer" for trust level 1', () => {
    const { container } = render(<TrustProgress trustLevel={1} />);
    // The header label is inside a span.text-indigo-400 within the first row
    const headerLabel = container.querySelector(".text-indigo-400");
    expect(headerLabel).not.toBeNull();
    expect(headerLabel!.textContent).toBe("Newcomer");
  });

  /** Verifies trust level 2 shows the "Proven" label in the header. */
  it('shows "Proven" for trust level 2', () => {
    const { container } = render(<TrustProgress trustLevel={2} />);
    const headerLabel = container.querySelector(".text-indigo-400");
    expect(headerLabel).not.toBeNull();
    expect(headerLabel!.textContent).toBe("Proven");
  });

  /** Verifies trust level 3 shows the "Trusted" label in the header. */
  it('shows "Trusted" for trust level 3', () => {
    const { container } = render(<TrustProgress trustLevel={3} />);
    const headerLabel = container.querySelector(".text-indigo-400");
    expect(headerLabel).not.toBeNull();
    expect(headerLabel!.textContent).toBe("Trusted");
  });

  /** Verifies trust level 4 shows the "Core" label in the header. */
  it('shows "Core" for trust level 4', () => {
    const { container } = render(<TrustProgress trustLevel={4} />);
    const headerLabel = container.querySelector(".text-indigo-400");
    expect(headerLabel).not.toBeNull();
    expect(headerLabel!.textContent).toBe("Core");
  });

  /** Verifies the "Level X / 4" fraction text is displayed. */
  it('shows "Level X / 4" text', () => {
    render(<TrustProgress trustLevel={3} />);
    expect(screen.getByText("Level 3 / 4")).toBeInTheDocument();
  });

  /**
   * Verifies the correct number of progress bar segments are filled.
   * For trust level 2, the first two bars should have their colored class,
   * while the remaining two should show the unfilled bg-zinc-800 class.
   */
  it("fills correct number of progress bars for level 2", () => {
    const { container } = render(<TrustProgress trustLevel={2} />);
    // The progress bars are inside a "flex gap-1" container
    const barContainer = container.querySelector(".flex.gap-1");
    expect(barContainer).not.toBeNull();
    const bars = barContainer!.children;
    expect(bars).toHaveLength(4);

    // First two bars should be filled (have their color class, not bg-zinc-800)
    expect(bars[0].className).toContain("bg-zinc-500"); // Level 1 color
    expect(bars[1].className).toContain("bg-blue-500"); // Level 2 color

    // Last two bars should be unfilled (bg-zinc-800)
    expect(bars[2].className).toContain("bg-zinc-800");
    expect(bars[3].className).toContain("bg-zinc-800");
  });
});
