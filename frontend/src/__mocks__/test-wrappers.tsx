/**
 * @file Test wrapper components that provide mock contexts for testing
 * @module __mocks__/test-wrappers
 *
 * Provides pre-configured mock context data and wrapper components for testing
 * components that depend on WebSocket, auth, or navigation contexts. These
 * wrappers eliminate boilerplate in component tests by providing sensible
 * default values for all context fields.
 *
 * Exports:
 * - `defaultWsData`: Complete WsData object with empty/connected defaults
 * - `defaultAuthData`: Mock authenticated user for auth-dependent components
 * - `mockNextNavigation()`: Factory for mocking next/navigation hooks
 * - `createWsWrapper()`: Factory for creating WebSocket context wrappers
 *
 * @see {@link ../contexts/websocket-context} WebSocket context
 * @see {@link ../contexts/auth-context} Auth context
 */
import React from "react";
import type { WsData, WorkerStatus } from "@/hooks/use-websocket";
import type { MonthlyEarning, CreditRow } from "@/hooks/use-earnings";
import type { ActiveFormStat, CreditRate } from "@/hooks/use-marketplace";

/**
 * Default mock WsData object with all fields at empty/connected defaults.
 * Simulates a connected coordinator with no active searches, workers, or agents.
 * Use `createWsWrapper({ ...overrides })` to customize specific fields.
 */
export const defaultWsData: WsData = {
  status: { active: false, checkpoint: null },
  fleet: {
    workers: [],
    total_workers: 0,
    total_cores: 0,
    total_tested: 0,
    total_found: 0,
  },
  coordinator: null,
  searches: [],
  searchJobs: [],
  deployments: [],
  notifications: [],
  agentTasks: [],
  agentBudgets: [],
  runningAgents: [],
  projects: [],
  records: [],
  lastPrimeFound: null,
  aiEngine: null,
  strategy: null,
  connected: true,
  sendMessage: () => {},
};

/**
 * Default mock auth context data for testing components that require
 * an authenticated user. Provides a test user with no active session,
 * and no-op signIn/signOut functions.
 */
export const defaultAuthData = {
  user: { id: "test-user", email: "test@example.com" } as unknown as import("@supabase/supabase-js").User,
  session: null as unknown as import("@supabase/supabase-js").Session | null,
  loading: false,
  signIn: async () => null as string | null,
  signOut: async () => {},
};

/**
 * Creates a mock for the next/navigation module used by Next.js pages.
 * Returns mock implementations of usePathname, useRouter, useSearchParams,
 * and useParams with configurable pathname.
 *
 * Usage: `vi.mock("next/navigation", () => mockNextNavigation("/browse"))`
 *
 * The router methods (push, replace, back, etc.) are no-ops by default.
 * Override individual methods if you need to assert navigation calls.
 */
export function mockNextNavigation(pathname: string = "/") {
  return {
    usePathname: () => pathname,
    useRouter: () => ({
      push: () => {},
      replace: () => {},
      back: () => {},
      forward: () => {},
      refresh: () => {},
      prefetch: () => {},
    }),
    useSearchParams: () => new URLSearchParams(),
    useParams: () => ({}),
  };
}

/**
 * Creates a React wrapper component that provides WebSocket context data
 * with customizable overrides. Merges the provided overrides with
 * defaultWsData for a complete WsData object.
 *
 * Note: This wrapper uses a simplified approach -- it renders children
 * directly without an actual context provider. For tests that need the
 * real WebSocketProvider, use it directly with mocked transport hooks.
 *
 * Usage:
 * ```typescript
 * const wrapper = createWsWrapper({ connected: true, fleet: { ... } });
 * const { result } = renderHook(() => useMyHook(), { wrapper });
 * ```
 */
export function createWsWrapper(overrides: Partial<WsData> = {}) {
  const wsData = { ...defaultWsData, ...overrides };

  // We provide the context via a mock of the useWs hook
  return function WsWrapper({ children }: { children: React.ReactNode }) {
    return <>{children}</>;
  };
}

/**
 * Factory for a WorkerStatus (aliased as NodeStatus) with sensible defaults.
 * Overrides are merged on top of defaults so tests only need to specify
 * the fields under test.
 */
export function makeNodeStatus(overrides: Partial<WorkerStatus> = {}): WorkerStatus {
  return {
    worker_id: "node-abc123",
    hostname: "compute-alpha",
    cores: 8,
    search_type: "kbn",
    search_params: JSON.stringify({ k: 3, base: 2, min_n: 1, max_n: 100000 }),
    current: "",
    tested: 10000,
    found: 2,
    uptime_secs: 3600,
    last_heartbeat_secs_ago: 5,
    ...overrides,
  };
}

/**
 * Factory for a MonthlyEarning row with sensible defaults.
 * Used by EarningsChart tests.
 */
export function makeMonthlyEarning(overrides: Partial<MonthlyEarning> = {}): MonthlyEarning {
  return {
    month: "2026-01-01",
    total_credits: 1000,
    block_count: 50,
    ...overrides,
  };
}

/**
 * Factory for a CreditRow (credit transaction) with sensible defaults.
 * Used by EarningsHistoryTable tests.
 */
export function makeCreditRow(overrides: Partial<CreditRow> = {}): CreditRow {
  return {
    id: 1,
    block_id: 42,
    credit: 100,
    reason: "block_completed",
    granted_at: "2026-02-20T12:00:00Z",
    ...overrides,
  };
}

/**
 * Factory for an ActiveFormStat with sensible defaults.
 * Used by FormShowcaseCard tests.
 */
export function makeActiveFormStat(overrides: Partial<ActiveFormStat> = {}): ActiveFormStat {
  return {
    form: "kbn",
    job_count: 3,
    total_blocks: 100,
    completed_blocks: 45,
    ...overrides,
  };
}

/**
 * Factory for a CreditRate with sensible defaults.
 * Used by RateTable tests.
 */
export function makeCreditRate(overrides: Partial<CreditRate> = {}): CreditRate {
  return {
    resource_type: "cpu_core_hours",
    credits_per_unit: 10,
    unit_label: "core-hour",
    updated_at: "2026-02-20T12:00:00Z",
    ...overrides,
  };
}
