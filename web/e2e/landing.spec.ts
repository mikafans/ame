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

test("visitor can preview a first learning journey from an intent", async ({
  page,
}) => {
  await page.route("**/public/v1/onboarding/preview", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        normalizedStatement: "Learn distributed systems",
        promise:
          "Build a durable foundation through guided practice and review",
        templateId: "understand-a-subject",
        templateVersion: 1,
        objectives: [
          {
            verb: "identify",
            statement: "Identify the core concepts: distributed systems",
            successCriteria: "Name the essential ideas and how they relate",
          },
        ],
        firstActivity: {
          kind: "explanation",
          title: "Get oriented and see what you already know",
          purpose: "orientation",
          estimatedMinutes: 5,
        },
      }),
    });
  });

  await page.goto("/");
  await page
    .getByLabel("What would you like to learn?")
    .fill("I would like to learn distributed systems");
  await page.getByRole("button", { name: "See my plan" }).click();

  await expect(page.getByText("Build a durable foundation")).toBeVisible();
  await expect(
    page.getByText("Get oriented and see what you already know"),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Start this journey" }),
  ).toHaveAttribute("href", /\/start\?prompt=/);
});

test("saved color mode is applied before the first page render", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("ame.colorMode", "dark");
  });
  await page.goto("/");

  await expect(page.locator("html")).toHaveClass(/dark/);
  await expect(
    page.getByRole("button", { name: "Use light mode" }),
  ).toBeVisible();
});
