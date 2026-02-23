/**
 * @file Tests for the Contribute page
 * @module __tests__/pages/contribute
 *
 * Validates the Contribute page at `/contribute`, which enables browser-based
 * compute contribution. Operators can run prime searches in a Web Worker
 * (WASM-accelerated with JS BigInt fallback). The page shows session stats
 * (time, candidates tested, primes found, speed, blocks/hour), a start/stop
 * control panel with status badge and engine mode indicator, and a scrollable
 * activity log.
 *
 * The exported `ContributePage` wraps `ContributeContent` inside
 * `ContributeProvider`. All dependencies are mocked to isolate page layout
 * and rendering logic.
 *
 * @see {@link ../../app/contribute/page} Source page
 * @see {@link ../../contexts/worker-context} Worker context provider
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

// ── Mutable mock state ──────────────────────────────────────────

const mockContribute = {
  status: "idle" as "idle" | "claiming" | "running" | "submitting" | "paused" | "error",
  stats: {
    tested: 0,
    found: 0,
    blocksCompleted: 0,
    speed: 0,
    sessionStart: null as number | null,
    currentBlockId: null as number | null,
    searchType: null as string | null,
    error: null as string | null,
    mode: null as "wasm" | "js" | null,
    blockProgress: null as number | null,
    lastSubmission: null as {
      status: string;
      hashVerified: boolean;
      creditsEarned: number;
      trustLevel: number;
      badgesEarned: number;
      warnings: string[];
    } | null,
  },
  log: [] as Array<{ id: number; time: number; type: string; message: string }>,
  start: vi.fn(),
  stop: vi.fn(),
};

// ── Mock hooks and components ───────────────────────────────────

vi.mock("@/contexts/worker-context", () => ({
  ContributeProvider: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  useContribute: () => mockContribute,
}));

vi.mock("@/components/view-header", () => ({
  ViewHeader: ({
    title,
    subtitle,
  }: {
    title: string;
    subtitle?: string;
  }) => (
    <div data-testid="view-header">
      <h1>{title}</h1>
      {subtitle && <p>{subtitle}</p>}
    </div>
  ),
}));

vi.mock("lucide-react", () => ({
  Activity: () => <span data-testid="icon-activity" />,
  AlertTriangle: () => <span data-testid="icon-alert-triangle" />,
  Award: () => <span data-testid="icon-award" />,
  CheckCircle: () => <span data-testid="icon-check-circle" />,
  Clock: () => <span data-testid="icon-clock" />,
  Coins: () => <span data-testid="icon-coins" />,
  Cpu: () => <span data-testid="icon-cpu" />,
  Hash: () => <span data-testid="icon-hash" />,
  Play: () => <span data-testid="icon-play" />,
  Search: () => <span data-testid="icon-search" />,
  Shield: () => <span data-testid="icon-shield" />,
  Square: () => <span data-testid="icon-square" />,
  Sparkles: () => <span data-testid="icon-sparkles" />,
  Zap: () => <span data-testid="icon-zap" />,
}));

vi.mock("@/hooks/use-contribute-profile", () => ({
  useContributeProfile: () => ({
    badges: [],
    badgeDefinitions: [],
    recentCredits: [],
    totalCredits: 0,
    trust: { trust_level: 1, consecutive_valid: 0, total_valid: 0 },
    nextBadge: null,
    loading: false,
    error: null,
  }),
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TooltipContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TooltipProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TooltipTrigger: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/lib/format", () => ({
  API_BASE: "http://localhost:3000",
  numberWithCommas: (x: number) =>
    x.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ","),
}));

import ContributePage from "@/app/contribute/page";

// ── Helpers ─────────────────────────────────────────────────────

function resetMock() {
  mockContribute.status = "idle";
  mockContribute.stats = {
    tested: 0,
    found: 0,
    blocksCompleted: 0,
    speed: 0,
    sessionStart: null,
    currentBlockId: null,
    searchType: null,
    error: null,
    mode: null,
    blockProgress: null,
    lastSubmission: null,
  };
  mockContribute.log = [];
}

beforeEach(() => {
  resetMock();
});

// ── Tests ───────────────────────────────────────────────────────

describe("ContributePage", () => {
  it("renders 'Contribute' title", () => {
    render(<ContributePage />);
    expect(screen.getByText("Contribute")).toBeInTheDocument();
  });

  it("renders 'Session Time' stat card", () => {
    render(<ContributePage />);
    expect(screen.getByText("Session Time")).toBeInTheDocument();
  });

  it("renders 'Candidates Tested' stat card", () => {
    render(<ContributePage />);
    expect(screen.getByText("Candidates Tested")).toBeInTheDocument();
  });

  it("renders 'Primes Found' stat card", () => {
    render(<ContributePage />);
    expect(screen.getByText("Primes Found")).toBeInTheDocument();
  });

  it("renders 'Speed' stat card", () => {
    render(<ContributePage />);
    expect(screen.getByText("Speed")).toBeInTheDocument();
  });

  it("shows '--:--' for session time when not running", () => {
    render(<ContributePage />);
    expect(screen.getByText("--:--")).toBeInTheDocument();
  });

  it("shows 'Start Contributing' button when idle", () => {
    render(<ContributePage />);
    expect(screen.getByText("Start Contributing")).toBeInTheDocument();
  });

  it("shows 'Stop' button when running", () => {
    mockContribute.status = "running";
    mockContribute.stats.sessionStart = Date.now() - 5000;
    render(<ContributePage />);
    expect(screen.getByText("Stop")).toBeInTheDocument();
  });

  it("shows status badge with correct text for idle", () => {
    render(<ContributePage />);
    expect(screen.getByText("Idle")).toBeInTheDocument();
  });

  it("shows status badge with correct text for running", () => {
    mockContribute.status = "running";
    render(<ContributePage />);
    expect(screen.getByText("Computing")).toBeInTheDocument();
  });

  it("shows activity log empty state", () => {
    render(<ContributePage />);
    expect(
      screen.getByText("Start contributing to see activity here.")
    ).toBeInTheDocument();
  });

  it("shows error message when stats.error is set", () => {
    mockContribute.stats.error = "Connection failed";
    render(<ContributePage />);
    expect(screen.getByText("Connection failed")).toBeInTheDocument();
  });

  it("shows last submission feedback when present", () => {
    mockContribute.stats.lastSubmission = {
      status: "ok",
      hashVerified: true,
      creditsEarned: 500,
      trustLevel: 2,
      badgesEarned: 0,
      warnings: [],
    };
    render(<ContributePage />);
    expect(screen.getByText("Last submission")).toBeInTheDocument();
    expect(screen.getByText("Hash verified")).toBeInTheDocument();
    expect(screen.getByText("+500 credits")).toBeInTheDocument();
  });

  it("shows warning when last submission has warnings", () => {
    mockContribute.stats.lastSubmission = {
      status: "ok",
      hashVerified: false,
      creditsEarned: 0,
      trustLevel: 1,
      badgesEarned: 0,
      warnings: ["hash mismatch — no credits for this block"],
    };
    render(<ContributePage />);
    expect(
      screen.getByText("hash mismatch — no credits for this block")
    ).toBeInTheDocument();
  });

  it("shows new badges earned in last submission", () => {
    mockContribute.stats.lastSubmission = {
      status: "ok",
      hashVerified: true,
      creditsEarned: 1500,
      trustLevel: 3,
      badgesEarned: 2,
      warnings: [],
    };
    render(<ContributePage />);
    expect(screen.getByText("2 new badges!")).toBeInTheDocument();
  });

  it("does not show last submission section when null", () => {
    mockContribute.stats.lastSubmission = null;
    render(<ContributePage />);
    expect(screen.queryByText("Last submission")).not.toBeInTheDocument();
  });
});
