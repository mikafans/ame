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
  await page.goto("/login");
  await page.evaluate((t) => {
    document.cookie = `ame_token=${t}; path=/; max-age=86400; SameSite=Lax`;
  }, token);
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

async function finishSession(
  request: APIRequestContext,
  token: string,
  sessionId: string,
) {
  const stateResponse = await request.get(
    `${API_URL}/v1/sessions/${sessionId}`,
    {
      headers: { Authorization: `Bearer ${token}` },
    },
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
      return { body: "def solve():\n    return None", language: "python" };
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

    await page.goto("/library");
    await expect(page.getByRole("heading", { name: "Library" })).toBeVisible();
    await expect(
      page.getByRole("button", { name: /All quizzes \(\d+\)/ }),
    ).toBeVisible();
    await expect(page.getByText("Up next")).toBeVisible();
    await expect(page.getByText("Due in 2 days")).toBeVisible();
    await expect(page.getByText("Questions", { exact: true })).toBeVisible();
    await expect(page.getByText("Duration", { exact: true })).toBeVisible();
    await expect(page.getByText("Attempts", { exact: true })).toBeVisible();
    await expect(page.getByText("Recommended prep")).toBeVisible();
    await screenshot(page, "library");

    const quizId = await firstActiveQuizId(request, token);
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

    await page.getByRole("button", { name: "Start quiz" }).click();
    await expect(page).toHaveURL(/\/sessions\/[0-9a-f-]+$/);
    await expect(page.getByText(/Attempt 1 of 2/)).toBeVisible();
    await expect(page.getByText(/Question 1 of/)).toBeVisible();
    await expect(page.getByText("Question palette")).toBeVisible();
    await expect(page.getByText("Integrity")).toBeVisible();
    await expect(page.getByText("Allowed")).toBeVisible();
    await expect(page.getByText("One sheet of notes (any)")).toBeVisible();
    await expect(page.getByText(/\d{1,2}:\d{2}/)).toBeVisible();
    await page.getByRole("button", { name: "O(log n)" }).click();
    await expect(page.getByText("Answered · 1", { exact: true })).toBeVisible();
    await screenshot(page, "active-session");

    const sessionId = page.url().split("/").pop();
    expect(sessionId).toBeTruthy();
    await finishSession(request, token, sessionId!);
    await page.goto(`/sessions/${sessionId}/results`);
    await expect(page.getByRole("heading", { name: /Results/ })).toBeVisible();
    await expect(page.getByText("Cohort distribution")).toBeVisible();
    await expect(page.getByText("Answer review")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back to library" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "View 6-week plan" }),
    ).toBeVisible();
    await screenshot(page, "results");

    await page.goto("/progress");
    await expect(
      page.getByRole("heading", { name: "Progress dashboard" }),
    ).toBeVisible();
    await expect(page.getByText("AVG SCORE")).toBeVisible();
    await expect(page.getByText("ATTEMPTS")).toBeVisible();
    await expect(page.getByText("CURRENT STREAK")).toBeVisible();
    await screenshot(page, "progress");

    await page.goto("/exams");
    await expect(page.getByRole("heading", { name: "Exams" })).toBeVisible();
    await expect(page.getByRole("button", { name: /All\(/ })).toBeVisible();
    await expect(page.getByText("Composition")).toBeVisible();
    await screenshot(page, "exams");

    const fatalErrors = consoleErrors.filter(
      (error) =>
        !error.includes("Download the React DevTools") &&
        !error.includes("webpack-hmr"),
    );
    expect(fatalErrors).toEqual([]);
  });
});
