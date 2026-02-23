/**
 * @file Tests for the adminFetch authenticated API wrapper
 * @module __tests__/lib/api
 *
 * Validates that adminFetch correctly integrates with Supabase Auth to
 * attach JWT Bearer tokens to outgoing fetch requests. The wrapper reads
 * the current session via `supabase.auth.getSession()` and, when an
 * access_token is present, sets the `Authorization: Bearer <token>` header.
 * All paths are prefixed with the NEXT_PUBLIC_API_URL (API_BASE).
 *
 * Tests cover: token attachment, missing session handling, URL prefixing,
 * request option pass-through, and response forwarding.
 *
 * @see {@link ../../lib/api} Source module
 * @see {@link ../../lib/supabase} Supabase client singleton (mocked)
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

// Hoist mock functions so they are available when vi.mock factories execute
// (vi.mock calls are hoisted to the top of the file by Vitest).
const { mockGetSession, mockFetch } = vi.hoisted(() => ({
  mockGetSession: vi.fn(),
  mockFetch: vi.fn(),
}));

// Mock the Supabase client module with a controllable getSession stub.
vi.mock("@/lib/supabase", () => ({
  supabase: {
    auth: {
      getSession: mockGetSession,
    },
  },
}));

// Mock global fetch to capture and inspect outgoing requests.
vi.stubGlobal("fetch", mockFetch);

import { adminFetch } from "@/lib/api";

describe("adminFetch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: successful fetch returning a 200 Response.
    mockFetch.mockResolvedValue(new Response("{}", { status: 200 }));
  });

  /** Verifies the Authorization header is set when a session with access_token exists. */
  it("attaches Authorization header when session has access_token", async () => {
    mockGetSession.mockResolvedValue({
      data: { session: { access_token: "test-jwt-token-123" } },
    });

    await adminFetch("/api/status");

    expect(mockFetch).toHaveBeenCalledOnce();
    const [, options] = mockFetch.mock.calls[0];
    const headers = new Headers(options.headers);
    expect(headers.get("Authorization")).toBe("Bearer test-jwt-token-123");
  });

  /** Verifies the Authorization header is omitted when no session exists. */
  it("omits Authorization header when no session", async () => {
    mockGetSession.mockResolvedValue({
      data: { session: null },
    });

    await adminFetch("/api/status");

    expect(mockFetch).toHaveBeenCalledOnce();
    const [, options] = mockFetch.mock.calls[0];
    const headers = new Headers(options.headers);
    expect(headers.get("Authorization")).toBeNull();
  });

  /** Verifies the API_BASE prefix is prepended to the request path. */
  it("prepends API_BASE to the path", async () => {
    mockGetSession.mockResolvedValue({
      data: { session: null },
    });

    await adminFetch("/api/agents/tasks");

    const [url] = mockFetch.mock.calls[0];
    // API_BASE defaults to "" when NEXT_PUBLIC_API_URL is not set in test env.
    expect(url).toContain("/api/agents/tasks");
  });

  /** Verifies that request options (method, body) are passed through to fetch. */
  it("passes through request options like method and body", async () => {
    mockGetSession.mockResolvedValue({
      data: { session: { access_token: "token" } },
    });

    const body = JSON.stringify({ name: "test-task" });
    await adminFetch("/api/agents/tasks", {
      method: "POST",
      body,
    });

    expect(mockFetch).toHaveBeenCalledOnce();
    const [, options] = mockFetch.mock.calls[0];
    expect(options.method).toBe("POST");
    expect(options.body).toBe(body);
  });

  /** Verifies graceful handling when session data structure has no access_token. */
  it("handles missing session data gracefully", async () => {
    mockGetSession.mockResolvedValue({
      data: { session: { /* no access_token */ } },
    });

    await adminFetch("/api/health");

    expect(mockFetch).toHaveBeenCalledOnce();
    const [, options] = mockFetch.mock.calls[0];
    const headers = new Headers(options.headers);
    expect(headers.get("Authorization")).toBeNull();
  });

  /** Verifies the raw fetch Response is returned to the caller. */
  it("returns the fetch Response object", async () => {
    const mockResponse = new Response('{"ok":true}', { status: 200 });
    mockFetch.mockResolvedValue(mockResponse);
    mockGetSession.mockResolvedValue({
      data: { session: null },
    });

    const result = await adminFetch("/api/health");
    expect(result).toBe(mockResponse);
  });
});
