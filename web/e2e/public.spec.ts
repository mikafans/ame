import { expect, test } from "@playwright/test";

test.describe("public documentation", () => {
  test("landing page exposes self-hosting links", async ({ page }) => {
    await page.goto("/");

    await expect(
      page.getByRole("link", { name: "Self-hosting", exact: true }),
    ).toHaveAttribute("href", "/self-hosting");
    await expect(
      page.getByRole("link", { name: "Agent API", exact: true }),
    ).toHaveAttribute("href", "/agent");
    await expect(
      page.getByRole("link", { name: "Agent guide", exact: true }),
    ).toHaveAttribute("href", "/agent");
    await expect(
      page.getByRole("link", { name: "How agent setup works", exact: true }),
    ).toHaveAttribute("href", "/agent");
  });

  test("agent guide is public and does not redirect to login", async ({
    page,
  }) => {
    const browserErrors: string[] = [];
    page.on("pageerror", (error) => browserErrors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") browserErrors.push(message.text());
    });

    await page.goto("/agent");

    await expect(page).toHaveURL(/\/agent$/);
    await expect(
      page.getByRole("heading", { name: /Give your agent AME/ }),
    ).toBeVisible();
    await expect(
      page.getByRole("link", { name: "Skill manifest" }),
    ).toHaveAttribute("href", "/public/skill.json");
    expect(browserErrors).toEqual([]);
  });

  test("agent guide fits a narrow learner viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/agent");

    await expect(
      page.getByRole("heading", { name: /Give your agent AME/ }),
    ).toBeVisible();
    const dimensions = await page.evaluate(() => ({
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: document.documentElement.scrollWidth,
    }));
    expect(dimensions.scrollWidth).toBe(dimensions.clientWidth);
  });

  test("self-hosting page renders the operator guide", async ({ page }) => {
    await page.goto("/self-hosting");

    await expect(
      page.getByRole("heading", { name: "Self-hosting AME" }),
    ).toBeVisible();
    await expect(page.getByText("Configure and start")).toBeVisible();
    await expect(
      page.getByRole("link", { name: "Back to AME" }),
    ).toHaveAttribute("href", "/");
  });
});
