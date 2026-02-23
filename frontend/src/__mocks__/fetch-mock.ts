/**
 * @module __mocks__/fetch-mock
 *
 * Test utilities for mocking the global `fetch` function. Provides helpers
 * for persistent responses, one-shot responses, ordered response sequences,
 * and error simulation. Uses `vi.stubGlobal` under the hood.
 *
 * @example
 * ```ts
 * beforeEach(() => resetFetchMock());
 * afterEach(() => vi.unstubAllGlobals());
 *
 * mockFetch({ data: [1, 2, 3] });
 * const res = await fetch("/api/foo");
 * expect(await res.json()).toEqual({ data: [1, 2, 3] });
 * ```
 */

import { vi } from "vitest";

/** Create a minimal Response-like object for the given JSON body. */
function makeResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: status === 200 ? "OK" : `HTTP ${status}`,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(JSON.stringify(body)),
    headers: new Headers(),
    clone: () => makeResponse(body, status),
  } as unknown as Response;
}

/**
 * Mock `fetch` to always return the same response body (200 OK).
 * Replaces any previous fetch mock.
 */
export function mockFetch(body: unknown, status = 200) {
  const fn = vi.fn().mockResolvedValue(makeResponse(body, status));
  vi.stubGlobal("fetch", fn);
  return fn;
}

/**
 * Mock `fetch` to return the given body once, then reject with
 * "No more mocked responses" on subsequent calls.
 */
export function mockFetchOnce(body: unknown, status = 200) {
  const fn = vi
    .fn()
    .mockResolvedValueOnce(makeResponse(body, status))
    .mockRejectedValue(new Error("No more mocked responses"));
  vi.stubGlobal("fetch", fn);
  return fn;
}

/**
 * Mock `fetch` to return responses from an ordered sequence.
 * Each call consumes the next response in the array. Useful for
 * testing hooks that issue multiple parallel fetches.
 */
export function mockFetchSequence(
  responses: Array<{ body: unknown; status?: number }>
) {
  let idx = 0;
  const fn = vi.fn().mockImplementation(() => {
    if (idx >= responses.length) {
      return Promise.reject(new Error("No more mocked responses"));
    }
    const { body, status = 200 } = responses[idx++];
    return Promise.resolve(makeResponse(body, status));
  });
  vi.stubGlobal("fetch", fn);
  return fn;
}

/**
 * Mock `fetch` to reject with a network error.
 */
export function mockFetchError(message = "Network error") {
  const fn = vi.fn().mockRejectedValue(new Error(message));
  vi.stubGlobal("fetch", fn);
  return fn;
}

/**
 * Reset the fetch mock to a default no-op that returns empty 200 JSON.
 * Call in `beforeEach` to ensure test isolation.
 */
export function resetFetchMock() {
  const fn = vi.fn().mockResolvedValue(makeResponse({}));
  vi.stubGlobal("fetch", fn);
  return fn;
}
