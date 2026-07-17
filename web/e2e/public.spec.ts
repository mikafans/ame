import { expect, test } from "@playwright/test";

test.describe("public documentation", () => {
  test("landing page exposes self-hosting links", async ({ page }) => {
    await page.goto("/");

    await expect(
      page.getByRole("link", { name: /Self-host AME/i }),
    ).toHaveAttribute("href", "/self-hosting");
    await expect(
      page.getByRole("link", { name: "Self-hosting", exact: true }),
    ).toHaveAttribute("href", "/self-hosting");
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
