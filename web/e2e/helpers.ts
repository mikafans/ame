/**
 * Shared e2e helpers.
 *
 * All specs import from here to avoid copy-pasting login/cookie/session
 * boilerplate. Keep this module free of test-framework side effects —
 * it must be importable from any spec without running setup hooks.
 */
import { type APIRequestContext, type Page, expect } from "@playwright/test";

export const API_URL = process.env.E2E_API_URL ?? "http://localhost:28080";
export const BASE_URL = process.env.E2E_BASE_URL ?? "http://localhost:23000";

// ---------------------------------------------------------------------------
// Auth helpers
// ---------------------------------------------------------------------------

/** Sleep for `ms` milliseconds. */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * POST /v1/auth/login and return the bearer token.
 * Retries up to 5 times on 429 (auth endpoint rate-limit).
 */
export async function loginAs(
  request: APIRequestContext,
  email: string,
  password = "password123",
): Promise<string> {
  for (let attempt = 0; attempt < 5; attempt++) {
    const resp = await request.post(`${API_URL}/v1/auth/login`, {
      data: { email, password },
    });
    if (resp.status() === 429) {
      await sleep(2000 * (attempt + 1));
      continue;
    }
    expect(resp.status(), `loginAs(${email}) failed`).toBe(200);
    return (await resp.json()).token as string;
  }
  throw new Error(`loginAs(${email}): too many retries on 429`);
}

/**
 * POST /v1/auth/register and return the bearer token.
 * Retries up to 5 times on 429 (auth endpoint rate-limit).
 */
export async function registerUser(
  request: APIRequestContext,
  opts: { email: string; password?: string; name: string },
): Promise<string> {
  return (await registerUserFull(request, opts)).token;
}

/**
 * POST /v1/auth/register and return both the bearer token and the user id.
 * Retries up to 5 times on 429 (auth endpoint rate-limit).
 */
export async function registerUserFull(
  request: APIRequestContext,
  opts: { email: string; password?: string; name: string },
): Promise<{ token: string; id: string }> {
  const password = opts.password ?? "password123";
  for (let attempt = 0; attempt < 5; attempt++) {
    const resp = await request.post(`${API_URL}/v1/auth/register`, {
      data: { email: opts.email, name: opts.name, password, role: "user" },
    });
    if (resp.status() === 429) {
      await sleep(2000 * (attempt + 1));
      continue;
    }
    expect(resp.status(), `registerUserFull(${opts.email}) failed`).toBe(201);
    const body = await resp.json();
    return { token: body.token as string, id: body.user?.id ?? body.id };
  }
  throw new Error(`registerUserFull(${opts.email}): too many retries on 429`);
}

/**
 * Set the ame_token cookie on the page so subsequent navigations are
 * authenticated. Must be called before the first page.goto().
 */
export async function setAuthCookie(page: Page, token: string): Promise<void> {
  const domain = new URL(BASE_URL).hostname;
  await page
    .context()
    .addCookies([{ name: "ame_token", value: token, domain, path: "/" }]);
}

// ---------------------------------------------------------------------------
// Session helpers
// ---------------------------------------------------------------------------

type QuestionStub = { questionId?: string; question_id?: string; kind: string };

/** Build a valid answer payload for any question kind. */
export function makeResponse(kind: string): Record<string, unknown> {
  switch (kind) {
    case "mc":
    case "mcq":
      return { selected_position: 0 };
    case "tf":
      return { answer: false };
    case "short":
      return { answer: "stack" };
    case "essay":
      return {
        body: "Sample essay answer for e2e testing purposes covering the topic adequately.",
        word_count: 12,
      };
    case "code":
      return { source: "def solve():\n    return None", language: "python" };
    default:
      return { answer: "e2e answer" };
  }
}

/** Answer all questions in a session and call /finish. */
export async function finishSession(
  request: APIRequestContext,
  token: string,
  sessionId: string,
): Promise<void> {
  const stateResp = await request.get(`${API_URL}/v1/sessions/${sessionId}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(stateResp.ok(), "fetch session state").toBeTruthy();
  const state = await stateResp.json();
  const questions: QuestionStub[] = state.questions ?? [];

  for (const q of questions) {
    const qid = String(q.questionId ?? q.question_id ?? "");
    await request.post(`${API_URL}/v1/sessions/${sessionId}/answer`, {
      headers: { Authorization: `Bearer ${token}` },
      data: { questionId: qid, response: makeResponse(q.kind) },
    });
  }

  const finish = await request.post(
    `${API_URL}/v1/sessions/${sessionId}/finish`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(finish.ok(), "finish session").toBeTruthy();
}

// ---------------------------------------------------------------------------
// Quiz helpers
// ---------------------------------------------------------------------------

/**
 * Return the id of the first active quiz that contains at least one MCQ
 * AND has at least minQuestions total questions (default 5).
 *
 * The minimum count avoids matching small e2e test-fixture quizzes (e.g.,
 * visibility-spec quizzes with 1 question) and ensures the returned quiz
 * has enough content for a meaningful session round-trip.
 */
export async function firstMcqQuizId(
  request: APIRequestContext,
  token: string,
  minQuestions = 5,
): Promise<string> {
  const listResp = await request.get(
    `${API_URL}/v1/assessments?status=active`,
    {
      headers: { Authorization: `Bearer ${token}` },
    },
  );
  expect(listResp.ok(), "list assessments").toBeTruthy();
  const { assessments } = await listResp.json();
  for (const q of assessments as Array<{ id: string }>) {
    const detail = await request.get(`${API_URL}/v1/assessments/${q.id}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!detail.ok()) continue;
    const d = await detail.json();
    const questions: Array<{ kind: string }> = d.questions ?? [];
    if (
      questions.length >= minQuestions &&
      questions.some((qs) => qs.kind === "mc" || qs.kind === "mcq")
    ) {
      return q.id;
    }
  }
  throw new Error(
    `No active quiz with MCQ questions and >= ${minQuestions} total questions found`,
  );
}
