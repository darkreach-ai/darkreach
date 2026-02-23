/**
 * @file Tests for the use-earnings hook
 * @module __tests__/hooks/use-earnings
 *
 * Validates the operator earnings hook which fetches credit history and
 * monthly earnings from the REST API. The hook requires JWT authentication
 * (Supabase session) and fetches two endpoints in parallel:
 * - `/api/v1/operators/me/credits` -- paginated credit transaction rows
 * - `/api/v1/operators/me/earnings` -- 12-month earnings aggregation
 *
 * Auth-guarded: when no session is present, all fetches are skipped.
 * Polls every 30 seconds to keep the earnings dashboard current.
 *
 * @see {@link ../../hooks/use-earnings} Source hook
 * @see {@link ../../app/earnings/page} Earnings dashboard page
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
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

import { useEarnings } from "@/hooks/use-earnings";

describe("useEarnings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSessionValue = { access_token: "test-jwt-token" };
    // Default: both endpoints succeed with empty arrays
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });
  });

  /**
   * Verifies that when there is no session token, the hook skips
   * all fetch calls. This prevents 401 errors for unauthenticated users.
   */
  it("skips fetch when no session token", async () => {
    mockSessionValue = null;

    const { result } = renderHook(() => useEarnings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // No fetch calls should have been made (the callbacks early-return)
    expect(mockFetch).not.toHaveBeenCalled();
  });

  /**
   * Verifies that both endpoints are fetched in parallel on mount when
   * a valid session is present. The hook uses Promise.all to avoid
   * sequential waterfall requests.
   */
  it("fetches both endpoints in parallel on mount", async () => {
    const { result } = renderHook(() => useEarnings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockFetch).toHaveBeenCalledTimes(2);
    const urls = mockFetch.mock.calls.map((c: unknown[]) => c[0]);
    expect(urls).toContainEqual(
      expect.stringContaining("/api/v1/operators/me/credits")
    );
    expect(urls).toContainEqual(
      expect.stringContaining("/api/v1/operators/me/earnings")
    );
  });

  /**
   * Verifies that the credits endpoint response populates the credits array
   * with CreditRow objects containing id, block_id, credit amount, and reason.
   */
  it("sets credits array from response", async () => {
    const mockCredits = [
      { id: 1, block_id: 10, credit: 5.0, reason: "block_completed", granted_at: "2026-02-20T12:00:00Z" },
      { id: 2, block_id: 11, credit: 3.0, reason: "block_completed", granted_at: "2026-02-21T12:00:00Z" },
    ];
    const mockEarningsData = [
      { month: "2026-02", total_credits: 8.0, block_count: 2 },
    ];

    mockFetch.mockImplementation((url: string) => {
      if (url.includes("/credits")) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockCredits),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve(mockEarningsData),
      });
    });

    const { result } = renderHook(() => useEarnings());

    await waitFor(() => {
      expect(result.current.credits).toHaveLength(2);
    });
    expect(result.current.credits[0].credit).toBe(5.0);
    expect(result.current.credits[1].reason).toBe("block_completed");
  });

  /**
   * Verifies that the earnings endpoint response populates the monthly
   * earnings array with month, total_credits, and block_count.
   */
  it("sets earnings array from response", async () => {
    const mockEarningsData = [
      { month: "2026-01", total_credits: 45.0, block_count: 12 },
      { month: "2026-02", total_credits: 22.5, block_count: 6 },
    ];

    mockFetch.mockImplementation((url: string) => {
      if (url.includes("/earnings")) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockEarningsData),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([]),
      });
    });

    const { result } = renderHook(() => useEarnings());

    await waitFor(() => {
      expect(result.current.earnings).toHaveLength(2);
    });
    expect(result.current.earnings[0].month).toBe("2026-01");
    expect(result.current.earnings[1].total_credits).toBe(22.5);
  });

  /**
   * Verifies that the Authorization Bearer header is attached to both
   * fetch calls using the session's access_token.
   */
  it("attaches Authorization header", async () => {
    const { result } = renderHook(() => useEarnings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    for (const call of mockFetch.mock.calls) {
      const options = call[1] as RequestInit;
      expect(options.headers).toEqual(
        expect.objectContaining({
          Authorization: "Bearer test-jwt-token",
        })
      );
    }
  });

  /**
   * Verifies that the hook sets up a 30-second polling interval to keep
   * earnings data fresh on the dashboard.
   */
  it("polls every 30 seconds", async () => {
    const setIntervalSpy = vi.spyOn(global, "setInterval");

    renderHook(() => useEarnings());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000);
    setIntervalSpy.mockRestore();
  });

  /**
   * Verifies that the hook passes limit and offset query parameters
   * to the credits endpoint for pagination support.
   */
  it("passes limit and offset params", async () => {
    const { result } = renderHook(() => useEarnings(25, 50));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const creditsCall = mockFetch.mock.calls.find(
      (c: unknown[]) => (c[0] as string).includes("/credits")
    );
    expect(creditsCall).toBeDefined();
    expect(creditsCall![0]).toContain("limit=25");
    expect(creditsCall![0]).toContain("offset=50");
  });

  /**
   * Verifies that loading transitions from true to false after the
   * parallel fetch completes.
   */
  it("loading transitions from true to false", async () => {
    const { result } = renderHook(() => useEarnings());

    // Initially loading
    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
  });
});
