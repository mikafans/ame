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

// Authoring is no longer role-gated: every authenticated user can author
// quizzes, compose exams, grade, and reach the agent API surface.
test.describe("authoring navigation (all users)", () => {
  test.describe.configure({ mode: "serial" });

  let userToken: string;

  test.beforeAll(async ({ request }) => {
    userToken = await loginAs(request, "learner@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await withToken(page, userToken);
  });

  test("user can reach Author studio", async ({ page }) => {
    await page.goto(`${BASE}/author`);
    await expect(
      page.getByRole("heading", { name: "Author studio" }),
    ).toBeVisible({ timeout: 8000 });
  });

  test("user sees Author studio in nav", async ({ page }) => {
    await page.goto(`${BASE}/library`);
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("Author studio")).toBeVisible({
      timeout: 8000,
    });
  });

  test("user sees Agent API in nav", async ({ page }) => {
    await page.goto(`${BASE}/library`);
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("Agent API")).toBeVisible({ timeout: 8000 });
  });

  test("user can reach the agent API page", async ({ page }) => {
    await page.goto(`${BASE}/agent`);
    await expect(
      page.getByRole("heading", { name: "Agent integration" }),
    ).toBeVisible({ timeout: 8000 });
  });

  test("exams page is visible to all authenticated users", async ({ page }) => {
    await page.goto("/exams");
    await expect(page.getByRole("heading", { name: "Exams" })).toBeVisible({
      timeout: 8000,
    });
  });

  test("MCP tools tab renders descriptors without errors", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });

    await page.goto(`${BASE}/agent`);
    await page.getByText("MCP tools").click();
    await page.waitForTimeout(2000);
    const fatalErrors = errors.filter(
      (e) => !e.includes("net::ERR") && !e.includes("Failed to load resource"),
    );
    expect(fatalErrors).toHaveLength(0);
  });

  test("compose form is reachable and shows fields", async ({ page }) => {
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
    await withToken(page, token);
    await page.goto("/");
    await expect(page.getByText("Tweaks (demo)")).not.toBeVisible();
  });
});
