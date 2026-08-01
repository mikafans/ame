import { expect, test } from "@playwright/test";

test("landing page sends a new learner to public agent setup, not signup", async ({
  page,
}) => {
  await page.goto("/");

  await expect(
    page.getByRole("heading", {
      name: "Give your agent a goal. Meet it in a course.",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Set up your agent" }),
  ).toHaveAttribute("href", "/agent");
  await expect(page.getByRole("link", { name: "Sign up free" })).toHaveCount(0);
});

test("saved color mode is applied before the first page render", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("ame.theme", "night-study");
  });
  await page.goto("/");

  await expect(page.locator("html")).toHaveAttribute(
    "data-ame-theme",
    "night-study",
  );
  await expect(
    page.getByRole("button", { name: "Use light mode" }),
  ).toBeVisible();
});
