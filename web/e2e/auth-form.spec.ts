import { expect, test } from "@playwright/test";

test("login form exposes password-manager autofill semantics", async ({
  page,
}) => {
  await page.goto("/login");

  await expect(page.locator("#auth-form")).toHaveAttribute("method", "post");
  await expect(page.locator("#auth-form")).toHaveAttribute(
    "autocomplete",
    "on",
  );
  await expect(page.locator("#email")).toHaveAttribute(
    "autocomplete",
    "username",
  );
  await expect(page.locator("#password")).toHaveAttribute(
    "autocomplete",
    "current-password",
  );
  await expect(page.locator("label[for=email]")).toBeVisible();
  await expect(page.locator("label[for=password]")).toBeVisible();
});
