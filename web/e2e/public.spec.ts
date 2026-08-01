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
      page.getByRole("link", { name: "llms.txt", exact: true }),
    ).toHaveAttribute("href", "/public/llms.txt");
  });

  test("agent guide is public and does not redirect to login", async ({
    page,
  }) => {
    await page.goto("/agent");

    await expect(page).toHaveURL(/\/agent$/);
    await expect(
      page.getByRole("heading", { name: "Bring your agent to AME" }),
    ).toBeVisible();
    await expect(
      page.getByRole("link", { name: "Skill manifest" }),
    ).toHaveAttribute("href", "/public/skill.json");
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
