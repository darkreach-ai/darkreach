/**
 * @file Tests for the use-marketplace hook
 * @module __tests__/hooks/use-marketplace
 *
 * Validates the marketplace data hook which fetches active search form
 * statistics and credit conversion rates from two public REST endpoints:
 * - `/api/v1/marketplace/forms` -- per-form job/block counts (direct JSON)
 * - `/api/resources/rates` -- credit rates (unwraps `data.data ?? data`)
 *
 * Both endpoints are public (no auth required). The hook fetches both
 * in parallel on mount and polls every 30 seconds.
 *
 * @see {@link ../../hooks/use-marketplace} Source hook
 * @see {@link ../../app/marketplace/page} Marketplace overview page
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

// --- Format mock: remove API_BASE prefix for test fetch URLs ---
vi.mock("@/lib/format", () => ({ API_BASE: "" }));

// --- Global fetch mock ---
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

import { useMarketplace } from "@/hooks/use-marketplace";

describe("useMarketplace", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that both the forms and rates endpoints are fetched in
   * parallel on mount. The hook uses Promise.all to avoid sequential
   * waterfall requests.
   */
  it("fetches both endpoints on mount", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });

    const { result } = renderHook(() => useMarketplace());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockFetch).toHaveBeenCalledTimes(2);
    const urls = mockFetch.mock.calls.map((c: unknown[]) => c[0]);
    expect(urls).toContainEqual(
      expect.stringContaining("/api/v1/marketplace/forms")
    );
    expect(urls).toContainEqual(
      expect.stringContaining("/api/resources/rates")
    );
  });

  /**
   * Verifies that the forms endpoint response directly populates the
   * forms array (no unwrapping needed -- the JSON is the array itself).
   */
  it("sets forms from response", async () => {
    const mockForms = [
      { form: "factorial", job_count: 3, total_blocks: 100, completed_blocks: 75 },
      { form: "kbn", job_count: 5, total_blocks: 200, completed_blocks: 150 },
    ];

    mockFetch.mockImplementation((url: string) => {
      if (url.includes("/marketplace/forms")) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve(mockForms),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([]),
      });
    });

    const { result } = renderHook(() => useMarketplace());

    await waitFor(() => {
      expect(result.current.forms).toHaveLength(2);
    });
    expect(result.current.forms[0].form).toBe("factorial");
    expect(result.current.forms[1].completed_blocks).toBe(150);
  });

  /**
   * Verifies that the rates endpoint response is unwrapped via
   * `data.data ?? data`. The API wraps rates in a `{ data: [...] }`
   * envelope, which the hook strips before setting state.
   */
  it("sets rates with data.data unwrapping", async () => {
    const mockRates = [
      { resource_type: "cpu_hour", credits_per_unit: 1.0, unit_label: "CPU-hour", updated_at: "2026-02-22T00:00:00Z" },
      { resource_type: "gpu_hour", credits_per_unit: 5.0, unit_label: "GPU-hour", updated_at: "2026-02-22T00:00:00Z" },
    ];

    mockFetch.mockImplementation((url: string) => {
      if (url.includes("/resources/rates")) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ data: mockRates }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve([]),
      });
    });

    const { result } = renderHook(() => useMarketplace());

    await waitFor(() => {
      expect(result.current.rates).toHaveLength(2);
    });
    expect(result.current.rates[0].resource_type).toBe("cpu_hour");
    expect(result.current.rates[1].credits_per_unit).toBe(5.0);
  });

  /**
   * Verifies that the hook sets up a 30-second polling interval to
   * keep marketplace data fresh.
   */
  it("polls every 30 seconds", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });

    const setIntervalSpy = vi.spyOn(global, "setInterval");

    renderHook(() => useMarketplace());

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalled();
    });

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000);
    setIntervalSpy.mockRestore();
  });

  /**
   * Verifies that a fetch error does not crash the hook and preserves
   * the previous state. The hook catches network errors and silently
   * keeps existing data, which is appropriate for a polling scenario.
   */
  it("handles fetch error gracefully and keeps previous state", async () => {
    mockFetch.mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => useMarketplace());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // Forms and rates remain empty arrays (initial state) on error
    expect(result.current.forms).toEqual([]);
    expect(result.current.rates).toEqual([]);
  });

  /**
   * Verifies that loading transitions from true to false after the
   * initial parallel fetch completes.
   */
  it("loading transitions correctly", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });

    const { result } = renderHook(() => useMarketplace());

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
  });

  /**
   * Verifies that the hook exposes a refetch function for on-demand
   * data refresh, independent of the polling interval.
   */
  it("returns refetch function", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([]),
    });

    const { result } = renderHook(() => useMarketplace());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(typeof result.current.refetch).toBe("function");
  });
});
