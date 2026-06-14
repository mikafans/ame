/**
 * UI/UX spec alignment — comprehensive learner surface contract test.
 *
 * Exercises: Explore, assessment preview, active session (MCQ), results, progress
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

    // --- Explore ---
    await page.goto("/explore");
    // Wait for the auth loading to finish and user to be visible in sidebar
    console.log(await page.locator(".MuiDrawer-paper").innerText());
    await expect(page.getByText(/Ada Lovelace/i).first()).toBeVisible({
      timeout: 10000,
    });

    await expect(page.getByRole("heading", { name: "Explore" })).toBeVisible();
    await expect(page.getByRole("button", { name: "All" })).toBeVisible();
    // TODO(explore): verify that the "Up next" or equivalent concept still exists on Explore
    // If Explore has a featured/recommended section, verify the assertions still apply
    await screenshot(page, "explore");

    // --- Assessment preview (use a assessment with MCQ questions for the session test) ---
    const assessmentId = await firstMcqAssessmentId(request, token);
    await page.goto(`/assessments/${assessmentId}/preview`);
    await expect(page.getByText("Explore", { exact: true })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back to Explore" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /Start (exam|assessment)/ }),
    ).toBeVisible();
    await expect(page.getByText(/questions/i).first()).toBeVisible();
    await expect(page.getByText(/pts/i).first()).toBeVisible();
    await screenshot(page, "assessment-preview");

    // --- Active session: navigate to an MCQ question ---
    await page.getByRole("button", { name: /Start (exam|assessment)/ }).click();
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
      page.getByRole("button", { name: "Back to Explore" }),
    ).toBeVisible();
    // Wait for the first answer card to render before reading captions
    await expect(page.locator(".MuiCardContent-root").first()).toBeVisible({
      timeout: 5000,
    });
    await page.waitForLoadState("networkidle");
    // At least one answer should show a non-blank value
    const cardContents = await page
      .locator(".MuiCardContent-root")
      .allTextContents();
    const hasNonBlankAnswer = cardContents.some((content) => {
      if (
        content.includes("Correct:") &&
        !content.includes("Correct: —") &&
        !content.includes("Correct:  ")
      ) {
        return true;
      }
      if (content.includes("Your Answer")) {
        const matches = content.match(/Your Answer\s*(.+)/s);
        if (
          matches &&
          matches[1] &&
          !matches[1].trim().startsWith("—") &&
          matches[1].trim() !== ""
        ) {
          return true;
        }
      }
      if (content.includes("Your submission")) {
        const matches = content.match(/Your submission\s*(.+)/s);
        if (
          matches &&
          matches[1] &&
          !matches[1].trim().startsWith("—") &&
          matches[1].trim() !== ""
        ) {
          return true;
        }
      }
      return false;
    });
    expect(hasNonBlankAnswer).toBe(true);
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
