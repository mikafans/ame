/**
 * UI/UX spec alignment — the first-user, agent-friendly learning journey.
 *
 * This contract follows the product's primary self-host story: a visitor
 * states an intent, creates a local learner account, completes the first
 * starter check, and receives a grounded next recommendation.
 */
import { expect, test } from "@playwright/test";

test("learner can turn an intent into an evidence-backed next step", async ({
  page,
}) => {
  const prompt = "I would like to learn a new subject";
  const email = `subject-uiux-${Date.now()}@example.com`;

  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Study what you don't know yet." }),
  ).toBeVisible();

  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByRole("button", { name: "See my plan" }).click();
  await expect(page.getByText("Build a durable foundation")).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Start this journey" }),
  ).toBeVisible();

  await page.getByRole("link", { name: "Start this journey" }).click();
  await expect(page).toHaveURL(/\/start\?prompt=/);
  await expect(
    page.getByRole("heading", {
      name: "Turn your intent into a first useful session.",
    }),
  ).toBeVisible();

  await page.getByLabel("Your name").fill("Subject Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill("subject-local-2026");
  await page.getByRole("button", { name: "Create my journey" }).click();

  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);
  await expect(page.getByText(prompt, { exact: true })).toBeVisible();
  await expect(page.getByText("Your first activity is ready")).toBeVisible();
  await expect(page.getByRole("button", { name: "Begin" })).toBeVisible();

  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByText(/How familiar are you with .+\?/)).toBeVisible();
  await page.getByRole("button", { name: "new to me" }).click();
  await page
    .getByLabel(/What would you like to .+\?/)
    .fill("Understand the first useful idea and apply it");
  await page.getByRole("button", { name: "Mark activity complete" }).click();

  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Try a short first task" }).first(),
  ).toBeVisible();
  await expect(
    page.getByText("Your latest activity produced evidence."),
  ).toBeVisible();
});

test("a returning learner resumes from the learning desk", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill("haru@example.com");
  await page.getByLabel("Password").fill("password123");
  await page.getByRole("button", { name: "Sign in" }).click();

  await expect(page).toHaveURL(/\/learning$/);
  await expect(
    page.getByRole("heading", { name: "Continue with the next useful thing." }),
  ).toBeVisible();
  await expect(
    page.getByText("I would like to learn a new subject", { exact: true }),
  ).toBeVisible();
});
