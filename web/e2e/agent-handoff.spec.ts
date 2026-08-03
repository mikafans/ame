import { expect, test } from "@playwright/test";

const baseUrl = process.env.E2E_BASE_URL ?? "http://localhost:23000";

test.describe("agent handoff", () => {
  test("asks an anonymous visitor to sign in before granting access", async ({
    page,
  }) => {
    await page.route("**/api/v1/me", (route) => route.fulfill({ status: 401 }));
    await page.goto("/agent");

    await expect(
      page.getByRole("heading", {
        name: "Sign in before you give an agent access.",
      }),
    ).toBeVisible();
    await expect(
      page.getByRole("link", { name: "Sign in to create a handoff" }),
    ).toHaveAttribute("href", "/login?returnTo=/agent");
  });

  test("creates a one-time handoff without putting its secret in a URL", async ({
    page,
  }) => {
    let delegationReads = 0;
    await page
      .context()
      .addCookies([{ name: "ame_session", value: "1", url: baseUrl }]);
    await page.route("**/api/v1/me", (route) =>
      route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          id: "learner-1",
          displayName: "Haru",
          email: "haru@example.test",
          role: "user",
        }),
      }),
    );
    await page.route("**/api/v1/agent-delegations", async (route) => {
      if (route.request().method() === "GET") {
        delegationReads += 1;
        await route.fulfill({ contentType: "application/json", body: "[]" });
        return;
      }
      await expect(route.request().postDataJSON()).toEqual({
        goal: "Learn Flink state recovery",
        expiresInMinutes: 60,
      });
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          delegation: {
            id: "delegation-1",
            goal: "Learn Flink state recovery",
            expiresAt: "2026-08-03T12:00:00Z",
          },
          handoff:
            "Authorization: Bearer dlg_delegation-1_one-time-secret\nGoal: Learn Flink state recovery",
        }),
      });
    });

    await page.goto("/agent");
    await page
      .getByLabel("What should the agent prepare?")
      .fill("Learn Flink state recovery");
    await page.getByRole("button", { name: "Create secure handoff" }).click();

    await expect(page.getByLabel("One-time agent handoff")).toHaveValue(
      /Authorization: Bearer dlg_delegation-1_one-time-secret/,
    );
    expect(page.url()).not.toContain("dlg_delegation-1_one-time-secret");
    expect(delegationReads).toBeGreaterThan(0);
  });

  test("sends a signed-in learner from the fallback form to agent handoff", async ({
    page,
  }) => {
    await page
      .context()
      .addCookies([{ name: "ame_session", value: "1", url: baseUrl }]);
    await page.route("**/api/v1/me", (route) =>
      route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          id: "learner-1",
          displayName: "Haru",
          role: "user",
        }),
      }),
    );
    await page.route("**/api/v1/agent-delegations", (route) =>
      route.fulfill({ contentType: "application/json", body: "[]" }),
    );

    await page.goto("/start");

    await expect(page).toHaveURL(/\/agent$/);
    await expect(
      page.getByRole("heading", {
        name: "Tell your agent what you want to learn. It sets up the course.",
      }),
    ).toBeVisible();
    await expect(page.getByLabel("What would you like to learn?")).toHaveCount(
      0,
    );
  });
});
