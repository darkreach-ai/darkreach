import { test, expect } from "@playwright/test";
import { setupAuthenticatedPage } from "./helpers";

test.describe("Network page", () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthenticatedPage(page);
  });

  test("renders network stats with zero state", async ({ page }) => {
    await page.goto("/network");
    await expect(page.getByText("Network").first()).toBeVisible({ timeout: 10000 });
    // Without WS data, stat cards should show zeroes
    await expect(page.getByText("Machines").first()).toBeVisible();
    await expect(page.getByText("Nodes").first()).toBeVisible();
    await expect(page.getByText("Cores").first()).toBeVisible();
  });

  test("shows empty state when no machines online", async ({ page }) => {
    await page.goto("/network");
    // With no workers connected, should show empty state
    await expect(page.getByText(/no compute machines/i)).toBeVisible({ timeout: 10000 });
  });

  test("shows filter controls", async ({ page }) => {
    await page.goto("/network");
    // Filter input should be visible
    await expect(page.getByPlaceholder(/filter/i)).toBeVisible({ timeout: 10000 });
  });
});
