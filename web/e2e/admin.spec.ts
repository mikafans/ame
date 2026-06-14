import { test, expect } from "@playwright/test";
import { loginAs, setAuthCookie, API_URL } from "./helpers";

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

  test("assessment preview drawer reveals questions and answers", async ({
    page,
    request,
  }) => {
    // Admins own no assessments, so resolve a known MCQ-bearing one via the
    // admin endpoints (cross-owner). This also exercises the moderation path.
    const list = await request.get(
      `${API_URL}/v1/admin/assessments?status=active&limit=50`,
      { headers: { Authorization: `Bearer ${token}` } },
    );
    expect(list.ok(), "list admin assessments").toBeTruthy();
    const { assessments } = await list.json();
    let title: string | undefined;
    for (const a of assessments as Array<{ id: string; title: string }>) {
      const detail = await request.get(
        `${API_URL}/v1/admin/assessments/${a.id}`,
        { headers: { Authorization: `Bearer ${token}` } },
      );
      if (!detail.ok()) continue;
      const d = await detail.json();
      const hasMc = (d.questions ?? []).some(
        (q: { kind: string; payload?: { correct_index?: number } }) =>
          q.kind === "mc" && typeof q.payload?.correct_index === "number",
      );
      if (hasMc) {
        title = a.title;
        break;
      }
    }
    expect(title, "an active assessment with an MC question").toBeTruthy();
    const targetTitle = title as string;

    await page.goto("/admin/assessments");
    await expect(
      page.getByRole("heading", { name: "Manage Assessments" }),
    ).toBeVisible({ timeout: 8000 });

    // Filter to the target assessment, then open its preview drawer.
    await page.getByPlaceholder(/Search title/i).fill(targetTitle);
    await page.getByRole("button", { name: "Search" }).click();
    const row = page
      .locator("table tbody tr", { hasText: targetTitle })
      .first();
    await expect(row).toBeVisible({ timeout: 8000 });
    await row.getByRole("button", { name: "Preview content" }).click();

    // Drawer shows the question list; the answer reveal (CORRECT chip) is admin-only.
    const drawer = page.locator(".MuiDrawer-paper").first();
    await expect(drawer).toBeVisible();
    await expect(drawer.getByText(/question/i).first()).toBeVisible();
    await expect(
      drawer.locator(".MuiChip-root").filter({ hasText: "CORRECT" }).first(),
    ).toBeVisible({
      timeout: 8000,
    });
  });
});
