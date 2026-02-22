import { type Page } from "@playwright/test";

const SUPABASE_URL = "https://nljvgyorzoxajodkkqdu.supabase.co";
const STORAGE_KEY = "sb-nljvgyorzoxajodkkqdu-auth-token";

/** Fake auth session injected into localStorage so the AuthProvider sees a user. */
const MOCK_SESSION = {
  access_token: "fake-access-token",
  token_type: "bearer",
  expires_in: 86400,
  expires_at: Math.floor(Date.now() / 1000) + 86400,
  refresh_token: "fake-refresh-token",
  user: {
    id: "00000000-0000-0000-0000-000000000001",
    aud: "authenticated",
    role: "authenticated",
    email: "test@darkreach.ai",
    email_confirmed_at: "2025-01-01T00:00:00Z",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    app_metadata: { provider: "email" },
    user_metadata: {},
  },
};

/** Inject a fake auth session into localStorage before the page loads. */
export async function mockAuth(page: Page) {
  await page.addInitScript(
    ({ key, session }) => {
      localStorage.setItem(key, JSON.stringify(session));
    },
    { key: STORAGE_KEY, session: MOCK_SESSION },
  );
}

/** Mock Supabase auth API endpoints so getSession / onAuthStateChange work. */
export async function mockAuthApi(page: Page) {
  // getSession reads from storage first, but the SDK also calls the API for refresh
  await page.route(`${SUPABASE_URL}/auth/v1/**`, (route) => {
    const url = route.request().url();
    if (url.includes("/token")) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(MOCK_SESSION),
      });
    }
    if (url.includes("/user")) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(MOCK_SESSION.user),
      });
    }
    // Default: return empty OK
    return route.fulfill({ status: 200, body: "{}" });
  });
}

export const MOCK_PRIMES = [
  { id: 1, form: "Factorial", expression: "27!+1", digits: 29, found_at: "2025-06-01T12:00:00Z", proof_method: "Pocklington", verified: true, verified_at: null, verification_method: null, verification_tier: null, tags: [] },
  { id: 2, form: "Factorial", expression: "37!-1", digits: 44, found_at: "2025-06-02T14:00:00Z", proof_method: "Morrison", verified: true, verified_at: null, verification_method: null, verification_tier: null, tags: [] },
  { id: 3, form: "KBN", expression: "3*2^127-1", digits: 39, found_at: "2025-06-03T10:00:00Z", proof_method: "LLR", verified: false, verified_at: null, verification_method: null, verification_tier: null, tags: [] },
];

export const MOCK_STATS = {
  total: 42,
  largest_expression: "3*2^127-1",
  largest_digits: 39,
  by_form: [
    { form: "Factorial", count: 20 },
    { form: "KBN", count: 15 },
    { form: "Palindromic", count: 7 },
  ],
};

export const MOCK_TIMELINE = [
  { bucket: "2025-06-01", form: "Factorial", count: 5 },
  { bucket: "2025-06-02", form: "KBN", count: 3 },
];

export const MOCK_DISTRIBUTION = [
  { bucket: 10, form: "Factorial", count: 8 },
  { bucket: 20, form: "KBN", count: 6 },
  { bucket: 30, form: "Palindromic", count: 4 },
];

/** Mock all REST API endpoints with test data. */
export async function mockSupabaseData(page: Page) {
  // Auth profile — return admin role for tests
  await page.route("**/api/auth/me**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ role: "admin", operator_id: "00000000-0000-0000-0000-000000000001" }),
    }),
  );

  await page.route("**/api/stats", (route) => {
    const url = route.request().url();
    // Only match /api/stats exactly, not /api/stats/timeline etc.
    if (url.match(/\/api\/stats\/?(\?|$)/)) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(MOCK_STATS),
      });
    }
    return route.fallback();
  });

  await page.route("**/api/stats/timeline**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(MOCK_TIMELINE),
    }),
  );

  await page.route("**/api/stats/distribution**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(MOCK_DISTRIBUTION),
    }),
  );

  await page.route("**/api/stats/leaderboard**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([]),
    }),
  );

  await page.route("**/api/stats/tags**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([]),
    }),
  );

  await page.route("**/api/prime-verification/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ ok: true, stats: { pending: 0, claimed: 0, verified: 0, failed: 0, total_primes: 0, quorum_met: 0 }, results: [] }),
    }),
  );

  // Primes list and detail — REST API returns { primes: [], total: N }
  await page.route("**/api/primes**", (route) => {
    const url = route.request().url();
    // Verifications sub-endpoint: /api/primes/:id/verifications
    if (url.match(/\/api\/primes\/\d+\/verifications/)) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ ok: true, results: [] }),
      });
    }
    // Detail query: /api/primes/:id (numeric ID at end, no further path)
    if (url.match(/\/api\/primes\/\d+\/?(\?|$)/)) {
      const detail = {
        ...MOCK_PRIMES[0],
        search_params: JSON.stringify({ start: 1, end: 100 }),
        verification_tier: 1,
        verification_method: "GMP MR-25",
        verified_at: "2025-06-01T12:30:00Z",
        tags: [],
      };
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(detail),
      });
    }
    // List query: /api/primes?limit=...&offset=...
    return route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        primes: MOCK_PRIMES,
        total: MOCK_PRIMES.length,
        limit: 50,
        offset: 0,
      }),
    });
  });
}

/** Set up all mocks needed for an authenticated page with data. */
export async function setupAuthenticatedPage(page: Page) {
  await mockAuth(page);
  await mockAuthApi(page);
  await mockSupabaseData(page);
}
