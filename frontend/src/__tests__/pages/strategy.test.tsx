/**
 * @file Tests for the Strategy Engine page
 * @module __tests__/pages/strategy
 *
 * Validates the Strategy Engine admin page at `/strategy`, which displays the
 * AI strategy engine status, form scoring table, decision timeline, AI engine
 * state (tick count, cost model, weights), and configuration panel. The page
 * wraps its inner content in a React Suspense boundary.
 *
 * Tests verify: page heading/subtitle, header metrics, status/last-tick/spend
 * overview cards, tab navigation (Overview, Scoring, Decisions, AI Engine,
 * Config), score bars, scoring table with form names and values, decision
 * cards with type/form badges and estimated cost, AI engine tick count and
 * cost model version, config panel input fields, and the Force Tick button.
 *
 * @see {@link ../../app/strategy/page} Source page
 * @see {@link ../../hooks/use-strategy} Strategy data hooks
 * @see {@link ../../contexts/websocket-context} AI engine WebSocket data
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mutable mock return values ──────────────────────────────────

let mockStatus: {
  enabled: boolean;
  tick_interval_secs: number;
  last_tick: string | null;
  monthly_spend_usd: number;
  monthly_budget_usd: number;
  max_concurrent_projects: number;
} = {
  enabled: true,
  tick_interval_secs: 300,
  last_tick: "2026-02-20T12:00:00Z",
  monthly_spend_usd: 15.5,
  monthly_budget_usd: 100,
  max_concurrent_projects: 3,
};

const defaultStatus = { ...mockStatus };

// ── Mock hooks ──────────────────────────────────────────────────

vi.mock("@/hooks/use-strategy", () => ({
  useStrategyStatus: () => ({
    status: mockStatus,
    loading: false,
    error: null,
    refetch: vi.fn(),
  }),
  useStrategyScores: () => ({
    scores: [
      {
        form: "factorial",
        record_gap: 0.5,
        yield_rate: 0.3,
        cost_efficiency: 0.7,
        coverage_gap: 0.4,
        network_fit: 0.6,
        total: 0.55,
      },
      {
        form: "kbn",
        record_gap: 0.4,
        yield_rate: 0.5,
        cost_efficiency: 0.6,
        coverage_gap: 0.3,
        network_fit: 0.7,
        total: 0.52,
      },
    ],
    loading: false,
    error: null,
    refetch: vi.fn(),
  }),
  useStrategyDecisions: () => ({
    decisions: [
      {
        id: 1,
        decision_type: "create_project",
        form: "factorial",
        summary: "Create search",
        reasoning: "High gap",
        params: null,
        estimated_cost_usd: 1.5,
        action_taken: "executed",
        override_reason: null,
        project_id: null,
        search_job_id: null,
        scores: null,
        created_at: "2026-02-20T12:00:00Z",
      },
    ],
    loading: false,
    error: null,
    refetch: vi.fn(),
  }),
  useStrategyConfig: () => ({
    config: {
      id: 1,
      enabled: true,
      max_concurrent_projects: 3,
      max_monthly_budget_usd: 100,
      max_per_project_budget_usd: 25,
      preferred_forms: ["factorial"],
      excluded_forms: [],
      min_idle_workers_to_create: 2,
      record_proximity_threshold: 0.8,
      tick_interval_secs: 300,
      updated_at: "2026-02-20",
    },
    loading: false,
    error: null,
    refetch: vi.fn(),
  }),
  updateStrategyConfig: vi.fn(),
  triggerStrategyTick: vi.fn(),
  overrideDecision: vi.fn(),
}));

vi.mock("@/contexts/websocket-context", () => ({
  useWs: () => ({
    aiEngine: {
      tick_count: 42,
      cost_model_version: 3,
      recent_decisions: [],
      scoring_weights: null,
    },
  }),
}));

// ── Mock UI components ──────────────────────────────────────────

vi.mock("@/components/view-header", () => ({
  ViewHeader: ({
    title,
    subtitle,
    metadata,
    actions,
  }: {
    title: string;
    subtitle: string;
    metadata?: React.ReactNode;
    actions?: React.ReactNode;
  }) => (
    <div data-testid="view-header">
      <h1>{title}</h1>
      <p>{subtitle}</p>
      {metadata}
      {actions}
    </div>
  ),
  HeaderBadge: ({ children }: { children: React.ReactNode }) => (
    <span>{children}</span>
  ),
  HeaderMetric: ({ value }: { value: React.ReactNode }) => (
    <span>{value}</span>
  ),
}));

vi.mock("@/components/empty-state", () => ({
  EmptyState: ({ message }: { message: string }) => (
    <div data-testid="empty-state">{message}</div>
  ),
}));

vi.mock("lucide-react", () => ({
  Brain: () => <span data-testid="icon-brain" />,
  Play: () => <span data-testid="icon-play" />,
  Pause: () => <span data-testid="icon-pause" />,
  Clock: () => <span data-testid="icon-clock" />,
  DollarSign: () => <span data-testid="icon-dollar" />,
  Activity: () => <span data-testid="icon-activity" />,
  ChevronDown: () => <span data-testid="icon-chevron-down" />,
  ChevronRight: () => <span data-testid="icon-chevron-right" />,
  RefreshCw: () => <span data-testid="icon-refresh" />,
  Cpu: () => <span data-testid="icon-cpu" />,
  Target: () => <span data-testid="icon-target" />,
  CheckCircle2: () => <span data-testid="icon-check" />,
  XCircle: () => <span data-testid="icon-xcircle" />,
  CircleDot: () => <span data-testid="icon-circledot" />,
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
  usePathname: () => "/strategy",
}));

import StrategyPage from "@/app/strategy/page";

beforeEach(() => {
  mockStatus = { ...defaultStatus };
});

// ── Tests ───────────────────────────────────────────────────────

describe("StrategyPage", () => {
  it("renders 'Strategy Engine' title", () => {
    render(<StrategyPage />);
    expect(screen.getByText("Strategy Engine")).toBeInTheDocument();
  });

  it("renders subtitle text", () => {
    render(<StrategyPage />);
    expect(
      screen.getByText(
        "Autonomous search form selection and project creation"
      )
    ).toBeInTheDocument();
  });

  it("shows status metric 'Active' when enabled", () => {
    render(<StrategyPage />);
    expect(screen.getByText("Active")).toBeInTheDocument();
  });

  it("shows monthly spend metric in header", () => {
    render(<StrategyPage />);
    expect(screen.getByText("$15.50 / $100")).toBeInTheDocument();
  });

  it("renders tabs (Overview, Scoring, Decisions, AI Engine, Config)", () => {
    render(<StrategyPage />);
    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText(/Scoring/)).toBeInTheDocument();
    expect(screen.getByText(/Decisions/)).toBeInTheDocument();
    expect(screen.getByText(/AI Engine/)).toBeInTheDocument();
    expect(screen.getByText("Config")).toBeInTheDocument();
  });

  it("shows 'On' in status card when enabled", () => {
    render(<StrategyPage />);
    expect(screen.getByText("On")).toBeInTheDocument();
  });

  it("shows 'Never' when last_tick is null", () => {
    mockStatus = { ...defaultStatus, last_tick: null, monthly_spend_usd: 0 };
    render(<StrategyPage />);
    expect(screen.getByText("Never")).toBeInTheDocument();
  });

  it("shows monthly spend amount in overview card", () => {
    render(<StrategyPage />);
    expect(screen.getByText("$15.50")).toBeInTheDocument();
  });

  it("shows active projects limit in overview card", () => {
    render(<StrategyPage />);
    expect(screen.getByText(/— \/ 3/)).toBeInTheDocument();
  });

  it("renders score bars in overview (top 5 forms)", () => {
    render(<StrategyPage />);
    // ScoreBar renders form names as text
    expect(screen.getByText("factorial")).toBeInTheDocument();
    expect(screen.getByText("kbn")).toBeInTheDocument();
  });

  it("renders score values in overview score bars", () => {
    render(<StrategyPage />);
    // ScoreBar renders total.toFixed(2) — "0.55" and "0.52"
    expect(screen.getByText("0.55")).toBeInTheDocument();
    expect(screen.getByText("0.52")).toBeInTheDocument();
  });

  it("scoring tab shows table with form names", async () => {
    render(<StrategyPage />);
    const scoringTab = screen.getByText(/Scoring/);
    await userEvent.click(scoringTab);
    expect(screen.getByText("factorial")).toBeInTheDocument();
    expect(screen.getByText("kbn")).toBeInTheDocument();
  });

  it("scoring tab shows form scores in table", async () => {
    render(<StrategyPage />);
    const scoringTab = screen.getByText(/Scoring/);
    await userEvent.click(scoringTab);
    // Table shows total.toFixed(3) — "0.550" and "0.520"
    expect(screen.getByText("0.550")).toBeInTheDocument();
    expect(screen.getByText("0.520")).toBeInTheDocument();
  });

  it("decisions tab shows decision cards", async () => {
    render(<StrategyPage />);
    const decisionsTab = screen.getByText(/Decisions/);
    await userEvent.click(decisionsTab);
    expect(screen.getByText("Create search")).toBeInTheDocument();
  });

  it("decision card shows decision type badge", async () => {
    render(<StrategyPage />);
    const decisionsTab = screen.getByText(/Decisions/);
    await userEvent.click(decisionsTab);
    expect(screen.getByText("create project")).toBeInTheDocument();
  });

  it("decision card shows form badge", async () => {
    render(<StrategyPage />);
    const decisionsTab = screen.getByText(/Decisions/);
    await userEvent.click(decisionsTab);
    const formBadges = screen.getAllByText("factorial");
    expect(formBadges.length).toBeGreaterThanOrEqual(1);
  });

  it("decision card shows estimated cost", async () => {
    render(<StrategyPage />);
    const decisionsTab = screen.getByText(/Decisions/);
    await userEvent.click(decisionsTab);
    expect(screen.getByText(/Est\. \$1\.50/)).toBeInTheDocument();
  });

  it("AI engine tab shows tick count", async () => {
    render(<StrategyPage />);
    const aiTab = screen.getByText(/AI Engine/);
    await userEvent.click(aiTab);
    expect(screen.getByText("42")).toBeInTheDocument();
  });

  it("AI engine tab shows cost model version", async () => {
    render(<StrategyPage />);
    const aiTab = screen.getByText(/AI Engine/);
    await userEvent.click(aiTab);
    expect(screen.getByText("v3")).toBeInTheDocument();
  });

  it("config panel renders input fields", async () => {
    render(<StrategyPage />);
    const configTab = screen.getByText("Config");
    await userEvent.click(configTab);
    expect(
      screen.getByText("Max Concurrent Projects")
    ).toBeInTheDocument();
    expect(screen.getByText("Monthly Budget (USD)")).toBeInTheDocument();
    expect(
      screen.getByText("Per-Project Budget (USD)")
    ).toBeInTheDocument();
    expect(
      screen.getByText("Min Idle Workers to Create")
    ).toBeInTheDocument();
    expect(
      screen.getByText("Record Proximity Threshold")
    ).toBeInTheDocument();
    expect(
      screen.getByText("Tick Interval (seconds)")
    ).toBeInTheDocument();
  });

  it("Force Tick button is present", () => {
    render(<StrategyPage />);
    expect(screen.getByText("Force Tick")).toBeInTheDocument();
  });
});
