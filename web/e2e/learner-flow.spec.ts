import { test, expect } from "@playwright/test";

const API_URL = process.env.E2E_API_URL ?? "http://localhost:8080";

test.describe("learner golden path", () => {
  let learnerToken: string;

  test.beforeAll(async ({ request }) => {
    const resp = await request.post(`${API_URL}/v1/auth/login`, {
      data: { email: "learner@example.com", password: "password123" },
    });
    learnerToken = (await resp.json()).token;
  });

  test.beforeEach(async ({ page }) => {
    await page.context().addCookies([
      {
        name: "ame_token",
        value: learnerToken,
        url: API_URL,
      },
    ]);
  });

  test("login page renders and rejects bad credentials", async ({ page }) => {
    await page.goto("/login");
    await page.fill('input[type="email"]', "nobody@example.com");
    await page.fill('input[type="password"]', "badpass");
    await page.click('button[type="submit"]');
    await expect(page.getByText("missing or invalid token")).toBeVisible({
      timeout: 8000,
    });
  });

  test("library screen loads", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Library" })).toBeVisible();
  });

  test("practice setup form renders", async ({ page }) => {
    await page.goto("/practice");
    await expect(page.getByText("Practice")).toBeVisible();
    await expect(page.getByText("Topics")).toBeVisible();
  });

  test("progress page renders", async ({ page }) => {
    await page.goto("/progress");
    await expect(
      page.getByRole("heading", { name: "Progress dashboard" }),
    ).toBeVisible();
  });

  test("unauthenticated user redirects to login", async ({ page, context }) => {
    await context.clearCookies();
    await page.goto("/");
    await page.waitForURL("**/login", { timeout: 5000 });
  });
});
