import { expect, test } from "@playwright/test";

const email = process.env.E2E_REFERENCE_EMAIL;
const password = process.env.E2E_REFERENCE_PASSWORD;

test("learner separates active work, completed courses, progress, and sources", async ({
  page,
}) => {
  if (!email || !password) {
    throw new Error(
      "local learner workspace smoke requires fixture credentials",
    );
  }

  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Sign in" }).click();

  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);
  await expect(
    page.getByRole("heading", {
      name: "Asynchronous TCP service design with Netty",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "Event loops and channel ownership: key idea",
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByText("Your path")).not.toBeVisible();

  await page.getByRole("link", { name: "Course", exact: true }).click();
  await expect(page.getByTestId("learner-course-tab")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Modules" })).toBeVisible();

  await page.getByRole("link", { name: "Progress", exact: true }).click();
  await expect(page.getByTestId("learner-progress-tab")).toBeVisible();
  await expect(page.getByText("Not assessed yet").first()).toBeVisible();

  await page.getByRole("link", { name: "Sources", exact: true }).click();
  await expect(page.getByTestId("learner-resources-tab")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "course-notes.txt", exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/netty-tcp-service/)).toBeVisible();
  await expect(page.getByText(/flink-clickstream/)).toHaveCount(0);

  await page.getByRole("link", { name: "Courses", exact: true }).click();
  const library = page.getByTestId("course-library");
  await expect(library).toBeVisible();
  await expect(library.getByRole("heading", { name: "Active" })).toBeVisible();
  await expect(
    library.getByRole("heading", { name: "Completed" }),
  ).toBeVisible();
  await expect(
    library.getByRole("heading", {
      name: "Asynchronous TCP service design with Netty",
    }),
  ).toBeVisible();
  await expect(
    library.getByRole("heading", {
      name: "Reliable clickstream aggregation with Apache Flink",
    }),
  ).toBeVisible();
});
