import { expect, test } from "@playwright/test";
import { loginAs, setAuthCookie } from "./helpers";

test.describe("brand v0 smoke", () => {
  let token: string;

  test.beforeAll(async ({ request }) => {
    token = await loginAs(request, "ada@example.com");
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("renders brand palette, favicon links, and theme-aware mascot", async ({
    page,
  }) => {
    await page.goto("/explore");
    await expect(page).toHaveURL(/\/explore$/);
    const primaryButton = page.getByRole("button", { name: /^Start$/ }).first();
    await expect(primaryButton).toBeVisible({ timeout: 10000 });

    await expect(page.getByTestId("brand-wordmark").first()).toHaveText("ame");

    await expect(
      page.locator('link[rel="icon"][href="/ame-icon-light.svg"]'),
    ).toHaveAttribute("media", "(prefers-color-scheme: light)");
    await expect(
      page.locator('link[rel="icon"][href="/ame-icon-dark.svg"]'),
    ).toHaveAttribute("media", "(prefers-color-scheme: dark)");
    await expect(
      page.locator('link[rel="icon"][href="/ame-icon-16.svg"]'),
    ).toHaveAttribute("sizes", "16x16");

    const mascot = page.getByTestId("brand-mascot").first();
    await expect(mascot).toHaveAttribute("src", /ame-icon-light\.svg$/);

    const wordmarkGradient = await page
      .getByTestId("brand-wordmark")
      .first()
      .evaluate((node) => getComputedStyle(node).backgroundImage);
    expect(wordmarkGradient).toContain("rgb(69, 196, 185)");
    expect(wordmarkGradient).toContain("rgb(255, 143, 180)");

    const primaryBackground = await primaryButton.evaluate(
      (node) => getComputedStyle(node).backgroundImage,
    );
    expect(primaryBackground).toContain("rgb(47, 167, 158)");
    expect(primaryBackground).toContain("rgb(69, 196, 185)");

    await page.getByRole("button", { name: "Dark mode" }).click();
    await expect(mascot).toHaveAttribute("src", /ame-icon-dark\.svg$/);
  });

  test("freezes mascot animation when reduced motion is requested", async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/ame-icon-light.svg");
    const animatedElementIds = await page.evaluate(() =>
      ["tailL", "tailR", "fig", "eyes"].filter((id) => {
        const el = document.getElementById(id);
        return el && el.getAnimations().length > 0;
      }),
    );

    expect(animatedElementIds).toEqual([]);
  });
});
