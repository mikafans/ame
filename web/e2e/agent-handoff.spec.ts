import { expect, test } from "@playwright/test";

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
});
