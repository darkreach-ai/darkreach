import { test, expect } from "@playwright/test";
import { setupAuthenticatedPage } from "./helpers";

test.describe("Browse page", () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedPage(page);
  });

  test("renders filter controls and primes table", async ({ page }) => {
    await page.goto("/browse");
    // Should see search input
    await expect(page.getByPlaceholder(/expression/i)).toBeVisible({ timeout: 10000 });
    // Should see prime data in table
    await expect(page.getByText("27!+1")).toBeVisible();
  });

  test("renders Browse heading with count", async ({ page }) => {
    await page.goto("/browse");
    await expect(page.getByRole("heading", { name: /browse/i })).toBeVisible({ timeout: 10000 });
    // Should show prime count from mock data
    await expect(page.getByText("3 primes").first()).toBeVisible();
  });

  test("clicking a row opens detail dialog", async ({ page }) => {
    await page.goto("/browse");
    // Wait for table to render
    await expect(page.getByText("27!+1")).toBeVisible({ timeout: 10000 });
    // Click the first prime row
    await page.getByText("27!+1").click();
    // Detail dialog should open
    await expect(page.locator('[role="dialog"]')).toBeVisible({ timeout: 5000 });
  });
});
