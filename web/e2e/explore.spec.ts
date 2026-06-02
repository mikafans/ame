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
});
