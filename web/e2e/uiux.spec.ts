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
    page.getByRole("heading", { name: "Your learning, with a next move." }),
  ).toBeVisible();
  await expect(
    page.getByText("I would like to learn a new subject", { exact: true }),
  ).toBeVisible();
});

test("learner can complete an agent-provided assessment and open its deep dive", async ({
  page,
}) => {
  const prompt = "I would like to learn distributed systems";
  const email = `assessment-uiux-${Date.now()}@example.com`;

  await page.goto("/start");
  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByLabel("Your name").fill("Assessment Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill("assessment-local-2026");
  await page.getByRole("button", { name: "Create my journey" }).click();

  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);
  const journeyId = page
    .url()
    .match(/\/learning\/journeys\/([0-9a-f-]+)$/)?.[1];
  expect(journeyId).toBeTruthy();

  const journeyResponse = await page.request.get(
    `/api/v1/learning/journeys/${journeyId}`,
  );
  expect(journeyResponse.ok()).toBeTruthy();
  const journey = await journeyResponse.json();
  const activity = journey.activities[0];
  const objective = journey.objectives[0];

  const questionResponse = await page.request.post("/api/v1/questions", {
    data: {
      kind: "multiple_choice",
      prompt: "Which layer coordinates distributed work?",
      options: [
        { id: "wrong", text: "A local text editor", is_correct: false },
        { id: "right", text: "A coordinator", is_correct: true },
      ],
      points: 1,
      reviewStatus: "approved",
      sourceReferences: ["https://example.com/distributed-systems"],
    },
  });
  expect(questionResponse.ok()).toBeTruthy();
  const question = await questionResponse.json();

  const assessmentResponse = await page.request.post("/api/v1/assessments", {
    data: {
      activityId: activity.id,
      mode: "practice",
      status: "published",
      items: [
        {
          objectiveId: objective.id,
          questionId: question.questionId,
          questionVersion: question.version,
          orderIndex: 0,
          points: 1,
        },
      ],
    },
  });
  expect(assessmentResponse.ok()).toBeTruthy();

  const evidenceResponse = await page.request.post(
    "/api/v1/progress/evidence",
    {
      data: {
        journeyId,
        objectiveId: objective.id,
        activityId: activity.id,
        value: 0.5,
        derivationVersion: 1,
      },
    },
  );
  expect(evidenceResponse.ok()).toBeTruthy();
  const evidence = await evidenceResponse.json();

  const deepDiveResponse = await page.request.post("/api/v1/deep-dives", {
    data: {
      journeyId,
      activityId: activity.id,
      objectiveId: objective.id,
      triggeringEvidenceId: evidence.id,
      title: "A coordinator gives the system a shared decision point",
      body: "A coordinator tracks shared work and helps workers agree on progress.",
      example: "A scheduler assigns work while workers execute it.",
      caveats: ["The exact responsibilities vary by system design."],
      sourceReferences: ["https://example.com/distributed-systems"],
      applicationTask:
        "Describe which component coordinates your next example.",
      reviewStatus: "approved",
    },
  });
  expect(deepDiveResponse.ok()).toBeTruthy();

  await page.getByRole("button", { name: "Begin" }).click();
  await expect(
    page.getByText("Practice assessment", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Which layer coordinates distributed work?"),
  ).toBeVisible();
  await page.getByRole("button", { name: "Submit assessment" }).click();
  await expect(
    page.getByText("Answer every assessment question before submitting", {
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "A coordinator" }).click();
  await page.getByRole("button", { name: "Submit assessment" }).click();

  await expect(page.getByText("Assessment complete")).toBeVisible();
  await expect(
    page.getByText("correct", { exact: true }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "A coordinator gives the system a shared decision point",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Source-backed explanation", { exact: true }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", {
      name: "A coordinator gives the system a shared decision point",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("Assessment history", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Your saved attempts", { exact: true }),
  ).toBeVisible();

  const refreshedJourneyResponse = await page.request.get(
    `/api/v1/learning/journeys/${journeyId}`,
  );
  expect(refreshedJourneyResponse.ok()).toBeTruthy();
  const refreshedJourney = await refreshedJourneyResponse.json();
  const nextActivity = refreshedJourney.activities.find(
    (candidate: { status: string }) => candidate.status === "ready",
  );
  expect(nextActivity).toBeTruthy();
  const gradedAssessmentResponse = await page.request.post(
    "/api/v1/assessments",
    {
      data: {
        activityId: nextActivity.id,
        mode: "graded",
        status: "published",
        items: [
          {
            objectiveId: objective.id,
            questionId: question.questionId,
            questionVersion: question.version,
            orderIndex: 0,
            points: 1,
          },
        ],
      },
    },
  );
  expect(gradedAssessmentResponse.ok()).toBeTruthy();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(
    page.getByText("Graded assessment", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "A coordinator" }).click();
  await page.getByRole("button", { name: "Submit assessment" }).click();
  await expect(page.getByText("Assessment complete")).toBeVisible();
});
