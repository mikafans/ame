import { test, expect } from "@playwright/test";

const BASE = process.env.E2E_BASE_URL ?? "http://localhost:3000";
const API = process.env.E2E_API_URL ?? "http://localhost:8080";

test.describe("Instructor role", () => {
  test.describe.configure({ mode: "serial" });

  let token: string;

  test.beforeAll(async ({ request }) => {
    const r = await request.post(`${API}/v1/auth/login`, {
      data: { email: "admin@example.com", password: "password123" },
    });
    expect(r.status()).toBe(200);
    token = (await r.json()).token;
  });

  test("grading page accessible and shows queue", async ({ page }) => {
    await page
      .context()
      .addCookies([
        { name: "ame_token", value: token, domain: "localhost", path: "/" },
      ]);
    await page.goto(`${BASE}/grading`);
    await expect(
      page.locator("text=/essay grading|no essays pending/i").first(),
    ).toBeVisible({ timeout: 10000 });
  });

  test("exams page accessible and shows compose button", async ({ page }) => {
    await page
      .context()
      .addCookies([
        { name: "ame_token", value: token, domain: "localhost", path: "/" },
      ]);
    await page.goto(`${BASE}/exams`);
    await expect(page.locator("text=/compose exam/i").first()).toBeVisible({
      timeout: 10000,
    });
  });

  test("agent page accessible and shows API keys tab", async ({ page }) => {
    await page
      .context()
      .addCookies([
        { name: "ame_token", value: token, domain: "localhost", path: "/" },
      ]);
    await page.goto(`${BASE}/agent`);
    await expect(
      page.locator("text=/new api key|api key/i").first(),
    ).toBeVisible({ timeout: 10000 });
  });
});
