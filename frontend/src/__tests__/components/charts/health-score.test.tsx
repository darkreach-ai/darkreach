/**
 * @file Tests for the HealthScore radial gauge component
 * @module __tests__/components/charts/health-score
 *
 * Validates the HealthScore SVG gauge that displays a composite system health
 * score from 0 to 100. Tests cover score display, status text rendering,
 * color thresholds (green >= 80, amber >= 50, red < 50), clamping behavior
 * for out-of-range values, and the default size prop.
 *
 * No external mocks needed — HealthScore is a pure presentational component
 * that renders inline SVG with no external dependencies.
 *
 * @see {@link ../../../components/charts/health-score} Source component
 */
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { HealthScore } from "@/components/charts/health-score";

describe("HealthScore", () => {
  /** Verifies the numeric score is rendered inside the gauge overlay. */
  it("renders the score number in the display", () => {
    render(<HealthScore score={85} status="healthy" />);
    expect(screen.getByText("85")).toBeInTheDocument();
  });

  /** Verifies the status label text (e.g., "healthy") appears below the score. */
  it("shows status text", () => {
    render(<HealthScore score={75} status="degraded" />);
    expect(screen.getByText("degraded")).toBeInTheDocument();
  });

  /** Verifies scores >= 80 produce the green color (#34d399) on the arc and number. */
  it("uses green color (#34d399) for score >= 80", () => {
    const { container } = render(<HealthScore score={92} status="healthy" />);
    const circles = container.querySelectorAll("circle");
    // The second circle is the score arc
    const scoreArc = circles[1];
    expect(scoreArc).toHaveAttribute("stroke", "#34d399");
    // The score number span should also use green
    const scoreSpan = screen.getByText("92");
    expect(scoreSpan).toHaveStyle({ color: "#34d399" });
  });

  /** Verifies scores 50-79 produce the amber color (#fbbf24) on the arc and number. */
  it("uses amber color (#fbbf24) for score 50-79", () => {
    const { container } = render(<HealthScore score={65} status="degraded" />);
    const circles = container.querySelectorAll("circle");
    const scoreArc = circles[1];
    expect(scoreArc).toHaveAttribute("stroke", "#fbbf24");
    const scoreSpan = screen.getByText("65");
    expect(scoreSpan).toHaveStyle({ color: "#fbbf24" });
  });

  /** Verifies scores < 50 produce the red color (#f87171) on the arc and number. */
  it("uses red color (#f87171) for score < 50", () => {
    const { container } = render(<HealthScore score={30} status="critical" />);
    const circles = container.querySelectorAll("circle");
    const scoreArc = circles[1];
    expect(scoreArc).toHaveAttribute("stroke", "#f87171");
    const scoreSpan = screen.getByText("30");
    expect(scoreSpan).toHaveStyle({ color: "#f87171" });
  });

  /** Verifies scores above 100 are clamped to 100 in the display. */
  it("clamps score above 100 to 100", () => {
    render(<HealthScore score={150} status="healthy" />);
    expect(screen.getByText("100")).toBeInTheDocument();
    expect(screen.queryByText("150")).not.toBeInTheDocument();
  });

  /** Verifies negative scores are clamped to 0 in the display. */
  it("clamps negative score to 0", () => {
    render(<HealthScore score={-20} status="critical" />);
    expect(screen.getByText("0")).toBeInTheDocument();
    expect(screen.queryByText("-20")).not.toBeInTheDocument();
  });

  /** Verifies the SVG uses the default size of 140px when no size prop is given. */
  it("uses default size of 140", () => {
    const { container } = render(<HealthScore score={50} status="ok" />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("width", "140");
    expect(svg).toHaveAttribute("height", "140");
  });
});
