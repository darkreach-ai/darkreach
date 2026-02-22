import { test, expect } from "@playwright/test";
import { setupAuthenticatedPage } from "./helpers";

test.describe("Dashboard page", () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedPage(page);
  });

  test("renders database status with prime count", async ({ page }) => {
    await page.goto("/");
    // Should see total primes from mock stats in database status card
    await expect(page.getByText("42 primes stored")).toBeVisible({ timeout: 10000 });
  });

  test("renders primes table with mock data", async ({ page }) => {
    await page.goto("/");
    // Should see prime expressions from mock data in the primes table
    await expect(page.getByText("27!+1").first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("37!-1").first()).toBeVisible();
  });

  test("renders form badges from stats", async ({ page }) => {
    await page.goto("/");
    // Should see form names from mock data
    await expect(page.getByText("Factorial").first()).toBeVisible({ timeout: 10000 });
  });

  test("shows idle status when no WebSocket data", async ({ page }) => {
    await page.goto("/");
    // Without WS data, should show idle state
    await expect(page.getByText(/idle/i).first()).toBeVisible({ timeout: 10000 });
  });
});
