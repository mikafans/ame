import {
  test,
  expect,
  type Page,
  type APIRequestContext,
} from "@playwright/test";

const BASE = process.env.E2E_BASE_URL ?? "http://localhost:3000";
const API_URL = process.env.E2E_API_URL ?? "http://localhost:8080";

async function loginAs(
  request: APIRequestContext,
  email: string,
): Promise<string> {
  const resp = await request.post(`${API_URL}/v1/auth/login`, {
    data: { email, password: "password123" },
  });
  expect(resp.status()).toBe(200);
  return (await resp.json()).token;
}

async function withToken(page: Page, token: string) {
  await page
    .context()
    .addCookies([{ name: "ame_token", value: token, url: API_URL }]);
}

async function withLearner(page: Page, token: string) {
  await withToken(page, token);
}

async function withInstructor(page: Page, token: string) {
  await withToken(page, token);
}

test.describe("role-gated navigation", () => {
  let learnerToken: string;

  test.beforeAll(async ({ request }) => {
    learnerToken = await loginAs(request, "learner@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await withLearner(page, learnerToken);
  });

  test("learner cannot navigate to /author/*", async ({ page }) => {
    await page.goto("/author/some-quiz-id");
    const url = page.url();
    const hasAuthorContent = await page
      .getByText("Author studio")
      .isVisible()
      .catch(() => false);
    expect(url.includes("/login") || !hasAuthorContent).toBeTruthy();
  });

  test("learner does not see Author studio in nav", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Author studio")).not.toBeVisible();
  });

  test("learner does not see Agent API in nav", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Agent API")).not.toBeVisible();
  });

  test("learner cannot navigate to /agent", async ({ page }) => {
    await page.goto("/agent");
    const denied = await page
      .getByText("Agent integration is available to instructors")
      .isVisible()
      .catch(() => false);
    const redirected = page.url().includes("/login");
    expect(denied || redirected).toBeTruthy();
  });

  test("exams page is visible to all authenticated users", async ({ page }) => {
    await page.goto("/exams");
    await expect(page.getByRole("heading", { name: "Exams" })).toBeVisible({
      timeout: 8000,
    });
  });
});

// All instructor tests share one token — the API replaces the token hash on every
// login call, so multiple concurrent beforeAll hooks would invalidate each other.
test.describe("instructor flows", () => {
  test.describe.configure({ mode: "serial" });

  let instructorToken: string;

  test.beforeAll(async ({ request }) => {
    instructorToken = await loginAs(request, "instructor@example.com");
  });

  test("instructor can reach Author studio", async ({ page }) => {
    await withInstructor(page, instructorToken);
    await page.goto(`${BASE}/author`);
    await expect(
      page.getByRole("heading", { name: "Author studio" }),
    ).toBeVisible({ timeout: 8000 });
  });

  test("instructor sees Author studio in nav", async ({ page }) => {
    await withInstructor(page, instructorToken);
    await page.goto(`${BASE}/library`);
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("Author studio")).toBeVisible({
      timeout: 8000,
    });
  });

  test("instructor sees Agent API in nav", async ({ page }) => {
    await withInstructor(page, instructorToken);
    await page.goto(`${BASE}/library`);
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("Agent API")).toBeVisible({ timeout: 8000 });
  });

  test("MCP tools tab renders descriptors without errors", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });

    await withInstructor(page, instructorToken);
    await page.goto(`${BASE}/agent`);
    await page.getByText("MCP tools").click();
    await page.waitForTimeout(2000);
    const fatalErrors = errors.filter(
      (e) => !e.includes("net::ERR") && !e.includes("Failed to load resource"),
    );
    expect(fatalErrors).toHaveLength(0);
  });

  test("compose form is reachable and shows fields", async ({ page }) => {
    await withInstructor(page, instructorToken);
    await page.goto(`${BASE}/exams`);
    await page.waitForLoadState("networkidle");
    await expect(page.getByRole("heading", { name: "Exams" })).toBeVisible({
      timeout: 8000,
    });
    await page.getByText("Compose exam").click();
    await expect(
      page.getByRole("heading", { name: "Compose exam" }),
    ).toBeVisible({ timeout: 5000 });
    await expect(page.getByLabel("Name")).toBeVisible();
    // The Sections sub-form — target the dialog-unique "Add section" button
    // rather than the word "Sections", which now also appears in the exam
    // detail panel behind the modal.
    await expect(
      page.getByRole("button", { name: "Add section" }),
    ).toBeVisible();
  });
});

test.describe("tweaks panel (demo mode)", () => {
  test("tweaks panel not visible when DEMO_MODE is off", async ({
    page,
    request,
  }) => {
    const token = await loginAs(request, "learner@example.com");
    await withLearner(page, token);
    await page.goto("/");
    await expect(page.getByText("Tweaks (demo)")).not.toBeVisible();
  });
});
