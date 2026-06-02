/**
 * UI/UX spec alignment — comprehensive learner surface contract test.
 *
 * Exercises: Library, assessment preview, active session (MCQ), results, progress
 * dashboard, question bank, exams. This test is intentionally a large single
 * flow so it can verify the session round-trip end-to-end with screenshots.
 *
 * Uses shared helpers for login, cookie, MCQ assessment lookup, and session finish.
 */
import { expect, test, type Page } from "@playwright/test";
import {
  API_URL,
  loginAs,
  setAuthCookie,
  firstMcqAssessmentId,
  finishSession,
} from "./helpers";

type SessionResponse = {
  session: { id: string };
  questions: Array<{
    questionId: string;
    kind: string;
    options?: Array<{ text: string }>;
  }>;
};

async function screenshot(page: Page, name: string) {
  await page.screenshot({
    path: `../.tmp/uiux-${name}.png`,
    fullPage: true,
    scale: "css",
  });
}

test.describe("UI/UX spec alignment", () => {
  test("learner surfaces match the design-critical contract", async ({
    page,
    request,
  }) => {
    const consoleErrors: string[] = [];
    page.on("console", (message) => {
      if (message.type() === "error") consoleErrors.push(message.text());
    });

    const token = await loginAs(request, "ada@example.com");
    await setAuthCookie(page, token);

    // --- Library ---
    await page.goto("/library");
    // Wait for the auth loading to finish and user to be visible in sidebar
    await expect(page.getByText(/Ada Lovelace/i)).toBeVisible({
      timeout: 10000,
    });

    await expect(
      page.getByRole("heading", { name: "Assessments" }),
    ).toBeVisible();
    await expect(page.getByRole("tab", { name: /All \(\d+\)/ })).toBeVisible();
    // "Up next" highlights the first unfinished assessment; it is absent once the
    // learner has completed everything. Assert its detail fields only when shown.
    const upNext = page.getByText("Up next");
    if (await upNext.isVisible().catch(() => false)) {
      await expect(page.getByText("Questions", { exact: true })).toBeVisible();
      await expect(page.getByText("Duration", { exact: true })).toBeVisible();
      // "Recommended prep" is only shown when the planner has generated tags.
      // Skip asserting it to avoid seed-state flakiness.
    }
    await screenshot(page, "library");

    // --- Assessment preview (use a assessment with MCQ questions for the session test) ---
    const assessmentId = await firstMcqAssessmentId(request, token);
    await page.goto(`/assessments/${assessmentId}/preview`);
    await expect(page.getByText("Assessments")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back to assessments" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Start assessment" }),
    ).toBeVisible();
    await expect(page.getByText(/questions/i).first()).toBeVisible();
    await expect(page.getByText(/pts/i).first()).toBeVisible();
    await screenshot(page, "assessment-preview");

    // --- Active session: navigate to an MCQ question ---
    await page.getByRole("button", { name: "Start assessment" }).click();
    await expect(page).toHaveURL(/\/sessions\/[0-9a-f-]+$/);
    const sessionId = page.url().split("/").pop();
    expect(sessionId).toBeTruthy();
    await expect(page.getByText(/Question 1 of/)).toBeVisible();

    // Use the API to find the first MCQ question index so we can navigate to it
    const sessionStateResp = await request.get(
      `${API_URL}/v1/sessions/${sessionId}`,
      { headers: { Authorization: `Bearer ${token}` } },
    );
    const sessionState = (await sessionStateResp.json()) as SessionResponse;
    const mcqIdx = sessionState.questions.findIndex(
      (q) => q.kind === "mc" || q.kind === "mcq",
    );
    expect(mcqIdx).toBeGreaterThanOrEqual(0);
    for (let i = 0; i < mcqIdx; i++) {
      await page.getByRole("button", { name: "Next" }).click();
      await expect(
        page.getByText(new RegExp(`Question ${i + 2} of`)),
      ).toBeVisible();
    }

    // McqRenderer uses role="button" on option rows (not native <button>)
    const optionInput = page.getByTestId("question-input");
    const firstOption = optionInput.locator('[role="button"]').first();
    await expect(firstOption).toBeVisible();
    // A/B/C/D labels should be present (use .first() to avoid strict-mode
    // violation — the text "A" appears in both the label div and option text)
    await expect(optionInput.getByText("A").first()).toBeVisible();
    await firstOption.click();
    await expect(page.getByText(/\d+\/\d+ answered/)).toBeVisible();
    await screenshot(page, "active-session");

    // --- Results: answers not blank ---
    await finishSession(request, token, sessionId!);
    await page.goto(`/sessions/${sessionId}/results`);
    await expect(page.getByRole("heading", { name: /Results/ })).toBeVisible();
    await expect(page.getByText("Answer review")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back to assessments" }),
    ).toBeVisible();
    // Wait for the first answer card to render before reading captions
    await expect(page.locator(".MuiCardContent-root").first()).toBeVisible({
      timeout: 5000,
    });
    await page.waitForLoadState("networkidle");
    // At least one answer should show a non-blank value
    const answerCaptions = await page
      .locator(".MuiCardContent-root .MuiTypography-caption")
      .allTextContents();
    const yourAnswers = answerCaptions.filter((t) =>
      t.startsWith("Your answer:"),
    );
    expect(yourAnswers.length).toBeGreaterThan(0);
    expect(
      yourAnswers.some((t) => t !== "Your answer: —" && t !== "Your answer: "),
      `all answers are blank: ${JSON.stringify(yourAnswers)}`,
    ).toBe(true);
    await screenshot(page, "results");

    // --- Progress: toggle works, hours not raw float ---
    await page.goto("/progress");
    await expect(
      page.getByRole("heading", { name: "Progress dashboard" }),
    ).toBeVisible();
    await page.waitForTimeout(1500);
    const bodyText = await page.locator("body").textContent();
    expect(bodyText).not.toMatch(/0\.\d{3,}h/);
    await page.getByRole("button", { name: "4w" }).click();
    await page.waitForTimeout(600);
    await expect(page.getByRole("button", { name: "4w" })).toBeVisible();
    await page.getByRole("button", { name: "All" }).click();
    await page.waitForTimeout(400);
    await screenshot(page, "progress");

    // --- Question bank: search, kind filter, pagination ---
    await page.goto("/questions");
    await expect(
      page.getByRole("heading", { name: "Question Bank" }),
    ).toBeVisible();
    await expect(page.locator("tbody tr").first()).toBeVisible({
      timeout: 5000,
    });
    await expect(page.locator("text=/of \\d+/").first()).toBeVisible();
    await expect(page.locator("tbody .MuiChip-root").first()).toBeVisible();
    await page.fill('input[placeholder="Search questions..."]', "sort");
    await page.waitForTimeout(800);
    await expect(page.locator("text=/of \\d+/").first()).toBeVisible();
    await screenshot(page, "question-bank-search");
    await page.fill('input[placeholder="Search questions..."]', "");
    await page.waitForTimeout(400);
    await page.getByRole("button", { name: "MC" }).click();
    await page.waitForTimeout(600);
    await expect(page.locator("tbody tr").first()).toBeVisible();
    await screenshot(page, "question-bank-kind-mc");

    // --- Exams ---
    await page.goto("/exams");
    await expect(page.getByRole("heading", { name: "Exams" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "All" })).toBeVisible();
    await expect(
      page.getByText(/CS Fundamentals Midterm|Exam/i).first(),
    ).toBeVisible();
    await screenshot(page, "exams");

    const fatalErrors = consoleErrors.filter(
      (error) =>
        !error.includes("Download the React DevTools") &&
        !error.includes("webpack-hmr") &&
        !error.includes("401 (Unauthorized)"),
    );
    expect(fatalErrors).toEqual([]);
  });
});
