/**
 * @file Tests for the use-prime-verification hooks
 * @module __tests__/hooks/use-prime-verification
 *
 * Validates three hooks for the distributed prime verification system:
 *
 * - `usePrimeVerificationStats()` -- polls verification queue depth and
 *   completion counts from `/api/prime-verification/stats`. Unwraps the
 *   nested `data.ok.stats` response structure.
 *
 * - `usePrimeVerifications(primeId)` -- fetches per-prime verification
 *   history from `/api/primes/{id}/verifications`. Unwraps `data.ok.results`.
 *   Skips fetch and resets results when primeId is null.
 *
 * - `useTagDistribution()` -- fetches tag distribution data from
 *   `/api/stats/tags`. Unwraps `data.data ?? data` and guards against
 *   non-array responses.
 *
 * @see {@link ../../hooks/use-prime-verification} Source hooks
 * @see {@link ../../app/browse/page} Browse page (verification tab)
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// --- Global fetch mock ---
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

import {
  usePrimeVerificationStats,
  usePrimeVerifications,
  useTagDistribution,
} from "@/hooks/use-prime-verification";

// ── usePrimeVerificationStats ────────────────────────────────────

describe("usePrimeVerificationStats", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches verification queue stats on mount
   * from the `/api/prime-verification/stats` endpoint.
   */
  it("fetches on mount", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: {
            ok: true,
            stats: {
              pending: 10,
              claimed: 3,
              verified: 100,
              failed: 2,
              total_primes: 115,
              quorum_met: 95,
            },
          },
        }),
    });

    const { result } = renderHook(() => usePrimeVerificationStats());

    await waitFor(() => {
      expect(result.current.stats).not.toBeNull();
    });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/prime-verification/stats")
    );
  });

  /**
   * Verifies that the hook correctly parses the nested response structure.
   * The API returns `{ data: { ok: true, stats: { ... } } }` which must
   * be unwrapped: first `json.data ?? json`, then check `data.ok` before
   * extracting `data.stats`.
   */
  it("parses nested data.ok.stats structure", async () => {
    const mockStats = {
      pending: 5,
      claimed: 2,
      verified: 50,
      failed: 1,
      total_primes: 58,
      quorum_met: 48,
    };
    mockFetch.mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: { ok: true, stats: mockStats },
        }),
    });

    const { result } = renderHook(() => usePrimeVerificationStats());

    await waitFor(() => {
      expect(result.current.stats).toEqual(mockStats);
    });
    expect(result.current.stats!.pending).toBe(5);
    expect(result.current.stats!.verified).toBe(50);
    expect(result.current.stats!.quorum_met).toBe(48);
    expect(result.current.loading).toBe(false);
  });

  /**
   * Verifies that the hook polls every 30 seconds using setInterval
   * to keep the verification queue stats dashboard current.
   */
  it("polls every 30 seconds", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: { ok: true, stats: { pending: 0, claimed: 0, verified: 0, failed: 0, total_primes: 0, quorum_met: 0 } },
        }),
    });

    const setIntervalSpy = vi.spyOn(global, "setInterval");

    renderHook(() => usePrimeVerificationStats());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000);
    setIntervalSpy.mockRestore();
  });

  /**
   * Verifies that a network error does not crash the hook. The stats
   * remain null (initial state) and loading transitions to false.
   */
  it("handles error gracefully", async () => {
    mockFetch.mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => usePrimeVerificationStats());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.stats).toBeNull();
  });
});

// ── usePrimeVerifications ────────────────────────────────────────

describe("usePrimeVerifications", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches verification history when a valid
   * primeId is provided. The API endpoint is `/api/primes/{id}/verifications`.
   */
  it("fetches when primeId provided", async () => {
    const mockResults = [
      {
        id: 1,
        prime_id: 42,
        status: "verified",
        claimed_by: "node-alpha",
        claimed_at: "2026-02-22T09:00:00Z",
        completed_at: "2026-02-22T09:05:00Z",
        verification_tier: 1,
        verification_method: "deterministic",
        result_detail: null,
        error_reason: null,
      },
    ];
    mockFetch.mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: { ok: true, results: mockResults },
        }),
    });

    const { result } = renderHook(() => usePrimeVerifications(42));

    await waitFor(() => {
      expect(result.current.results).toHaveLength(1);
    });
    expect(result.current.results[0].status).toBe("verified");
    expect(result.current.results[0].verification_method).toBe("deterministic");
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/primes/42/verifications")
    );
  });

  /**
   * Verifies that the hook skips the fetch when primeId is null.
   * This happens when no prime is selected in the browse page.
   */
  it("skips fetch when primeId is null", async () => {
    const { result } = renderHook(() => usePrimeVerifications(null));

    // Give the effect time to run
    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockFetch).not.toHaveBeenCalled();
    expect(result.current.results).toEqual([]);
  });

  /**
   * Verifies that changing primeId from a number to null clears the
   * results array. This resets the verification panel when the user
   * deselects a prime.
   */
  it("resets results when primeId changes to null", async () => {
    const mockResults = [
      {
        id: 1,
        prime_id: 42,
        status: "verified",
        claimed_by: null,
        claimed_at: null,
        completed_at: null,
        verification_tier: null,
        verification_method: null,
        result_detail: null,
        error_reason: null,
      },
    ];
    mockFetch.mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: { ok: true, results: mockResults },
        }),
    });

    const { result, rerender } = renderHook(
      ({ primeId }) => usePrimeVerifications(primeId),
      { initialProps: { primeId: 42 as number | null } }
    );

    await waitFor(() => {
      expect(result.current.results).toHaveLength(1);
    });

    // Change primeId to null
    rerender({ primeId: null });

    await waitFor(() => {
      expect(result.current.results).toEqual([]);
    });
  });
});

// ── useTagDistribution ───────────────────────────────────────────

describe("useTagDistribution", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches tag distribution on mount from
   * the `/api/stats/tags` endpoint.
   */
  it("fetches tags on mount", async () => {
    const mockTags = [
      { tag: "factorial", count: 100 },
      { tag: "kbn", count: 75 },
      { tag: "palindromic", count: 50 },
    ];
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: mockTags }),
    });

    const { result } = renderHook(() => useTagDistribution());

    await waitFor(() => {
      expect(result.current.tags).toHaveLength(3);
    });
    expect(result.current.tags[0].tag).toBe("factorial");
    expect(result.current.tags[0].count).toBe(100);
    expect(result.current.tags[2].tag).toBe("palindromic");
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/stats/tags")
    );
  });

  /**
   * Verifies that a non-array response is handled gracefully by
   * returning an empty array. The hook uses `Array.isArray(data) ? data : []`
   * to guard against malformed responses (e.g., `{ error: "..." }`).
   */
  it("handles non-array response", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ message: "no tags yet" }),
    });

    const { result } = renderHook(() => useTagDistribution());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // Should be empty array, not the object
    expect(result.current.tags).toEqual([]);
  });

  /**
   * Verifies that the hook exposes a refetch function for on-demand
   * refresh of the tag distribution data.
   */
  it("returns refetch function", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([{ tag: "factorial", count: 10 }]),
    });

    const { result } = renderHook(() => useTagDistribution());

    await waitFor(() => {
      expect(result.current.tags).toHaveLength(1);
    });

    expect(typeof result.current.refetch).toBe("function");

    // Call refetch and verify it triggers another fetch
    mockFetch.mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve([
          { tag: "factorial", count: 10 },
          { tag: "kbn", count: 5 },
        ]),
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.tags).toHaveLength(2);
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});
