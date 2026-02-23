/**
 * @file Tests for the NotificationToaster component
 * @module __tests__/components/prime-notifier
 *
 * Validates the invisible component that watches the WebSocket notification
 * stream and fires toast notifications (via Sonner) and browser notifications
 * (via the Notification API) for new events. Key behaviors tested:
 *
 * - First render marks all existing notifications as "seen" without toasting
 * - Subsequent new notifications fire the appropriate toast variant:
 *   "prime" -> toast.success, "error" -> toast.error, "search_start"/
 *   "search_done"/"milestone" -> toast.info, unknown -> default toast
 * - Browser notification is fired for "prime" kind via show()
 * - Deduplication: the same notification ID is only toasted once
 * - Ring buffer trims the seenIds Set when it exceeds 500 entries
 *
 * Note: The component's initialization logic skips toasting on the first
 * render that has notifications (marks all as seen). When the initial
 * notification list is empty, `initialized` stays false until the first
 * non-empty render. Tests that want to verify toasting of new notifications
 * must therefore seed the component with at least one initial notification
 * to trigger the initialization branch before adding new ones.
 *
 * @see {@link ../../components/prime-notifier} NotificationToaster source
 * @see {@link ../../contexts/websocket-context} useWs hook (notifications)
 * @see {@link ../../hooks/use-notifications} useBrowserNotifications hook
 */
import { vi, describe, it, expect, beforeEach } from "vitest";
import { render } from "@testing-library/react";

// Use vi.hoisted() so mock functions are available inside vi.mock() factories,
// which are hoisted to the top of the file by Vitest's transform.
const {
  mockToastSuccess,
  mockToastError,
  mockToastInfo,
  mockToastDefault,
  mockShow,
} = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
  mockToastInfo: vi.fn(),
  mockToastDefault: vi.fn(),
  mockShow: vi.fn(),
}));

vi.mock("sonner", () => {
  const toastFn = Object.assign(mockToastDefault, {
    success: mockToastSuccess,
    error: mockToastError,
    info: mockToastInfo,
  });
  return { toast: toastFn };
});

let mockNotifications: Array<{
  id: number;
  kind: string;
  title: string;
  details: string[];
  count: number;
  timestamp_ms: number;
}> = [];

vi.mock("@/contexts/websocket-context", () => ({
  useWs: () => ({ notifications: mockNotifications }),
}));

vi.mock("@/hooks/use-notifications", () => ({
  useBrowserNotifications: () => ({ show: mockShow }),
}));

import { NotificationToaster } from "@/components/prime-notifier";

/** Factory helper for creating a notification entry. */
function makeNotif(
  id: number,
  kind: string,
  title: string = `Notification ${id}`,
  details: string[] = [],
) {
  return { id, kind, title, details, count: 1, timestamp_ms: Date.now() };
}

/**
 * Helper to initialize the component with seed notifications (marked as
 * seen without toasting) and clear mock call counts before the real test
 * assertions. Returns the rerender function for subsequent updates.
 */
function initializeWith(seeds: typeof mockNotifications) {
  mockNotifications = seeds;
  const result = render(<NotificationToaster />);
  // The initialization branch fires synchronously in the useEffect during
  // the first render that has notifications, marking all as seen.
  vi.clearAllMocks();
  return result;
}

