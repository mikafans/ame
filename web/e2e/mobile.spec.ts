/**
 * Mobile viewport e2e tests.
 *
 * These tests verify that the responsive shell works on a mobile device
 * (Pixel 5, ~410px width). They run in a separate project from desktop tests
 * to avoid false failures caused by different UI layouts.
 */
import { test, expect } from "@playwright/test";
import {
  API_URL,
  loginAs,
  setAuthCookie,
  firstMcqAssessmentId,
} from "./helpers";

// ---------------------------------------------------------------------------
// Mobile UI — navigation, sidebar behavior
// ---------------------------------------------------------------------------
test.describe("mobile UI (seeded account)", () => {
  let token: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "ada@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("sidebar is collapsed behind a hamburger on mobile and toggles", async ({
    page,
  }) => {
    await page.goto("/explore");

    // The Sidebar renders its content in TWO drawers: a permanent one
    // (display:none at this width, class .MuiDrawer-docked) and a temporary
    // one (keepMounted, class .MuiDrawer-modal). Both contain the identity and
    // nav labels, so unscoped getByText would resolve to 2 elements and trip
    // Playwright strict mode. Scope all drawer-content queries to the mobile
    // (temporary) drawer.
    const mobileDrawer = page.locator(".MuiDrawer-modal");

    // Assert the hamburger is visible (it lives in the AppBar, so it's unique)
    await expect(
      page.getByRole("button", { name: /Open navigation/i }),
    ).toBeVisible({ timeout: 10000 });

    // Assert nav identity is NOT visible initially (drawer closed)
    await expect(mobileDrawer.getByText(/Ada Lovelace/i)).toBeHidden();

    // Tap the hamburger to open the drawer
    await page.getByRole("button", { name: /Open navigation/i }).click();

    // Assert identity becomes visible
    await expect(mobileDrawer.getByText(/Ada Lovelace/i)).toBeVisible({
      timeout: 8000,
    });

    // Tap a nav item to navigate and close the drawer
    await mobileDrawer.getByRole("button", { name: /Flashcards/i }).click();
    await page.waitForURL((u) => u.pathname === "/flashcards");

    // Assert the drawer closed and identity is hidden again
    await expect(mobileDrawer.getByText(/Ada Lovelace/i)).toBeHidden();
  });
});

// ---------------------------------------------------------------------------
// Mobile session flow — read-only smoke test
// ---------------------------------------------------------------------------
test.describe("mobile session flow (primary user)", () => {
  test.describe.configure({ mode: "serial" });

  let token: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "ada@example.com");
  });

  test("can open and view a session on a mobile viewport", async ({
    page,
    request,
  }) => {
    // Start a session via API
    const assessmentId = await firstMcqAssessmentId(request, token);
    const sr = await request.post(`${API_URL}/v1/sessions`, {
      data: { assessmentId },
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(sr.status()).toBe(201);
    const sessionId = (await sr.json()).sessionId;

    // Navigate to the session
    await setAuthCookie(page, token);
    await page.goto(`/sessions/${sessionId}`);

    // Assert the question renders
    await expect(page.locator("text=/question/i").first()).toBeVisible({
      timeout: 8000,
    });

    // Assert the header Submit button is reachable on the narrow viewport
    await expect(
      page.getByRole("button", { name: /submit/i }).first(),
    ).toBeVisible();
  });
});
