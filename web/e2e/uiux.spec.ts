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
  const prompt = "I would like to learn Flink and the Flink Operator";
  const email = `flink-uiux-${Date.now()}@example.com`;

  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Study what you don't know yet." }),
  ).toBeVisible();

  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByRole("button", { name: "See my plan" }).click();
  await expect(page.getByText("Build a durable foundation")).toBeVisible();
  await expect(
    page.getByText("Get oriented and see what you already know"),
  ).toBeVisible();

  await page.getByRole("link", { name: "Start this journey" }).click();
  await expect(page).toHaveURL(/\/start\?prompt=/);
  await expect(
    page.getByRole("heading", {
      name: "Turn your intent into a first useful session.",
    }),
  ).toBeVisible();

  await page.getByLabel("Your name").fill("Flink Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByRole("button", { name: "Create my journey" }).click();

  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);
  await expect(page.getByText(prompt, { exact: true })).toBeVisible();
  await expect(page.getByText("Your first activity is ready")).toBeVisible();
  await expect(page.getByRole("button", { name: "Begin" })).toBeVisible();

  await page.getByRole("button", { name: "Begin" }).click();
  await expect(
    page.getByText("How familiar are you with this topic?"),
  ).toBeVisible();
  await page.getByRole("button", { name: "new to me" }).click();
  await page
    .getByLabel("What would you like to be able to do first?")
    .fill("Deploy a simple FlinkDeployment and understand its lifecycle");
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
