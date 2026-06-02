import { expect, test } from "@playwright/test";
import { loginAs, setAuthCookie } from "./helpers";

test.describe("explore page", () => {
  let token: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "ada@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("explore page loads and shows data with empty filters", async ({
    page,
  }) => {
    await page.goto("/explore");

    // Wait for loading to finish
    const loading = page.getByText("Loading...");
    await expect(loading).not.toBeVisible();

    // Verify the table loads and rows are visible
    const firstRow = page.locator("table tbody tr").first();
    await expect(firstRow).toBeVisible({ timeout: 10000 });
  });

  test("unauthenticated access redirects to /login", async ({ page }) => {
    // Navigate with no cookie set
    await page.context().clearCookies();
    await page.goto("/explore");

    // Should redirect to login page with returnTo parameter
    await expect(page).toHaveURL(/\/login\?returnTo=%2Fexplore/);
  });

  test("access with invalid token redirects to /login without throwing unhandled exceptions", async ({
    page,
  }) => {
    // Set an invalid/malformed token
    await setAuthCookie(page, "invalid-token-12345");

    // Track console errors
    const consoleErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto("/explore");

    // Since token is invalid, /v1/me returns 401, which triggers a redirect to /login
    await expect(page).toHaveURL(/\/login/);

    // Ensure no unhandled rejections or crashes happened on the page
    const hasFetchError = consoleErrors.some((err) =>
      err.includes("Failed to fetch"),
    );
    expect(hasFetchError).toBe(false);
  });
});
