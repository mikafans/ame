import { test, expect } from "@playwright/test";

const BASE = process.env.E2E_BASE_URL ?? "http://localhost:3000";
const API = process.env.E2E_API_URL ?? "http://localhost:8080";

function makeResponse(kind: string): Record<string, unknown> {
  if (kind === "mc" || kind === "mcq") return { selected_position: 0 };
  if (kind === "tf") return { answer: true };
  if (kind === "essay")
    return { answer: "e2e answer for essay question", word_count: 5 };
  return { answer: "e2e answer" };
}

test.describe("Learner role", () => {
  test.describe.configure({ mode: "serial" });

  let token: string;
  let sessionId: string;

  test.beforeAll(async ({ request }) => {
    const ts = Date.now();
    const r = await request.post(`${API}/v1/auth/register`, {
      data: {
        email: `e2e-learner-${ts}@example.com`,
        password: "e2e-password-123",
        name: "E2E Learner",
        role: "learner",
      },
    });
    expect(r.status()).toBe(201);
    token = (await r.json()).token;
  });

  test("library page loads quizzes", async ({ page }) => {
    await page
      .context()
      .addCookies([{ name: "ame_token", value: token, url: BASE }]);
    await page.goto(`${BASE}/`);
    // Wait for the auth loading to finish and user to be visible in sidebar
    await expect(page.getByText(/E2E Learner/i)).toBeVisible({
      timeout: 10000,
    });

    await expect(page.getByRole("heading").first()).toBeVisible({
      timeout: 8000,
    });
  });

  test("can start a quiz session and answer questions", async ({
    page,
    request,
  }) => {
    const r = await request.get(`${API}/v1/quizzes?status=active`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    const quizzes = (await r.json()).quizzes ?? [];
    expect(quizzes.length).toBeGreaterThan(0);

    const sr = await request.post(`${API}/v1/sessions`, {
      data: { quizId: quizzes[0].id },
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(sr.status()).toBe(201);
    sessionId = (await sr.json()).sessionId;

    await page
      .context()
      .addCookies([{ name: "ame_token", value: token, url: BASE }]);
    await page.goto(`${BASE}/sessions/${sessionId}`);
    await expect(page.locator("text=/question/i").first()).toBeVisible({
      timeout: 8000,
    });
  });

  test("session results page shows score", async ({ page, request }) => {
    const d = await request
      .get(`${API}/v1/sessions/${sessionId}`, {
        headers: { Authorization: `Bearer ${token}` },
      })
      .then((r) => r.json());
    const questions: Array<{ id: string; kind: string }> =
      d.session?.questions ?? d.questions ?? [];

    for (const q of questions) {
      await request.post(`${API}/v1/sessions/${sessionId}/answer`, {
        data: { questionId: q.id, response: makeResponse(q.kind) },
        headers: { Authorization: `Bearer ${token}` },
      });
    }
    await request.post(`${API}/v1/sessions/${sessionId}/finish`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    await page
      .context()
      .addCookies([{ name: "ame_token", value: token, url: BASE }]);
    await page.goto(`${BASE}/sessions/${sessionId}/results`);
    await expect(page.locator("text=/%|correct|score/i").first()).toBeVisible({
      timeout: 8000,
    });
  });

  test("progress page renders stats", async ({ page }) => {
    await page
      .context()
      .addCookies([{ name: "ame_token", value: token, url: BASE }]);
    await page.goto(`${BASE}/progress`);
    await expect(
      page.locator("text=/progress|avg score|streak/i").first(),
    ).toBeVisible({ timeout: 8000 });
  });
});
