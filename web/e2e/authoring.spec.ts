/**
 * Authoring surface — every authenticated user can author, NOT admin-gated.
 *
 * Covers: Author studio, Agent API page, MCP tools tab, Exams compose form.
 * Source: plan10-author-agent.spec.ts (restructured, deduped with helpers).
 */
import { test, expect } from "@playwright/test";
import { loginAs, setAuthCookie } from "./helpers";

test.describe("authoring navigation (all authenticated users)", () => {
  test.describe.configure({ mode: "serial" });

  let token: string;

  test.beforeAll(async ({ request }) => {
    // Any authenticated user — use the seeded learner (no special role needed)
    token = await loginAs(request, "learner@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("user can reach Author studio", async ({ page }) => {
    await page.goto("/author");
    await expect(
      page.getByRole("heading", { name: "Author studio" }),
    ).toBeVisible({ timeout: 8000 });
  });

  test("Author studio appears in sidebar nav", async ({ page }) => {
    await page.goto("/library");
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("Author studio")).toBeVisible({
      timeout: 8000,
    });
  });

  test("Agent API appears in sidebar nav", async ({ page }) => {
    await page.goto("/library");
    await page.waitForLoadState("networkidle");
    await expect(page.getByText("Agent API")).toBeVisible({ timeout: 8000 });
  });

  test("user can reach the agent API page", async ({ page }) => {
    await page.goto("/agent");
    await expect(
      page.getByRole("heading", { name: "Agent integration" }),
    ).toBeVisible({ timeout: 8000 });
  });

  test("exams page is accessible to all authenticated users", async ({
    page,
  }) => {
    await page.goto("/exams");
    await expect(page.getByRole("heading", { name: "Exams" })).toBeVisible({
      timeout: 8000,
    });
  });

  test("MCP tools tab renders without fatal JS errors", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });

    await page.goto("/agent");
    await page.getByText("MCP tools").click();
    await page.waitForTimeout(2000);
    // Filter out network-level errors: CORS policy violations and fetch failures
    // are expected in the local dev environment where the API and web server run
    // on different ports without a shared reverse proxy.
    const fatalErrors = errors.filter(
      (e) =>
        !e.includes("net::ERR") &&
        !e.includes("Failed to load resource") &&
        !e.includes("CORS policy") &&
        !e.includes("blocked by CORS") &&
        !e.includes("Failed to fetch"),
    );
    expect(fatalErrors).toHaveLength(0);
  });

  test("compose exam form is reachable and shows required fields", async ({
    page,
  }) => {
    await page.goto("/exams");
    await page.waitForLoadState("networkidle");
    await page.getByText("Compose exam").click();
    await expect(
      page.getByRole("heading", { name: "Compose exam" }),
    ).toBeVisible({ timeout: 5000 });
    await expect(page.getByLabel("Name")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Add section" }),
    ).toBeVisible();
  });
});
