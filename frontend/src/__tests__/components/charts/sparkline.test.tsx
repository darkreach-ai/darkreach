/**
 * @file Tests for the Sparkline inline chart component
 * @module __tests__/components/charts/sparkline
 *
 * Validates the compact SVG sparkline component used in stat cards and
 * table rows. The component renders an SVG polyline from numeric data
 * points with optional gradient fill below the line. When fewer than
 * 2 data points are provided, it renders an empty SVG placeholder.
 *
 * Tests cover: empty data handling, dimension props, polyline rendering,
 * default dimensions, custom stroke color, gradient fill toggle, and
 * className pass-through.
 *
 * @see {@link ../../../components/charts/sparkline} Source component
 */
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { Sparkline } from "@/components/charts/sparkline";

// ── Factory helpers ──────────────────────────────────────────────

/** Generate an array of numeric data points for sparkline tests. */
function makeData(count: number): number[] {
  return Array.from({ length: count }, (_, i) => (i + 1) * 10);
}

// ── Sparkline component ──────────────────────────────────────────

describe("Sparkline", () => {
  /** Verifies the component renders an empty SVG when data has fewer than 2 points. */
  it("renders empty SVG when data has fewer than 2 points", () => {
    const { container } = render(<Sparkline data={[42]} />);
    const svg = container.querySelector("svg");
    expect(svg).toBeTruthy();
    // No polyline should be present — just an empty SVG shell.
    expect(svg!.querySelector("polyline")).toBeNull();
    expect(svg!.querySelector("path")).toBeNull();
    expect(svg!.querySelector("defs")).toBeNull();
  });

  /** Verifies empty data (zero points) also renders empty SVG. */
  it("renders empty SVG when data is empty", () => {
    const { container } = render(<Sparkline data={[]} />);
    const svg = container.querySelector("svg");
    expect(svg).toBeTruthy();
    expect(svg!.querySelector("polyline")).toBeNull();
  });

  /** Verifies SVG dimensions match the width and height props. */
  it("renders SVG with correct width and height from props", () => {
    const { container } = render(
      <Sparkline data={makeData(5)} width={120} height={48} />,
    );
    const svg = container.querySelector("svg");
    expect(svg).toBeTruthy();
    expect(svg!.getAttribute("width")).toBe("120");
    expect(svg!.getAttribute("height")).toBe("48");
  });

  /** Verifies a polyline element is rendered when sufficient data points exist. */
  it("renders polyline element with data of 5 points", () => {
    const { container } = render(<Sparkline data={makeData(5)} />);
    const polyline = container.querySelector("polyline");
    expect(polyline).toBeTruthy();
    // The points attribute should contain comma-separated coordinate pairs.
    const points = polyline!.getAttribute("points");
    expect(points).toBeTruthy();
    expect(points!.length).toBeGreaterThan(0);
  });

  /** Verifies default dimensions are 80x32 when width/height are not specified. */
  it("uses default dimensions (80x32) when not specified", () => {
    const { container } = render(<Sparkline data={makeData(3)} />);
    const svg = container.querySelector("svg");
    expect(svg!.getAttribute("width")).toBe("80");
    expect(svg!.getAttribute("height")).toBe("32");
  });

  /** Verifies the custom color is applied to the polyline stroke attribute. */
  it("applies custom color to polyline stroke", () => {
    const { container } = render(
      <Sparkline data={makeData(4)} color="#6366f1" />,
    );
    const polyline = container.querySelector("polyline");
    expect(polyline).toBeTruthy();
    expect(polyline!.getAttribute("stroke")).toBe("#6366f1");
  });

  /** Verifies gradient fill elements are present by default (fill=true). */
  it("renders gradient fill by default (linearGradient and path)", () => {
    const { container } = render(<Sparkline data={makeData(5)} />);
    const svg = container.querySelector("svg");
    // Default fill=true should produce a <defs> with <linearGradient> and a <path>.
    expect(svg!.querySelector("defs")).toBeTruthy();
    expect(svg!.querySelector("linearGradient")).toBeTruthy();
    expect(svg!.querySelector("path")).toBeTruthy();
  });

  /** Verifies fill elements are absent when fill=false is set. */
  it("hides fill when fill=false prop is set", () => {
    const { container } = render(
      <Sparkline data={makeData(5)} fill={false} />,
    );
    const svg = container.querySelector("svg");
    // No gradient or fill path should be rendered.
    expect(svg!.querySelector("defs")).toBeNull();
    expect(svg!.querySelector("linearGradient")).toBeNull();
    expect(svg!.querySelector("path")).toBeNull();
  });

  /** Verifies className is passed through to the SVG element. */
  it("applies className to SVG element", () => {
    const { container } = render(
      <Sparkline data={makeData(3)} className="my-sparkline" />,
    );
    const svg = container.querySelector("svg");
    expect(svg!.getAttribute("class")).toContain("my-sparkline");
  });
});
