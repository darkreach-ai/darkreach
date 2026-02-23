/**
 * @file Tests for the use-strategy hooks and action functions
 * @module __tests__/hooks/use-strategy
 *
 * Validates the AI strategy engine hooks which provide status, form scores,
 * decision history, and configuration from the REST API. The four hooks
 * follow a fetch-on-mount + 30s poll pattern (except useStrategyConfig
 * which fetches once). Three standalone action functions (PUT config,
 * POST override, POST tick) are also tested.
 *
 * The strategy engine implements an OODA decision loop (Observe-Orient-
 * Decide-Act) that autonomously selects search forms, creates projects,
 * and allocates resources. These hooks expose the engine's state to the
 * frontend dashboard.
 *
 * @see {@link ../../hooks/use-strategy} Source hooks and actions
 * @see {@link ../../app/strategy/page} Strategy dashboard page
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// Mock global fetch for REST API calls
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

import {
  useStrategyStatus,
  useStrategyScores,
  useStrategyDecisions,
  useStrategyConfig,
  updateStrategyConfig,
  overrideDecision,
  triggerStrategyTick,
} from "@/hooks/use-strategy";

// ── useStrategyStatus ────────────────────────────────────────────

describe("useStrategyStatus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches strategy status on mount and populates
   * the status object with engine state: enabled flag, tick interval,
   * budget limits, and concurrency caps.
   */
  it("fetches status on mount and sets status data", async () => {
    const mockStatus = {
      enabled: true,
      tick_interval_secs: 300,
      last_tick: "2026-02-22T10:00:00Z",
      monthly_spend_usd: 12.5,
      monthly_budget_usd: 100.0,
      max_concurrent_projects: 3,
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: mockStatus }),
    });

    const { result } = renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(result.current.status).toEqual(mockStatus);
    });
    expect(result.current.loading).toBe(false);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/strategy/status")
    );
  });

  /**
   * Verifies that loading transitions from true (initial) to false
   * after the fetch completes.
   */
  it("sets loading false after fetch", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ enabled: false }),
    });

    const { result } = renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
  });

  /**
   * Verifies that a non-ok HTTP response sets the error state to a
   * descriptive message rather than crashing the hook.
   */
  it("sets error on non-ok response", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
    });

    const { result } = renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(result.current.error).toBe("Failed to fetch strategy status");
    });
    expect(result.current.loading).toBe(false);
    expect(result.current.status).toBeNull();
  });

  /**
   * Verifies that a network error (fetch rejection) sets error to
   * "Network error" and does not leave the hook in a loading state.
   */
  it("sets error 'Network error' on fetch throw", async () => {
    mockFetch.mockRejectedValue(new Error("Connection refused"));

    const { result } = renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(result.current.error).toBe("Network error");
    });
    expect(result.current.loading).toBe(false);
  });

  /**
   * Verifies that the hook sets up a 30-second polling interval after
   * the initial fetch, re-fetching status to keep the dashboard current.
   */
  it("polls every 30 seconds", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ enabled: true }),
    });

    const setIntervalSpy = vi.spyOn(global, "setInterval");

    renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000);
    setIntervalSpy.mockRestore();
  });

  /**
   * Verifies that the polling interval is cleared on unmount to prevent
   * memory leaks and stale fetches after the component is removed.
   */
  it("cleanup clears interval on unmount", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ enabled: true }),
    });

    const clearIntervalSpy = vi.spyOn(global, "clearInterval");

    const { unmount } = renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
    clearIntervalSpy.mockRestore();
  });

  /**
   * Verifies that the refetch function re-fetches data on demand,
   * independent of the polling interval.
   */
  it("refetch function re-fetches data", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ enabled: false }),
    });

    const { result } = renderHook(() => useStrategyStatus());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});

// ── useStrategyScores ────────────────────────────────────────────

