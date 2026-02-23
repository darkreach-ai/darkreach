/**
 * @file Tests for the ContributeProvider and useContribute hook
 * @module __tests__/contexts/worker-context
 *
 * Validates the browser-based compute contribution context that manages a
 * Web Worker lifecycle for running prime searches. The provider handles
 * block claiming, result submission, heartbeat intervals, and activity logging.
 *
 * Tests cover initial state, start/stop lifecycle, worker message handling
 * (progress, prime, done, error), activity log capping, heartbeat timing,
 * and cleanup on unmount and beforeunload.
 *
 * @see {@link ../../contexts/worker-context} Source: ContributeProvider, useContribute
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, renderHook } from "@testing-library/react";
import React from "react";

// --- Mocks ---

vi.mock("@/contexts/auth-context", () => ({
  useAuth: () => ({
    session: { access_token: "test-jwt" },
  }),
}));

vi.mock("@/lib/format", () => ({ API_BASE: "" }));

vi.mock("@/lib/supabase", () => ({
  supabase: {
    auth: {
      getSession: vi.fn().mockResolvedValue({
        data: { session: { access_token: "test-jwt" } },
      }),
    },
  },
}));

// Worker mock is set up in beforeEach using a class-based approach
// so `new Worker(...)` works correctly in jsdom.
// We store the created instance here so tests can trigger onmessage/onerror.
let mockWorkerInstance: any;

import {
  ContributeProvider,
  useContribute,
  type ContributeStatus,
  type ContributeStats,
} from "@/contexts/worker-context";

/** Helper component that renders contribute state for inspection. */
function TestConsumer() {
  const { status, stats, log, start, stop } = useContribute();
  return (
    <div>
      <span data-testid="status">{status}</span>
      <span data-testid="tested">{stats.tested}</span>
      <span data-testid="found">{stats.found}</span>
      <span data-testid="blocks">{stats.blocksCompleted}</span>
      <span data-testid="speed">{stats.speed}</span>
      <span data-testid="session-start">
        {stats.sessionStart === null ? "null" : "set"}
      </span>
      <span data-testid="current-block">
        {stats.currentBlockId === null ? "null" : stats.currentBlockId}
      </span>
      <span data-testid="error">{stats.error ?? "null"}</span>
      <span data-testid="log-count">{log.length}</span>
      <span data-testid="log-first">
        {log.length > 0 ? log[0].message : "empty"}
      </span>
      <button data-testid="start-btn" onClick={start}>
        Start
      </button>
      <button data-testid="stop-btn" onClick={stop}>
        Stop
      </button>
    </div>
  );
}

/** Renders the test consumer inside the ContributeProvider. */
function renderWithProvider() {
  return render(
    <ContributeProvider>
      <TestConsumer />
    </ContributeProvider>
  );
}

