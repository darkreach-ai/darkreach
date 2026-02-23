/**
 * @file Tests for agent helper utilities
 * @module __tests__/components/agents/helpers
 *
 * Validates the shared badge renderers, icon factories, and cost estimation
 * heuristics used across agent management components. Tests cover:
 * - statusBadge: colored Badge for task statuses (pending → cancelled)
 * - priorityBadge: colored text label for task priorities (low → urgent)
 * - eventIcon: Lucide icon per event type (created, started, completed, etc.)
 * - statusDot: small colored dot indicator per task status
 * - roleBadge: colored Badge for agent roles (engine, frontend, ops, research)
 * - estimateCostRange: heuristic cost estimate based on model rate + description length
 * - MODEL_RATES: per-model cost rate table (opus, sonnet, haiku)
 *
 * @see {@link ../../../components/agents/helpers} Source module
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

// Mock lucide-react icons as simple spans with data-testid attributes.
vi.mock("lucide-react", () => ({
  Bot: (props: any) => <span data-testid="icon-bot" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Play: (props: any) => <span data-testid="icon-play" {...props} />,
  CheckCircle2: (props: any) => <span data-testid="icon-check-circle" {...props} />,
  XCircle: (props: any) => <span data-testid="icon-x-circle" {...props} />,
  MessageSquare: (props: any) => <span data-testid="icon-message" {...props} />,
  Wrench: (props: any) => <span data-testid="icon-wrench" {...props} />,
  ArrowRight: (props: any) => <span data-testid="icon-arrow-right" {...props} />,
  AlertCircle: (props: any) => <span data-testid="icon-alert-circle" {...props} />,
  Stethoscope: (props: any) => <span data-testid="icon-stethoscope" {...props} />,
  DollarSign: (props: any) => <span data-testid="icon-dollar" {...props} />,
  Cog: (props: any) => <span data-testid="icon-cog" {...props} />,
  Layout: (props: any) => <span data-testid="icon-layout" {...props} />,
  Server: (props: any) => <span data-testid="icon-server" {...props} />,
  BookOpen: (props: any) => <span data-testid="icon-book" {...props} />,
}));

// Mock the shadcn Badge component.
vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children, variant, className, ...props }: any) => (
    <span data-testid="badge" data-variant={variant} className={className} {...props}>
      {children}
    </span>
  ),
}));

import {
  statusBadge,
  priorityBadge,
  eventIcon,
  statusDot,
  roleBadge,
  estimateCostRange,
  MODEL_RATES,
  ROLE_CONFIG,
} from "@/components/agents/helpers";

// ── statusBadge ──────────────────────────────────────────────────

describe("statusBadge", () => {
  /** Verifies each status renders the correct Badge variant. */
  it.each([
    ["pending", "outline"],
    ["in_progress", "default"],
    ["completed", "secondary"],
    ["failed", "destructive"],
    ["cancelled", "outline"],
  ] as const)("renders %s status with %s variant", (status, expectedVariant) => {
    const { container } = render(statusBadge(status));
    const badge = container.querySelector("[data-testid='badge']");
    expect(badge).toBeTruthy();
    expect(badge!.getAttribute("data-variant")).toBe(expectedVariant);
  });

  /** Verifies each status renders the correct human-readable label. */
  it.each([
    ["pending", "Pending"],
    ["in_progress", "Running"],
    ["completed", "Completed"],
    ["failed", "Failed"],
    ["cancelled", "Cancelled"],
  ] as const)("renders %s status with label '%s'", (status, expectedLabel) => {
    const { container } = render(statusBadge(status));
    expect(container.textContent).toBe(expectedLabel);
  });

  /** Verifies unknown status falls back to outline variant and raw status text. */
  it("handles unknown status gracefully", () => {
    const { container } = render(statusBadge("unknown_status"));
    const badge = container.querySelector("[data-testid='badge']");
    expect(badge!.getAttribute("data-variant")).toBe("outline");
    expect(container.textContent).toBe("unknown_status");
  });
});

// ── priorityBadge ────────────────────────────────────────────────

