import { test, expect } from "@playwright/test";

test.describe("Theme", () => {
  test("defaults to dark theme", async ({ page }) => {
    await page.goto("/");
    const html = page.locator("html");
    await expect(html).toHaveClass(/dark/);
  });

  test("dark theme persists across page reload", async ({ page }) => {
    await page.goto("/");
    const html = page.locator("html");
    await expect(html).toHaveClass(/dark/);

    // Reload and verify dark theme persists
    await page.reload();
    await expect(html).toHaveClass(/dark/);
  });

  test("dark theme class applied with no localStorage", async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.removeItem("darkreach-theme");
    });
    await page.goto("/");
    const html = page.locator("html");
    await expect(html).toHaveClass(/dark/);
  });
});
