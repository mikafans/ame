import { test, expect } from "@playwright/test";

const API_TOKEN = process.env.E2E_API_TOKEN ?? "";

test.describe("learner golden path", () => {
  test.skip(!API_TOKEN, "E2E_API_TOKEN not set — skipping learner flow tests");

  test.beforeEach(async ({ page }) => {
    // Set auth cookie directly instead of going through login UI
    await page.goto("/login");
    await page.evaluate((token) => {
      document.cookie = `ame_token=${token}; path=/; max-age=86400`;
    }, API_TOKEN);
  });

  test("login page renders and rejects bad credentials", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: "Sign in" }).click();
    await page.fill('input[type="email"]', "nobody@example.com");
    await page.fill('input[type="password"]', "badpass");
    await page.click('button[type="submit"]');
    await expect(page.getByText("missing or invalid token")).toBeVisible({
      timeout: 8000,
    });
  });

  test("library screen loads", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Library" })).toBeVisible();
  });

  test("practice setup form renders", async ({ page }) => {
    await page.goto("/practice");
    await expect(page.getByText("Practice")).toBeVisible();
    await expect(page.getByText("Topics")).toBeVisible();
  });

  test("progress page renders", async ({ page }) => {
    await page.goto("/progress");
    await expect(
      page.getByRole("heading", { name: "Progress dashboard" }),
    ).toBeVisible();
  });

  test("unauthenticated user redirects to login", async ({ page, context }) => {
    await context.clearCookies();
    await page.goto("/");
    await page.waitForURL("**/login", { timeout: 5000 });
  });
});