describe("priorityBadge", () => {
  /** Verifies each priority level renders with its designated color class. */
  it.each([
    ["low", "text-muted-foreground"],
    ["normal", "text-foreground"],
    ["high", "text-amber-500"],
    ["urgent", "text-red-500"],
  ] as const)("renders %s priority with class %s", (priority, expectedClass) => {
    const { container } = render(priorityBadge(priority));
    const span = container.querySelector("span");
    expect(span).toBeTruthy();
    expect(span!.className).toContain(expectedClass);
    expect(span!.textContent).toBe(priority);
  });

  /** Verifies unknown priority still renders the text. */
  it("renders unknown priority without special color", () => {
    const { container } = render(priorityBadge("custom"));
    expect(container.textContent).toBe("custom");
  });
});

// ── eventIcon ────────────────────────────────────────────────────

describe("eventIcon", () => {
  /** Verifies the correct icon is returned for each known event type. */
  it.each([
    ["created", "icon-plus"],
    ["started", "icon-play"],
    ["completed", "icon-check-circle"],
    ["failed", "icon-x-circle"],
    ["error", "icon-alert-circle"],
    ["tool_call", "icon-wrench"],
  ] as const)("returns correct icon for '%s' event", (type, expectedTestId) => {
    const { container } = render(eventIcon(type));
    expect(container.querySelector(`[data-testid='${expectedTestId}']`)).toBeTruthy();
  });

  /** Verifies that an unknown event type falls back to the Bot icon. */
  it("returns Bot icon for unknown event types", () => {
    const { container } = render(eventIcon("some_unknown_type"));
    expect(container.querySelector("[data-testid='icon-bot']")).toBeTruthy();
  });
});

// ── statusDot ────────────────────────────────────────────────────

describe("statusDot", () => {
  /** Verifies the correct background color class for each status. */
  it.each([
    ["pending", "bg-muted-foreground/30"],
    ["in_progress", "bg-blue-500"],
    ["completed", "bg-green-500"],
    ["failed", "bg-red-500"],
    ["cancelled", "bg-muted-foreground/30"],
  ] as const)("renders %s status with class '%s'", (status, expectedClass) => {
    const { container } = render(statusDot(status));
    const dot = container.querySelector("span");
    expect(dot).toBeTruthy();
    expect(dot!.className).toContain(expectedClass);
  });

  /** Verifies unknown status falls back to bg-muted. */
  it("renders bg-muted for unknown status", () => {
    const { container } = render(statusDot("unknown"));
    const dot = container.querySelector("span");
    expect(dot!.className).toContain("bg-muted");
  });
});

// ── roleBadge ────────────────────────────────────────────────────

describe("roleBadge", () => {
  /** Verifies null input returns null (no rendered output). */
  it("returns null for null input", () => {
    const { container } = render(<>{roleBadge(null)}</>);
    expect(container.innerHTML).toBe("");
  });

  /** Verifies known roles render their configured labels and color classes. */
  it.each([
    ["engine", "Engine", "amber"],
    ["frontend", "Frontend", "blue"],
    ["ops", "Ops", "green"],
    ["research", "Research", "indigo"],
  ] as const)("renders %s role with label '%s' and %s color", (role, label, color) => {
    const { container } = render(roleBadge(role));
    const badge = container.querySelector("[data-testid='badge']");
    expect(badge).toBeTruthy();
    expect(badge!.textContent).toBe(label);
    expect(badge!.className).toContain(color);
  });

  /** Verifies an unknown role renders as a plain badge with the raw role name. */
  it("renders unknown role as plain badge with raw name", () => {
    const { container } = render(roleBadge("custom-role"));
    const badge = container.querySelector("[data-testid='badge']");
    expect(badge).toBeTruthy();
    expect(badge!.textContent).toBe("custom-role");
    expect(badge!.getAttribute("data-variant")).toBe("outline");
  });
});

// ── estimateCostRange ────────────────────────────────────────────