describe("NotificationToaster", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockNotifications = [];
  });

  it("renders nothing (returns null)", () => {
    mockNotifications = [];
    const { container } = render(<NotificationToaster />);
    expect(container.innerHTML).toBe("");
  });

  it("marks all initial notifications as seen without toasting", () => {
    mockNotifications = [
      makeNotif(1, "prime", "Found prime!"),
      makeNotif(2, "error", "Search failed"),
    ];
    render(<NotificationToaster />);
    expect(mockToastSuccess).not.toHaveBeenCalled();
    expect(mockToastError).not.toHaveBeenCalled();
    expect(mockToastInfo).not.toHaveBeenCalled();
    expect(mockToastDefault).not.toHaveBeenCalled();
  });

  it("fires toast.success for new 'prime' notification", () => {
    const seed = makeNotif(0, "milestone", "Seed notification");
    const { rerender } = initializeWith([seed]);

    mockNotifications = [seed, makeNotif(1, "prime", "New prime: 5000!+1")];
    rerender(<NotificationToaster />);

    expect(mockToastSuccess).toHaveBeenCalledWith(
      "New prime: 5000!+1",
      expect.objectContaining({ duration: 8000 }),
    );
  });

  it("fires toast.error for 'error' notification", () => {
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    mockNotifications = [seed, makeNotif(1, "error", "Search crashed")];
    rerender(<NotificationToaster />);

    expect(mockToastError).toHaveBeenCalledWith(
      "Search crashed",
      expect.objectContaining({ duration: 10000 }),
    );
  });

  it("fires toast.info for 'search_start' notification", () => {
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    mockNotifications = [seed, makeNotif(1, "search_start", "Factorial search started")];
    rerender(<NotificationToaster />);

    expect(mockToastInfo).toHaveBeenCalledWith(
      "Factorial search started",
      expect.objectContaining({ duration: 6000 }),
    );
  });

  it("fires toast.info for 'search_done' notification", () => {
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    mockNotifications = [seed, makeNotif(1, "search_done", "Search complete")];
    rerender(<NotificationToaster />);

    expect(mockToastInfo).toHaveBeenCalledWith(
      "Search complete",
      expect.objectContaining({ duration: 6000 }),
    );
  });

  it("fires default toast for unknown kind", () => {
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    mockNotifications = [seed, makeNotif(1, "unknown_kind", "Something happened")];
    rerender(<NotificationToaster />);

    expect(mockToastDefault).toHaveBeenCalledWith(
      "Something happened",
      expect.objectContaining({}),
    );
  });

  it("calls browser notification show() for prime kind", () => {
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    mockNotifications = [seed, makeNotif(1, "prime", "Found prime!")];
    rerender(<NotificationToaster />);

    expect(mockShow).toHaveBeenCalledWith(
      "Found prime!",
      expect.objectContaining({ tag: "prime-1" }),
    );
  });

  it("deduplicates -- same id only toasted once", () => {
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    const notif = makeNotif(1, "prime", "Found prime!");
    mockNotifications = [seed, notif];
    rerender(<NotificationToaster />);

    expect(mockToastSuccess).toHaveBeenCalledTimes(1);

    // Re-render with the same notifications -- should not toast again
    mockNotifications = [seed, notif];
    rerender(<NotificationToaster />);
    expect(mockToastSuccess).toHaveBeenCalledTimes(1);
  });

  it("ring buffer trims seenIds when over 500 without crashing", () => {
    // Start with one seed to initialize
    const seed = makeNotif(0, "milestone", "Seed");
    const { rerender } = initializeWith([seed]);

    // Add 510 new notifications in one batch to exceed the 500-entry threshold.
    // The component trims the seenIds Set down to 250 entries when it grows
    // past 500, which happens inside the notification processing loop.
    const batch = Array.from({ length: 510 }, (_, i) =>
      makeNotif(i + 1, "prime", `Prime #${i + 1}`),
    );
    mockNotifications = [seed, ...batch];
    rerender(<NotificationToaster />);

    // All 510 new notifications should have fired toasts on first appearance.
    expect(mockToastSuccess).toHaveBeenCalledTimes(510);

    // After the trim, a brand-new notification ID should still be processed.
    const freshNotif = makeNotif(9999, "prime", "Fresh prime after trim");
    mockNotifications = [seed, ...batch, freshNotif];
    rerender(<NotificationToaster />);

    // The fresh notification must have been toasted.
    expect(mockToastSuccess).toHaveBeenCalledWith(
      "Fresh prime after trim",
      expect.objectContaining({ duration: 8000 }),
    );
  });
});
