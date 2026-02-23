/**
 * @file Tests for the use-operator-resources hook
 * @module __tests__/hooks/use-operator-resources
 *
 * Validates the operator stats hook which fetches the authenticated
 * operator's account summary from `/api/v1/operators/stats`. Requires
 * JWT authentication (Supabase session). Returns username, credit balance,
 * primes found, trust level, and leaderboard rank.
 *
 * Auth-guarded: when no session is present, all fetches are skipped.
 * Polls every 30 seconds to keep the earnings dashboard current.
 *
 * @see {@link ../../hooks/use-operator-resources} Source hook
 * @see {@link ../../app/earnings/page} Earnings dashboard page
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// --- Auth mock: simulate Supabase session with JWT ---
let mockSessionValue: { access_token: string } | null = { access_token: "test-jwt-token" };
vi.mock("@/contexts/auth-context", () => ({
  useAuth: () => ({ session: mockSessionValue }),
}));

// --- Format mock: remove API_BASE prefix for test fetch URLs ---
vi.mock("@/lib/format", () => ({ API_BASE: "" }));

// --- Global fetch mock ---
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

import { useOperatorStats } from "@/hooks/use-operator-resources";

describe("useOperatorStats", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSessionValue = { access_token: "test-jwt-token" };
  });

  /**
   * Verifies that when there is no session token, the hook skips the
   * fetch call entirely. This prevents 401 errors for logged-out users.
   */
  it("skips fetch when no session", async () => {
    mockSessionValue = null;

    const { result } = renderHook(() => useOperatorStats());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockFetch).not.toHaveBeenCalled();
  });

  /**
   * Verifies that the hook fetches the operator stats endpoint with
   * the Authorization Bearer header on mount.
   */
  it("fetches stats endpoint with auth header", async () => {
    const mockStats = {
      username: "alice",
      credit: 150.0,
      primes_found: 42,
      trust_level: 3,
      rank: 7,
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockStats),
    });

    const { result } = renderHook(() => useOperatorStats());

    await waitFor(() => {
      expect(result.current.stats).not.toBeNull();
    });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/operators/stats"),
      expect.objectContaining({
        headers: { Authorization: "Bearer test-jwt-token" },
      })
    );
  });

  /**
   * Verifies that the operator stats are correctly set from the
   * response JSON, including username, credit balance, prime count,
   * trust level, and leaderboard rank.
   */
  it("sets stats from response", async () => {
    const mockStats = {
      username: "bob",
      credit: 75.5,
      primes_found: 18,
      trust_level: 2,
      rank: 15,
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockStats),
    });

    const { result } = renderHook(() => useOperatorStats());

    await waitFor(() => {
      expect(result.current.stats).toEqual(mockStats);
    });
    expect(result.current.stats!.username).toBe("bob");
    expect(result.current.stats!.credit).toBe(75.5);
    expect(result.current.stats!.primes_found).toBe(18);
  });

  /**
   * Verifies that the hook sets up a 30-second polling interval after
   * the initial fetch to keep stats current.
   */
  it("polls every 30 seconds", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ username: "alice", credit: 100, primes_found: 10, trust_level: 1, rank: 20 }),
    });

    const setIntervalSpy = vi.spyOn(global, "setInterval");

    renderHook(() => useOperatorStats());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000);
    setIntervalSpy.mockRestore();
  });

  /**
   * Verifies that loading transitions from true to false after the
   * initial fetch completes.
   */
  it("loading transitions correctly", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ username: "alice", credit: 50, primes_found: 5, trust_level: 1, rank: 30 }),
    });

    const { result } = renderHook(() => useOperatorStats());

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
  });

  /**
   * Verifies that a fetch error does not crash the hook and preserves
   * the previous state (null stats). The hook catches errors silently
   * and keeps the existing value, appropriate for a polling scenario.
   */
  it("handles fetch error", async () => {
    mockFetch.mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => useOperatorStats());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // Stats remain null (initial state) on error
    expect(result.current.stats).toBeNull();
  });
});
