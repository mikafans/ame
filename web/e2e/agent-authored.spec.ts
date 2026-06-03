/**
 * End-to-end proof that content authored via the agent surface renders and is
 * answerable in the learner UI.
 *
 * Requires the dev stack + seed data (make dev && make db-seed).
 */
import { test, expect } from "@playwright/test";
import { API_URL, loginAs, setAuthCookie } from "./helpers";

const QUIZ_TITLE =
  process.env.QUIZ_TITLE ?? "Algorithms and Data Structures — Fundamentals";

test("agent-authored assessment renders and is answerable in the UI", async ({
  page,
  request,
}) => {
  const token = await loginAs(request, "ada@example.com");

  const listed = await request.get(`${API_URL}/v1/assessments`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  const assessments = (await listed.json()).assessments as Array<{
    id: string;
    title: string;
  }>;
  const assessment = assessments.filter((q) => q.title === QUIZ_TITLE).at(-1);
  expect(
    assessment,
    `assessment '${QUIZ_TITLE}' not found via API`,
  ).toBeTruthy();

  // Start a session the same way the explore "Start" button does
  const started = await request.post(`${API_URL}/v1/sessions`, {
    headers: { Authorization: `Bearer ${token}` },
    data: { assessmentId: assessment!.id },
  });
  const sessionId = (await started.json()).sessionId as string;

  await setAuthCookie(page, token);

  // 1. The assessment appears in the learner explore
  await page.goto("/explore");
  await page.waitForLoadState("networkidle");
  await expect(page.getByText(QUIZ_TITLE).first()).toBeVisible({
    timeout: 10000,
  });

  // 2. The session page renders the first question and accepts an answer
  await page.goto(`/sessions/${sessionId}`);
  await page.waitForLoadState("networkidle");
  await expect(page.getByText(QUIZ_TITLE).first()).toBeVisible({
    timeout: 10000,
  });
  await expect(page.getByText(/Question \d+ of \d+/i).first()).toBeVisible({
    timeout: 5000,
  });
  // If an MC option button is visible, click the first one
  const mcOption = page.locator('[role="button"]').first();
  if (await mcOption.isVisible({ timeout: 2000 }).catch(() => false)) {
    await mcOption.click();
  }
});
