/**
 * Learner golden path — authentication, explore, sessions, results, progress.
 *
 * Merged from the old learner-flow.spec.ts and roles/learner.spec.ts.
 * Each describe block is independently runnable; serial mode within each group
 * is intentional so that session-creation and finish steps stay ordered.
 */
import { test, expect } from "@playwright/test";
import { API_URL, loginAs, setAuthCookie, makeResponse } from "./helpers";

// ---------------------------------------------------------------------------
// Auth: login / redirect behaviour
// ---------------------------------------------------------------------------
test.describe("authentication", () => {
  test("login page rejects bad credentials", async ({ page }) => {
    await page.goto("/login");
    await page.fill('input[type="email"]', "nobody@example.com");
    await page.fill('input[type="password"]', "badpass");
    await page.click('button[type="submit"]');
    // The API returns a message that the frontend surfaces in an Alert.
    // Use .first() to avoid strict-mode violation with the Next.js route announcer.
    await expect(page.getByRole("alert").first()).toBeVisible({
      timeout: 8000,
    });
  });

  test("unauthenticated request redirects to /login", async ({
    page,
    context,
  }) => {
    await context.clearCookies();
    await page.goto("/explore");
    await page.waitForURL((url) => url.pathname === "/login", {
      timeout: 5000,
    });
  });
});

// ---------------------------------------------------------------------------
// Learner UI — uses the seeded ada@example.com account
// ---------------------------------------------------------------------------
test.describe("learner UI (seeded account)", () => {
  let token: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "ada@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("explore page loads and shows sidebar identity", async ({ page }) => {
    await page.goto("/explore");
    await expect(page.getByText(/Ada Lovelace/i)).toBeVisible({
      timeout: 10000,
    });
    await expect(page.getByRole("heading", { name: "Explore" })).toBeVisible();
  });

  test("all assessments tab is visible", async ({ page }) => {
    await page.goto("/explore");
    await expect(page.getByRole("button", { name: "All" })).toBeVisible({
      timeout: 8000,
    });
  });

  test("practice setup form renders", async ({ page }) => {
    await page.goto("/practice");
    await expect(page.getByText("Practice")).toBeVisible({ timeout: 8000 });
    await expect(page.getByText("Topics", { exact: true })).toBeVisible();
  });

  test("progress page renders", async ({ page }) => {
    await page.goto("/progress");
    await expect(
      page.getByRole("heading", { name: "Progress dashboard" }),
    ).toBeVisible({ timeout: 8000 });
  });
});

// ---------------------------------------------------------------------------
// Learner session flow — uses primary user so we own seeded content
// ---------------------------------------------------------------------------
test.describe("learner session flow (primary user)", () => {
  test.describe.configure({ mode: "serial" });

  let token: string;
  let sessionId: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "ada@example.com");
  });

  test("can start a assessment session", async ({ page, request }) => {
    const r = await request.get(
      `${API_URL}/v1/assessments?status=active&mode=practice`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    const body = await r.json();
    const assessments = body.assessments ?? body;
    expect(
      assessments.length,
      "at least one active assessment",
    ).toBeGreaterThan(0);

    const sr = await request.post(`${API_URL}/v1/sessions`, {
      data: { assessmentId: assessments[0].id },
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(sr.status()).toBe(201);
    sessionId = (await sr.json()).sessionId;

    await setAuthCookie(page, token);
    await page.goto(`/sessions/${sessionId}`);
    await expect(page.locator("text=/question/i").first()).toBeVisible({
      timeout: 8000,
    });
  });

  test("session results page shows score", async ({ page, request }) => {
    const d = await request
      .get(`${API_URL}/v1/sessions/${sessionId}`, {
        headers: { Authorization: `Bearer ${token}` },
      })
      .then((r) => r.json());
    const questions: Array<{ id: string; kind: string }> =
      d.session?.questions ?? d.questions ?? [];

    for (const q of questions) {
      await request.post(`${API_URL}/v1/sessions/${sessionId}/answer`, {
        data: { questionId: q.id, response: makeResponse(q.kind) },
        headers: { Authorization: `Bearer ${token}` },
      });
    }
    await request.post(`${API_URL}/v1/sessions/${sessionId}/finish`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    await setAuthCookie(page, token);
    await page.goto(`/sessions/${sessionId}/results`);
    await expect(page.locator("text=/%|correct|score/i").first()).toBeVisible({
      timeout: 8000,
    });
    await expect(page.getByText(/Attempt 1 of 1/i)).toBeVisible({
      timeout: 8000,
    });
  });

  test("progress page renders stats", async ({ page }) => {
    await setAuthCookie(page, token);
    await page.goto("/progress");
    await expect(
      page.locator("text=/progress|avg score|streak/i").first(),
    ).toBeVisible({ timeout: 8000 });
  });
});
