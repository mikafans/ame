import { test, expect } from "@playwright/test";

// One-off proof that content authored via the agent surface renders and is
// answerable in the learner UI. Run against a dev stack:
//   E2E_API_URL=http://localhost:28080 E2E_BASE_URL=http://localhost:23000 \
//     bunx playwright test e2e/agent-authored.spec.ts --project=chromium
const API_URL = process.env.E2E_API_URL ?? "http://localhost:8080";
const QUIZ_TITLE = process.env.QUIZ_TITLE ?? "Kubernetes Basics (client demo)";

test("agent-authored quiz renders and is answerable in the UI", async ({
  page,
  request,
}) => {
  // Log in as a seeded learner and resolve the agent-authored quiz.
  const login = await request.post(`${API_URL}/v1/auth/login`, {
    data: { email: "learner@example.com", password: "password123" },
  });
  const token = (await login.json()).token as string;
  const auth = { authorization: `Bearer ${token}` };

  const listed = await request.get(`${API_URL}/v1/quizzes`, { headers: auth });
  const quizzes = (await listed.json()).quizzes as Array<{
    id: string;
    title: string;
  }>;
  const quiz = quizzes.filter((q) => q.title === QUIZ_TITLE).at(-1);
  expect(quiz, `quiz '${QUIZ_TITLE}' not found via API`).toBeTruthy();

  // Start a session the same way the library "Start" button does.
  const started = await request.post(`${API_URL}/v1/sessions`, {
    headers: auth,
    data: { quizId: quiz!.id },
  });
  const sessionId = (await started.json()).sessionId as string;

  await page
    .context()
    .addCookies([{ name: "ame_token", value: token, url: API_URL }]);

  // 1. The quiz appears in the learner library.
  await page.goto("/library");
  await page.waitForLoadState("networkidle");
  await expect(page.getByText(QUIZ_TITLE).first()).toBeVisible({
    timeout: 10000,
  });
  await page.screenshot({ path: ".tmp/agent-library.png", fullPage: true });

  // 2. The session renders the agent-authored MC question and accepts an answer.
  await page.goto(`/sessions/${sessionId}`);
  await page.waitForLoadState("networkidle");
  await expect(page.getByText(/smallest deployable unit/i)).toBeVisible({
    timeout: 10000,
  });
  for (const opt of ["Container", "Pod", "Node", "Service"]) {
    await expect(
      page
        .locator('[role="button"]', { hasText: new RegExp(`^.{0,3}${opt}$`) })
        .first(),
    ).toBeVisible();
  }
  await page.locator('[role="button"]', { hasText: /Pod$/ }).first().click();
  await page.screenshot({ path: ".tmp/agent-session.png", fullPage: true });
});