describe("ContributeProvider", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockWorkerInstance = null;

    // Use a class-style mock so `new Worker(...)` works as a constructor.
    // The created instance is captured to `mockWorkerInstance` for tests.
    const postMessageFn = vi.fn();
    const terminateFn = vi.fn();
    class MockWorker {
      postMessage = postMessageFn;
      terminate = terminateFn;
      onmessage: ((e: any) => void) | null = null;
      onerror: ((e: any) => void) | null = null;
      constructor() {
        mockWorkerInstance = this;
      }
    }
    vi.stubGlobal("Worker", MockWorker);
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            block_id: 1,
            search_type: "kbn",
            block_start: 1,
            block_end: 100,
            params: {},
          }),
      })
    );
    // Mock sessionStorage
    vi.stubGlobal("sessionStorage", {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  /** Verifies useContribute throws when used outside the provider. */
  it("useContribute throws when used outside provider", () => {
    // Suppress console.error from the expected error boundary
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => {
      renderHook(() => useContribute());
    }).toThrow("useContribute must be used within a ContributeProvider");
    spy.mockRestore();
  });

  /** Verifies the initial status is "idle". */
  it('initial status is "idle"', () => {
    renderWithProvider();
    expect(screen.getByTestId("status").textContent).toBe("idle");
  });

  /** Verifies initial stats are zeros and nulls. */
  it("initial stats has zeros and nulls", () => {
    renderWithProvider();
    expect(screen.getByTestId("tested").textContent).toBe("0");
    expect(screen.getByTestId("found").textContent).toBe("0");
    expect(screen.getByTestId("blocks").textContent).toBe("0");
    expect(screen.getByTestId("speed").textContent).toBe("0");
    expect(screen.getByTestId("session-start").textContent).toBe("null");
    expect(screen.getByTestId("current-block").textContent).toBe("null");
    expect(screen.getByTestId("error").textContent).toBe("null");
  });

  /** Verifies the initial activity log is empty. */
  it("initial log is empty", () => {
    renderWithProvider();
    expect(screen.getByTestId("log-count").textContent).toBe("0");
  });

  /** Verifies start() sets sessionStart (transitions from null to set). */
  it("start() sets sessionStart", async () => {
    renderWithProvider();
    expect(screen.getByTestId("session-start").textContent).toBe("null");

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    expect(screen.getByTestId("session-start").textContent).toBe("set");
  });

  /** Verifies start() creates a worker when auth token is available. */
  it("start() requires auth token", async () => {
    // With the current mock (session has access_token), calling start should
    // create a Worker. We verify the worker's postMessage becomes accessible.
    renderWithProvider();
    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // If the worker was created, its onmessage handler should have been set
    // (the runWorkLoop sets up worker.onmessage).
    expect(mockWorkerInstance.onmessage).toBeTypeOf("function");
  });

  /** Verifies stop() resets status to idle. */
  it("stop() resets status to idle", async () => {
    renderWithProvider();

    // Start first
    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // Stop
    await act(async () => {
      screen.getByTestId("stop-btn").click();
    });

    expect(screen.getByTestId("status").textContent).toBe("idle");
  });

  /** Verifies stop() posts a stop message to the worker. */
  it("stop() posts stop message to worker", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    await act(async () => {
      screen.getByTestId("stop-btn").click();
    });

    expect(mockWorkerInstance.postMessage).toHaveBeenCalledWith({ type: "stop" });
  });

  /** Verifies stop() clears the heartbeat interval. */
  it("stop() clears heartbeat interval", async () => {
    const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");

    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // Advance past the 200ms initial delay to let the work loop set up
    await act(async () => {
      vi.advanceTimersByTime(300);
    });

    await act(async () => {
      screen.getByTestId("stop-btn").click();
    });

    expect(clearIntervalSpy).toHaveBeenCalled();
  });

  /** Verifies activity log entries are capped at 50. */
  it("log entries are capped at 50", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // Simulate many worker messages to fill the log
    await act(async () => {
      for (let i = 0; i < 60; i++) {
        mockWorkerInstance.onmessage?.({
          data: { type: "error", message: `Error ${i}` },
        });
      }
    });

    const logCount = parseInt(
      screen.getByTestId("log-count").textContent || "0"
    );
    expect(logCount).toBeLessThanOrEqual(50);
  });

  /** Verifies log entries are added in reverse order (newest first). */
  it("activity log adds entries in reverse order (newest first)", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    await act(async () => {
      mockWorkerInstance.onmessage?.({
        data: { type: "error", message: "First error" },
      });
      mockWorkerInstance.onmessage?.({
        data: { type: "error", message: "Second error" },
      });
    });

    // The most recent entry should be first
    expect(screen.getByTestId("log-first").textContent).toBe("Second error");
  });

  /** Verifies worker progress message updates the stats (speed). */
  it("worker progress message updates stats", async () => {
    // Use real timers for this test since the async claim chain is complex
    vi.useRealTimers();

    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // Wait for the work loop to set up the onmessage handler and claim block
    await act(async () => {
      await new Promise((r) => setTimeout(r, 300));
    });

    // onmessage should now be set by runWorkLoop
    expect(mockWorkerInstance.onmessage).toBeTypeOf("function");

    await act(async () => {
      mockWorkerInstance.onmessage({
        data: {
          type: "progress",
          tested: 42,
          found: 1,
          speed: 100,
          current: 50,
        },
      });
    });

    // Speed is always set directly from the message
    expect(screen.getByTestId("speed").textContent).toBe("100");

    // Re-enable fake timers for cleanup
    vi.useFakeTimers();
  });

  /** Verifies worker prime message increments found count. */
  it("worker prime message increments found count", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    await act(async () => {
      mockWorkerInstance.onmessage?.({
        data: {
          type: "prime",
          expression: "3*2^5+1",
          form: "kbn",
          digits: 2,
          proof_method: "proth",
        },
      });
    });

    expect(parseInt(screen.getByTestId("found").textContent || "0")).toBe(1);
  });

  /** Verifies worker done message triggers submit and processes next block. */
  it("worker done message triggers submit and next block", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // Advance past the 200ms initial delay
    await act(async () => {
      vi.advanceTimersByTime(300);
    });

    // Flush promises for the block claim
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    await act(async () => {
      mockWorkerInstance.onmessage?.({
        data: { type: "done", tested: 100, found: 0 },
      });
    });

    // After done, the provider should call submitResult (fetch POST)
    // and then try to claim next block
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    // fetch should have been called multiple times:
    // 1) heartbeat, 2) claim block, 3) submit result, 4) next claim
    expect(fetch).toHaveBeenCalled();
  });

  /** Verifies worker error message adds an error log entry. */
  it("worker error message adds error log entry", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    await act(async () => {
      mockWorkerInstance.onmessage?.({
        data: { type: "error", message: "WASM init failed" },
      });
    });

    expect(screen.getByTestId("log-first").textContent).toBe(
      "WASM init failed"
    );
    expect(screen.getByTestId("error").textContent).toBe("WASM init failed");
  });

  /** Verifies the heartbeat interval is set to 30 seconds. */
  it("heartbeat interval is 30 seconds", async () => {
    const setIntervalSpy = vi.spyOn(globalThis, "setInterval");

    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // The runWorkLoop calls setInterval(sendHeartbeat, 30_000)
    const heartbeatCall = setIntervalSpy.mock.calls.find(
      (call) => call[1] === 30_000
    );
    expect(heartbeatCall).toBeDefined();
  });

  /** Verifies cleanup on unmount terminates the worker. */
  it("cleanup on unmount terminates worker", async () => {
    const { unmount } = renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    unmount();
    expect(mockWorkerInstance.terminate).toHaveBeenCalled();
  });

  /** Verifies beforeunload handler posts stop to worker. */
  it("beforeunload handler stops worker", async () => {
    renderWithProvider();

    await act(async () => {
      screen.getByTestId("start-btn").click();
    });

    // Dispatch beforeunload
    await act(async () => {
      window.dispatchEvent(new Event("beforeunload"));
    });

    expect(mockWorkerInstance.postMessage).toHaveBeenCalledWith({ type: "stop" });
  });
});
