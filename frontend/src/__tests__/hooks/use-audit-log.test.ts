/**
 * @file Tests for the use-audit-log hook
 * @module __tests__/hooks/use-audit-log
 *
 * Validates the admin audit log hook which fetches paginated and filtered
 * audit log entries from the admin API via `adminFetch`. Supports filtering
 * by action type and user ID, with configurable page size and offset.
 *
 * The hook uses `adminFetch` from `@/lib/api` which auto-attaches the
 * Supabase JWT for admin authentication. The `RequireAdmin` extractor on
 * the Rust backend rejects requests without a valid admin token.
 *
 * @see {@link ../../hooks/use-audit-log} Source hook
 * @see {@link ../../lib/api} adminFetch wrapper
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// --- adminFetch mock ---
const mockAdminFetch = vi.fn();
vi.mock("@/lib/api", () => ({
  adminFetch: (...args: unknown[]) => mockAdminFetch(...args),
}));

import { useAuditLog } from "@/hooks/use-audit-log";

/** Factory for creating a successful adminFetch response. */
function makeOkResponse(data: unknown) {
  return {
    ok: true,
    json: () => Promise.resolve(data),
  };
}

/** Factory for creating a failed adminFetch response. */
function makeErrorResponse(status: number, error: string) {
  return {
    ok: false,
    status,
    json: () => Promise.resolve({ error }),
  };
}

describe("useAuditLog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /**
   * Verifies that the hook fetches audit log entries on mount with
   * default pagination params (limit=50, offset=0 for page 1).
   */
  it("fetches on mount with default params (limit=50, offset=0)", async () => {
    mockAdminFetch.mockResolvedValue(
      makeOkResponse({ entries: [], total: 0 })
    );

    renderHook(() => useAuditLog());

    await waitFor(() => {
      expect(mockAdminFetch).toHaveBeenCalledTimes(1);
    });

    const url = mockAdminFetch.mock.calls[0][0] as string;
    expect(url).toContain("/api/audit");
    expect(url).toContain("limit=50");
    expect(url).toContain("offset=0");
  });

  /**
   * Verifies that custom page and limit values are translated to the
   * correct query parameters. Page 3 with limit 20 = offset 40.
   */
  it("passes page/limit as query params", async () => {
    mockAdminFetch.mockResolvedValue(
      makeOkResponse({ entries: [], total: 0 })
    );

    renderHook(() => useAuditLog({ page: 3, limit: 20 }));

    await waitFor(() => {
      expect(mockAdminFetch).toHaveBeenCalledTimes(1);
    });

    const url = mockAdminFetch.mock.calls[0][0] as string;
    expect(url).toContain("limit=20");
    expect(url).toContain("offset=40");
  });

  /**
   * Verifies that the action filter is included as a query parameter
   * when provided. This allows filtering by action type (e.g., "login",
   * "create_project", "delete_search").
   */
  it("includes action filter in params", async () => {
    mockAdminFetch.mockResolvedValue(
      makeOkResponse({ entries: [], total: 0 })
    );

    renderHook(() => useAuditLog({ action: "login" }));

    await waitFor(() => {
      expect(mockAdminFetch).toHaveBeenCalledTimes(1);
    });

    const url = mockAdminFetch.mock.calls[0][0] as string;
    expect(url).toContain("action=login");
  });

  /**
   * Verifies that the userId filter is included as a query parameter
   * when provided. This allows narrowing audit entries to a specific user.
   */
  it("includes userId filter in params", async () => {
    mockAdminFetch.mockResolvedValue(
      makeOkResponse({ entries: [], total: 0 })
    );

    renderHook(() => useAuditLog({ userId: "user-abc-123" }));

    await waitFor(() => {
      expect(mockAdminFetch).toHaveBeenCalledTimes(1);
    });

    const url = mockAdminFetch.mock.calls[0][0] as string;
    expect(url).toContain("user_id=user-abc-123");
  });

  /**
   * Verifies that the hook correctly sets entries and total from the
   * API response data. The response may be wrapped in a `{ data: ... }`
   * envelope which the hook unwraps via `json.data ?? json`.
   */
  it("sets entries and total from response", async () => {
    const mockEntries = [
      {
        id: 1,
        user_id: "user-1",
        user_email: "alice@example.com",
        action: "login",
        resource: null,
        method: "POST",
        status_code: 200,
        ip_address: "192.168.1.1",
        user_agent: "Mozilla/5.0",
        payload: null,
        created_at: "2026-02-22T10:00:00Z",
      },
      {
        id: 2,
        user_id: "user-1",
        user_email: "alice@example.com",
        action: "create_project",
        resource: "project:42",
        method: "POST",
        status_code: 201,
        ip_address: "192.168.1.1",
        user_agent: "Mozilla/5.0",
        payload: { name: "factorial hunt" },
        created_at: "2026-02-22T10:05:00Z",
      },
    ];

    mockAdminFetch.mockResolvedValue(
      makeOkResponse({ data: { entries: mockEntries, total: 42 } })
    );

    const { result } = renderHook(() => useAuditLog());

    await waitFor(() => {
      expect(result.current.entries).toHaveLength(2);
    });
    expect(result.current.entries[0].action).toBe("login");
    expect(result.current.entries[1].action).toBe("create_project");
    expect(result.current.total).toBe(42);
    expect(result.current.isLoading).toBe(false);
  });

  /**
   * Verifies that a non-ok response sets the error state with the
   * server's error message from the JSON body.
   */
  it("sets error on non-ok response", async () => {
    mockAdminFetch.mockResolvedValue(
      makeErrorResponse(403, "Forbidden: admin access required")
    );

    const { result } = renderHook(() => useAuditLog());

    await waitFor(() => {
      expect(result.current.error).toBe("Forbidden: admin access required");
    });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.entries).toEqual([]);
    expect(result.current.total).toBe(0);
  });

  /**
   * Verifies that a successful refetch clears a previously set error.
   * This ensures the UI removes error banners when data loads correctly.
   */
  it("clears error on successful refetch", async () => {
    // First call: error
    mockAdminFetch.mockResolvedValueOnce(
      makeErrorResponse(500, "Internal server error")
    );

    const { result } = renderHook(() => useAuditLog());

    await waitFor(() => {
      expect(result.current.error).toBe("Internal server error");
    });

    // Second call: success
    mockAdminFetch.mockResolvedValueOnce(
      makeOkResponse({ entries: [{ id: 1, user_id: "u1", user_email: null, action: "login", resource: null, method: "POST", status_code: 200, ip_address: null, user_agent: null, payload: null, created_at: "2026-02-22T12:00:00Z" }], total: 1 })
    );

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.entries).toHaveLength(1);
    expect(result.current.total).toBe(1);
  });

  /**
   * Verifies the isLoading state transitions: starts true, becomes
   * false after fetch completes (success or error).
   */
  it("isLoading transitions correctly", async () => {
    mockAdminFetch.mockResolvedValue(
      makeOkResponse({ entries: [], total: 0 })
    );

    const { result } = renderHook(() => useAuditLog());

    // isLoading starts true (initial state)
    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
  });
});
