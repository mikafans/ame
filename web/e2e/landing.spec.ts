import { expect, test } from "@playwright/test";

test("sign up CTA opens the account creation form", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Sign up free" }).first().click();

  await expect(page).toHaveURL(/\/login\?tab=signup$/);
  await expect(
    page.getByRole("tab", { name: "Create account" }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.getByLabel("Full name")).toBeVisible();
});
