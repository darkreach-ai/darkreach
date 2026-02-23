/**
 * @module __mocks__/worker-mock
 *
 * Mock Web Worker implementation for testing components that use the
 * Worker API (e.g., the ContributeProvider). Simulates the Worker
 * interface with controllable message dispatch.
 *
 * @example
 * ```ts
 * const { worker, getLastPostedMessage, simulateMessage } = createMockWorker();
 * vi.stubGlobal("Worker", vi.fn(() => worker));
 *
 * // Component posts a message to the worker
 * worker.postMessage({ type: "start", ... });
 * expect(getLastPostedMessage()).toEqual({ type: "start", ... });
 *
 * // Simulate a message from the worker back to main thread
 * simulateMessage({ type: "progress", tested: 100 });
 * ```
 */

import { vi } from "vitest";

export interface MockWorkerInstance {
  postMessage: ReturnType<typeof vi.fn>;
  terminate: ReturnType<typeof vi.fn>;
  onmessage: ((e: MessageEvent) => void) | null;
  onerror: ((e: ErrorEvent) => void) | null;
  addEventListener: ReturnType<typeof vi.fn>;
  removeEventListener: ReturnType<typeof vi.fn>;
}

export interface MockWorkerControls {
  /** The mock worker instance. */
  worker: MockWorkerInstance;
  /** Get the data from the last `postMessage` call. */
  getLastPostedMessage: () => unknown;
  /** Get all messages posted to the worker. */
  getAllPostedMessages: () => unknown[];
  /** Simulate a message event from the worker to the main thread. */
  simulateMessage: (data: unknown) => void;
  /** Simulate an error event from the worker. */
  simulateError: (message: string) => void;
}

/**
 * Creates a mock Worker instance with helper functions for testing.
 * The returned worker can be passed to `vi.stubGlobal("Worker", ...)`.
 */
export function createMockWorker(): MockWorkerControls {
  const postedMessages: unknown[] = [];

  const worker: MockWorkerInstance = {
    postMessage: vi.fn((data: unknown) => {
      postedMessages.push(data);
    }),
    terminate: vi.fn(),
    onmessage: null,
    onerror: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  };

  return {
    worker,
    getLastPostedMessage: () =>
      postedMessages.length > 0
        ? postedMessages[postedMessages.length - 1]
        : undefined,
    getAllPostedMessages: () => [...postedMessages],
    simulateMessage: (data: unknown) => {
      if (worker.onmessage) {
        worker.onmessage(new MessageEvent("message", { data }));
      }
    },
    simulateError: (message: string) => {
      if (worker.onerror) {
        const event = new ErrorEvent("error", { message });
        worker.onerror(event);
      }
    },
  };
}