describe("useStrategyScores", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches form scores on mount and returns
   * an array of score objects with per-form multi-factor ratings.
   */
  it("fetches and returns scores array", async () => {
    const mockScores = [
      {
        form: "factorial",
        record_gap: 0.8,
        yield_rate: 0.6,
        cost_efficiency: 0.9,
        coverage_gap: 0.3,
        network_fit: 0.7,
        total: 3.3,
      },
      {
        form: "kbn",
        record_gap: 0.5,
        yield_rate: 0.7,
        cost_efficiency: 0.6,
        coverage_gap: 0.8,
        network_fit: 0.9,
        total: 3.5,
      },
    ];
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: mockScores }),
    });

    const { result } = renderHook(() => useStrategyScores());

    await waitFor(() => {
      expect(result.current.scores).toHaveLength(2);
    });
    expect(result.current.scores[0].form).toBe("factorial");
    expect(result.current.scores[1].total).toBe(3.5);
    expect(result.current.loading).toBe(false);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/strategy/scores")
    );
  });

  /**
   * Verifies that a failed response sets the error state and leaves
   * scores as the initial empty array.
   */
  it("handles error", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
    });

    const { result } = renderHook(() => useStrategyScores());

    await waitFor(() => {
      expect(result.current.error).toBe("Failed to fetch scores");
    });
    expect(result.current.scores).toEqual([]);
  });

  /**
   * Verifies that a 30-second polling interval is set up for scores.
   */
  it("sets up 30s polling interval", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });

    const setIntervalSpy = vi.spyOn(global, "setInterval");

    renderHook(() => useStrategyScores());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000);
    setIntervalSpy.mockRestore();
  });
});

// ── useStrategyDecisions ─────────────────────────────────────────

describe("useStrategyDecisions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches decision history on mount and
   * returns an array of decision records with type, form, reasoning,
   * and audit trail fields.
   */
  it("fetches and returns decisions array", async () => {
    const mockDecisions = [
      {
        id: 1,
        decision_type: "create_project",
        form: "factorial",
        summary: "Create factorial 1000-2000",
        reasoning: "High record gap, low competition",
        params: { start: 1000, end: 2000 },
        estimated_cost_usd: 0.5,
        action_taken: "executed",
        override_reason: null,
        project_id: 42,
        search_job_id: null,
        scores: null,
        created_at: "2026-02-22T10:00:00Z",
      },
    ];
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockDecisions),
    });

    const { result } = renderHook(() => useStrategyDecisions());

    await waitFor(() => {
      expect(result.current.decisions).toHaveLength(1);
    });
    expect(result.current.decisions[0].decision_type).toBe("create_project");
    expect(result.current.decisions[0].form).toBe("factorial");
    expect(result.current.loading).toBe(false);
  });

  /**
   * Verifies that a failed response sets the error state.
   */
  it("sets error on failure", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
    });

    const { result } = renderHook(() => useStrategyDecisions());

    await waitFor(() => {
      expect(result.current.error).toBe("Failed to fetch decisions");
    });
    expect(result.current.decisions).toEqual([]);
  });
});

// ── useStrategyConfig ────────────────────────────────────────────

describe("useStrategyConfig", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that useStrategyConfig fetches once on mount and does NOT
   * set up a polling interval. Config is static and only changes on
   * explicit user action (PUT), so polling would be wasteful.
   */
  it("fetches config once with no poll interval", async () => {
    const mockConfig = {
      id: 1,
      enabled: true,
      max_concurrent_projects: 5,
      max_monthly_budget_usd: 100.0,
      max_per_project_budget_usd: 20.0,
      preferred_forms: ["factorial", "kbn"],
      excluded_forms: [],
      min_idle_workers_to_create: 2,
      record_proximity_threshold: 0.1,
      tick_interval_secs: 300,
      updated_at: "2026-02-22T10:00:00Z",
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: mockConfig }),
    });

    const setIntervalSpy = vi.spyOn(global, "setInterval");

    const { result } = renderHook(() => useStrategyConfig());

    await waitFor(() => {
      expect(result.current.config).toEqual(mockConfig);
    });
    expect(result.current.loading).toBe(false);
    expect(mockFetch).toHaveBeenCalledTimes(1);

    // No 30s polling interval should be registered for config.
    // Note: waitFor() internally uses setInterval(fn, 50) for its own polling,
    // so we filter to only check for the 30_000ms interval.
    const intervalCalls = setIntervalSpy.mock.calls.filter(
      (call) => call[1] === 30_000
    );
    expect(intervalCalls).toHaveLength(0);
    setIntervalSpy.mockRestore();
  });
});

