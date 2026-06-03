import { test, expect } from "@playwright/test";
import { loginAs, setAuthCookie } from "./helpers";

test.describe("admin portal", () => {
  let token: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "admin@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("admin dashboard and navigation loads", async ({ page }) => {
    await page.goto("/admin");
    await expect(
      page.getByRole("heading", { name: "Admin Console" }),
    ).toBeVisible({ timeout: 8000 });

    // Navigate to Manage Users
    await page.getByRole("button", { name: "Users Console" }).click();
    await expect(page).toHaveURL(/\/admin\/users/);
    await expect(
      page.getByRole("heading", { name: "Manage Users" }),
    ).toBeVisible();

    // Navigate back to admin
    await page.goto("/admin");

    // Navigate to Manage Assessments
    await page.getByRole("button", { name: "Moderation Console" }).click();
    await expect(page).toHaveURL(/\/admin\/assessments/);
    await expect(
      page.getByRole("heading", { name: "Manage Assessments" }),
    ).toBeVisible();

    // Navigate back to admin
    await page.goto("/admin");

    // Navigate to Audit Logs
    await page.getByRole("button", { name: "Audit Trail" }).click();
    await expect(page).toHaveURL(/\/admin\/audit/);
    await expect(
      page.getByRole("heading", { name: "Audit Logs" }),
    ).toBeVisible();

    // Navigate back to admin
    await page.goto("/admin");

    // Navigate to System Health
    await page.getByRole("button", { name: "System Status" }).click();
    await expect(page).toHaveURL(/\/admin\/health/);
    await expect(
      page.getByRole("heading", { name: "System Health" }),
    ).toBeVisible();
  });
});