describe("estimateCostRange", () => {
  /** Verifies correct cost range for opus model with a short description. */
  it("returns correct range for opus model with short description", () => {
    const range = estimateCostRange("opus", 100);
    // Short (<= 200): minMinutes=1, maxMinutes=4, rate=0.90
    expect(range.low).toBeCloseTo(0.9 * 1);
    expect(range.high).toBeCloseTo(0.9 * 4);
  });

  /** Verifies correct cost range for haiku model. */
  it("returns correct range for haiku model with short description", () => {
    const range = estimateCostRange("haiku", 50);
    // Short (<= 200): minMinutes=1, maxMinutes=4, rate=0.06
    expect(range.low).toBeCloseTo(0.06 * 1);
    expect(range.high).toBeCloseTo(0.06 * 4);
  });

  /** Verifies short description (< 200 chars) uses minMinutes=1, maxMinutes=4. */
  it("uses minMinutes=1, maxMinutes=4 for short descriptions (< 200 chars)", () => {
    const range = estimateCostRange("sonnet", 150);
    expect(range.low).toBeCloseTo(0.30 * 1);
    expect(range.high).toBeCloseTo(0.30 * 4);
  });

  /** Verifies medium description (200-500 chars) uses minMinutes=3, maxMinutes=8. */
  it("uses minMinutes=3, maxMinutes=8 for medium descriptions (200-500 chars)", () => {
    const range = estimateCostRange("sonnet", 350);
    // Medium (> 200, <= 500): minMinutes=3, maxMinutes=8, rate=0.30
    expect(range.low).toBeCloseTo(0.30 * 3);
    expect(range.high).toBeCloseTo(0.30 * 8);
  });

  /** Verifies long description (> 500 chars) uses minMinutes=3, maxMinutes=15. */
  it("uses minMinutes=3, maxMinutes=15 for long descriptions (> 500 chars)", () => {
    const range = estimateCostRange("sonnet", 600);
    // Long (> 500): minMinutes=3, maxMinutes=15, rate=0.30
    expect(range.low).toBeCloseTo(0.30 * 3);
    expect(range.high).toBeCloseTo(0.30 * 15);
  });

  /** Verifies unknown model falls back to sonnet rate. */
  it("falls back to sonnet rate for unknown model", () => {
    const range = estimateCostRange("unknown-model", 100);
    expect(range.low).toBeCloseTo(0.30 * 1);
    expect(range.high).toBeCloseTo(0.30 * 4);
  });
});

// ── MODEL_RATES ──────────────────────────────────────────────────

describe("MODEL_RATES", () => {
  /** Verifies all three expected model entries exist with correct rates. */
  it("has entries for opus, sonnet, and haiku", () => {
    expect(MODEL_RATES).toHaveProperty("opus");
    expect(MODEL_RATES).toHaveProperty("sonnet");
    expect(MODEL_RATES).toHaveProperty("haiku");
  });

  it("opus rate is $0.90/min", () => {
    expect(MODEL_RATES.opus.perMin).toBe(0.90);
    expect(MODEL_RATES.opus.label).toBe("Opus");
  });

  it("sonnet rate is $0.30/min", () => {
    expect(MODEL_RATES.sonnet.perMin).toBe(0.30);
    expect(MODEL_RATES.sonnet.label).toBe("Sonnet");
  });

  it("haiku rate is $0.06/min", () => {
    expect(MODEL_RATES.haiku.perMin).toBe(0.06);
    expect(MODEL_RATES.haiku.label).toBe("Haiku");
  });
});

// ── ROLE_CONFIG ──────────────────────────────────────────────────

describe("ROLE_CONFIG", () => {
  /** Verifies all four expected role entries exist. */
  it("has entries for engine, frontend, ops, and research", () => {
    expect(ROLE_CONFIG).toHaveProperty("engine");
    expect(ROLE_CONFIG).toHaveProperty("frontend");
    expect(ROLE_CONFIG).toHaveProperty("ops");
    expect(ROLE_CONFIG).toHaveProperty("research");
  });

  /** Verifies each role config has an icon, color, and label. */
  it("each role has icon, color, and label fields", () => {
    for (const key of ["engine", "frontend", "ops", "research"]) {
      const cfg = ROLE_CONFIG[key];
      expect(cfg).toHaveProperty("icon");
      expect(cfg).toHaveProperty("color");
      expect(cfg).toHaveProperty("label");
      expect(typeof cfg.label).toBe("string");
      expect(typeof cfg.color).toBe("string");
    }
  });
});
