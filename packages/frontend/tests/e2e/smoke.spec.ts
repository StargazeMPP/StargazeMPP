import { test, expect } from "@playwright/test";

test.describe("StargazeMPP smoke", () => {
  test("landing renders brand", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveTitle(/StargazeMPP/);
  });

  test("dashboard mounts wallet shell", async ({ page }) => {
    await page.goto("/dashboard");
    // The Lovable dashboard ships an "Overview" h1 once the route hydrates.
    await expect(page.getByRole("heading", { name: /overview/i })).toBeVisible();
  });

  test("docs renders single-page guide", async ({ page }) => {
    await page.goto("/docs");
    await expect(page.getByText(/Documentation/i).first()).toBeVisible();
  });

  test("privacy renders policy", async ({ page }) => {
    await page.goto("/privacy");
    await expect(page.getByText(/Privacy/i).first()).toBeVisible();
  });

  test("placeholder routes load with brand", async ({ page }) => {
    for (const path of [
      "/explore",
      "/playground",
      "/providers/mpp32",
      "/dashboard/stake",
      "/dashboard/sessions",
      "/dashboard/reputation",
      "/dashboard/provider",
      "/docs/architecture",
    ]) {
      const res = await page.goto(path);
      expect(res?.status()).toBeLessThan(400);
      await expect(page.locator("body")).toBeVisible();
    }
  });

  test("404 shows custom page", async ({ page }) => {
    await page.goto("/definitely-not-a-route");
    await expect(page.getByText("404")).toBeVisible();
  });
});