// ── updateStrategyConfig ─────────────────────────────────────────

describe("updateStrategyConfig", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the action sends a PUT request with JSON body
   * containing the partial config updates and returns the full
   * updated config from the server response.
   */
  it("sends PUT with JSON body", async () => {
    const updatedConfig = {
      id: 1,
      enabled: false,
      max_concurrent_projects: 3,
      max_monthly_budget_usd: 50.0,
      max_per_project_budget_usd: 10.0,
      preferred_forms: [],
      excluded_forms: [],
      min_idle_workers_to_create: 1,
      record_proximity_threshold: 0.2,
      tick_interval_secs: 600,
      updated_at: "2026-02-22T11:00:00Z",
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: updatedConfig }),
    });

    const result = await updateStrategyConfig({ enabled: false, tick_interval_secs: 600 });

    expect(result).toEqual(updatedConfig);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/strategy/config"),
      expect.objectContaining({
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: false, tick_interval_secs: 600 }),
      })
    );
  });

  /**
   * Verifies that a non-ok response throws an Error with the server's
   * error message (or a fallback message if none provided).
   */
  it("throws on non-ok response", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      json: () => Promise.resolve({ error: "Unauthorized" }),
    });

    await expect(
      updateStrategyConfig({ enabled: false })
    ).rejects.toThrow("Unauthorized");
  });
});

// ── overrideDecision ─────────────────────────────────────────────

describe("overrideDecision", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that overrideDecision sends a POST with the action_taken
   * and reason fields to the decision-specific override endpoint.
   */
  it("sends POST with action_taken and reason", async () => {
    mockFetch.mockResolvedValue({ ok: true });

    await overrideDecision(42, "rejected", "Budget too high");

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/strategy/decisions/42/override"),
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action_taken: "rejected",
          reason: "Budget too high",
        }),
      })
    );
  });

  /**
   * Verifies that a failed override throws an Error for the caller
   * to handle (e.g., display a toast notification).
   */
  it("throws on failure", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      json: () => Promise.resolve({ error: "Decision already finalized" }),
    });

    await expect(
      overrideDecision(42, "rejected", "Too expensive")
    ).rejects.toThrow("Decision already finalized");
  });
});

// ── triggerStrategyTick ──────────────────────────────────────────

describe("triggerStrategyTick", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that triggerStrategyTick sends a POST to the tick endpoint
   * and returns the resulting decisions and scores from the OODA cycle.
   */
  it("sends POST, returns decisions and scores", async () => {
    const mockResult = {
      decisions: [
        {
          id: 10,
          decision_type: "scale_up",
          form: "kbn",
          summary: "Add node for kbn",
          reasoning: "High yield rate",
          params: null,
          estimated_cost_usd: 1.0,
          action_taken: "executed",
          override_reason: null,
          project_id: null,
          search_job_id: null,
          scores: null,
          created_at: "2026-02-22T10:05:00Z",
        },
      ],
      scores: [
        {
          form: "kbn",
          record_gap: 0.5,
          yield_rate: 0.9,
          cost_efficiency: 0.7,
          coverage_gap: 0.4,
          network_fit: 0.8,
          total: 3.3,
        },
      ],
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: mockResult }),
    });

    const result = await triggerStrategyTick();

    expect(result).toEqual(mockResult);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/strategy/tick"),
      expect.objectContaining({ method: "POST" })
    );
  });
});
