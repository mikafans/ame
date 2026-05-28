import {
  expect,
  test,
  type APIRequestContext,
  type Page,
} from "@playwright/test";

const API_URL = process.env.E2E_API_URL ?? "http://localhost:8080";

type LoginResponse = {
  token: string;
};

type QuizListResponse = {
  quizzes: Array<{ id: string; title: string }>;
};

type SessionResponse = {
  session: { id: string };
  questions: Array<{
    questionId: string;
    kind: string;
    options?: Array<{ text: string }>;
  }>;
};

async function login(request: APIRequestContext) {
  const response = await request.post(`${API_URL}/v1/auth/login`, {
    data: { email: "learner@example.com", password: "password123" },
  });
  expect(response.ok()).toBeTruthy();
  return (await response.json()) as LoginResponse;
}

async function setAuthCookie(page: Page, token: string) {
  await page
    .context()
    .addCookies([
      { name: "ame_token", value: token, domain: "localhost", path: "/" },
    ]);
}

async function screenshot(page: Page, name: string) {
  await page.screenshot({
    path: `../.tmp/uiux-${name}.png`,
    fullPage: true,
    scale: "css",
  });
}

async function firstActiveQuizId(request: APIRequestContext, token: string) {
  const response = await request.get(`${API_URL}/v1/quizzes?status=active`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(response.ok()).toBeTruthy();
  const data = (await response.json()) as QuizListResponse;
  expect(data.quizzes.length).toBeGreaterThan(0);
  return data.quizzes[0].id;
}

async function firstMcqQuizId(
  request: APIRequestContext,
  token: string,
): Promise<string> {
  const listResp = await request.get(`${API_URL}/v1/quizzes?status=active`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(listResp.ok()).toBeTruthy();
  const { quizzes } = (await listResp.json()) as QuizListResponse;
  for (const q of quizzes) {
    const detail = await request.get(`${API_URL}/v1/quizzes/${q.id}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!detail.ok()) continue;
    const d = await detail.json();
    const questions: Array<{ kind: string }> = d.questions ?? [];
    if (questions.some((qs) => qs.kind === "mc" || qs.kind === "mcq")) {
      return q.id;
    }
  }
  throw new Error("No active quiz with MCQ questions found");
}

async function finishSession(
  request: APIRequestContext,
  token: string,
  sessionId: string,
) {
  const stateResponse = await request.get(
    `${API_URL}/v1/sessions/${sessionId}`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(stateResponse.ok()).toBeTruthy();
  const state = (await stateResponse.json()) as SessionResponse;

  for (const question of state.questions) {
    const body = responseFor(question.kind);
    const answerResponse = await request.post(
      `${API_URL}/v1/sessions/${sessionId}/answer`,
      {
        headers: { Authorization: `Bearer ${token}` },
        data: { questionId: question.questionId, response: body },
      },
    );
    expect(answerResponse.ok()).toBeTruthy();
  }

  const finishResponse = await request.post(
    `${API_URL}/v1/sessions/${sessionId}/finish`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(finishResponse.ok()).toBeTruthy();
}

function responseFor(kind: string) {
  switch (kind) {
    case "mc":
    case "mcq":
      return { selected_position: 0 };
    case "tf":
      return { answer: false };
    case "essay":
      return {
        body: "Depth-first search explores a path before backtracking, while breadth-first search explores by distance from the start. I would choose DFS for cycle detection or topological ordering and BFS for shortest paths in unweighted graphs.",
        word_count: 31,
      };
    case "code":
      return { source: "def solve():\n    return None", language: "python" };
    default:
      return { answer: "stack" };
  }
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

    const { token } = await login(request);
    await setAuthCookie(page, token);

    // --- Library ---
    await page.goto("/library");
    await expect(page.getByRole("heading", { name: "Library" })).toBeVisible();
    await expect(
      page.getByRole("tab", { name: /All quizzes \(\d+\)/ }),
    ).toBeVisible();
    await expect(page.getByText("Up next")).toBeVisible({ timeout: 8000 });
    await expect(page.getByText("Questions", { exact: true })).toBeVisible();
    await expect(page.getByText("Duration", { exact: true })).toBeVisible();
    await expect(page.getByText("Attempts", { exact: true })).toBeVisible();
    await expect(page.getByText("Recommended prep")).toBeVisible();
    await screenshot(page, "library");

    // --- Quiz preview (use a quiz with MCQ questions for the session test) ---
    const quizId = await firstMcqQuizId(request, token);
    await page.goto(`/quizzes/${quizId}/preview`);
    await expect(page.getByText("Library")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back to library" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Start quiz" }),
    ).toBeVisible();
    await expect(page.getByText(/questions/i).first()).toBeVisible();
    await expect(page.getByText(/pts/i).first()).toBeVisible();
    await screenshot(page, "quiz-preview");

    // --- Active session: navigate to an MCQ question ---
    await page.getByRole("button", { name: "Start quiz" }).click();
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
    // A/B/C/D labels should be present
    await expect(optionInput.getByText("A")).toBeVisible();
    await firstOption.click();
    await expect(page.getByText(/\d+\/\d+ answered/)).toBeVisible();
    await screenshot(page, "active-session");

    // --- Results: answers not blank ---
    await finishSession(request, token, sessionId!);
    await page.goto(`/sessions/${sessionId}/results`);
    await expect(page.getByRole("heading", { name: /Results/ })).toBeVisible();
    await expect(page.getByText("Answer review")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back to library" }),
    ).toBeVisible();
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
