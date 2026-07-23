/**
 * UI/UX spec alignment — the first-user, agent-friendly learning journey.
 *
 * This contract follows the product's primary self-host story: a visitor
 * states an intent, creates a local learner account, completes the first
 * starter check, and receives a grounded next recommendation.
 */
import { expect, test, type Page } from "@playwright/test";

function apiUrl(path: string) {
  return `${process.env.E2E_API_URL ?? "http://localhost:28080"}${path}`;
}

async function publishedGenerationRun(page: Page, operation: string) {
  const response = await page.request.post(apiUrl("/api/v1/generation-runs"), {
    data: {
      operation,
      provider: "test-provider",
      retryKey: `${operation}-${Date.now()}-${Math.random()}`,
      contentVersion: 1,
    },
  });
  expect(response.ok()).toBeTruthy();
  const run = await response.json();
  for (const status of ["running", "published"]) {
    const transition = await page.request.patch(
      apiUrl(`/api/v1/generation-runs/${run.id}`),
      { data: { status } },
    );
    expect(transition.ok()).toBeTruthy();
  }
  return run.id as string;
}

test("learner can turn an intent into an evidence-backed next step", async ({
  page,
}) => {
  const prompt = "I would like to learn a new subject";
  const topic = "a new subject";
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
  const journeyId = page
    .url()
    .match(/\/learning\/journeys\/([0-9a-f-]+)$/)?.[1];
  expect(journeyId).toBeTruthy();
  await expect(page.getByText(prompt, { exact: true })).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Foundations", exact: true }),
  ).toBeVisible();
  const currentChapter = page.locator('[data-current="true"]');
  await expect(currentChapter).toContainText("Current");
  const currentChapterToggle = currentChapter.locator(
    'button[aria-controls^="chapter-content-"]',
  );
  await expect(currentChapterToggle).toHaveAttribute("aria-expanded", "true");
  await currentChapterToggle.click();
  await expect(currentChapterToggle).toHaveAttribute("aria-expanded", "false");
  await currentChapterToggle.click();
  await expect(currentChapterToggle).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByText("Your first activity is ready")).toBeVisible();
  await expect(page.getByRole("button", { name: "Begin" })).toBeVisible();
  await expect(page.getByTestId("journey-detail-progress")).toContainText(
    /0\/\d+ complete · 0%/,
  );
  await expect(
    page.getByRole("button", { name: "Continue learning" }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Begin" }).click();
  await expect(
    page.getByRole("heading", { name: `Get oriented on ${topic}` }).first(),
  ).toBeVisible();
  await expect(page.getByText(/How familiar are you with .+\?/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Resume", exact: true }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: `Get oriented on ${topic}` }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Resume", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "new to me" }).click();
  await page
    .getByLabel(/What would you like to .+\?/)
    .fill("Understand the first useful idea and apply it");
  await page.getByRole("button", { name: "Mark activity complete" }).click();

  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();
  await expect(page.getByTestId("journey-detail-progress")).toContainText(
    /1\/\d+ complete · \d+%/,
  );
  await expect(
    page
      .getByRole("heading", {
        name: `Build a clear starting model for ${topic}`,
      })
      .first(),
  ).toBeVisible();
  const refreshedAfterStarter = await page.request.get(
    apiUrl(`/api/v1/learning/journeys/${journeyId}`),
  );
  expect(refreshedAfterStarter.ok()).toBeTruthy();
  const packageAfterStarter = await refreshedAfterStarter.json();
  const explanation = packageAfterStarter.activities.find(
    (activity: { payload?: { content?: { type?: string } } }) =>
      activity.payload?.content?.type === "explanation",
  );
  const workedExample = packageAfterStarter.activities.find(
    (activity: { payload?: { content?: { type?: string } } }) =>
      activity.payload?.content?.type === "worked_example",
  );
  expect(explanation).toBeTruthy();
  expect(workedExample).toBeTruthy();
  if (!explanation || !workedExample)
    throw new Error("first package is incomplete");
  expect(explanation.payload.content.type).toBe("explanation");
  expect(explanation.payload.content.body).toContain(topic);
  expect(workedExample.payload.content.type).toBe("worked_example");
  const contentGenerationRunId = await publishedGenerationRun(
    page,
    "learning.activity.content.compose",
  );
  const kindMismatchResponse = await page.request.patch(
    apiUrl(`/api/v1/learning/activities/${explanation.id}/content`),
    {
      data: {
        generationRunId: contentGenerationRunId,
        content: {
          type: "worked_example",
          heading: "Wrong activity kind",
          prompt: "This must be rejected.",
          steps: ["Do not store this."],
          reflection: "Try the matching activity instead.",
        },
        sourceReferences: ["https://example.test/subject/intro"],
        reviewStatus: "approved",
      },
    },
  );
  expect(kindMismatchResponse.status()).toBe(422);
  const groundedContent = {
    type: "explanation",
    heading: "A grounded starting model",
    body: "This is the first source-backed model for the learner.",
    key_points: ["Start with one observable mechanism."],
  };
  const groundedContentResponse = await page.request.patch(
    apiUrl(`/api/v1/learning/activities/${explanation.id}/content`),
    {
      data: {
        generationRunId: contentGenerationRunId,
        content: groundedContent,
        sourceReferences: ["https://example.test/subject/intro"],
        reviewStatus: "approved",
      },
    },
  );
  expect(groundedContentResponse.ok()).toBeTruthy();
  await page.reload();
  await page.getByRole("button", { name: "Begin" }).last().click();
  await expect(
    page.getByRole("heading", { name: groundedContent.heading }),
  ).toBeVisible();
  await expect(page.getByText(groundedContent.body)).toBeVisible();
  await expect(
    page.getByText("This objective has the weakest available evidence."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Mark activity complete" }).click();
  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();
  const workedExampleContent = {
    type: "worked_example",
    heading: "A grounded worked example",
    prompt: "Apply one idea to one observable situation.",
    steps: ["Name the situation.", "Apply the idea.", "Inspect the result."],
    reflection: "What would you try next?",
  };
  const workedExampleResponse = await page.request.patch(
    apiUrl(`/api/v1/learning/activities/${workedExample.id}/content`),
    {
      data: {
        generationRunId: contentGenerationRunId,
        content: workedExampleContent,
        sourceReferences: ["https://example.test/subject/example"],
        reviewStatus: "approved",
      },
    },
  );
  expect(workedExampleResponse.ok()).toBeTruthy();
  await page.reload();
  await page.getByRole("button", { name: "Begin" }).last().click();
  await expect(
    page.getByRole("heading", { name: workedExampleContent.heading }),
  ).toBeVisible();
  await expect(page.getByText(workedExampleContent.prompt)).toBeVisible();
  await expect(page.getByText("Source-backed activity")).toBeVisible();
  await page.goto("/learning");
  await expect(page.getByText(/Recent:/)).toBeVisible();
  const journeyDetailResponse = await page.request.get(
    apiUrl(`/api/v1/learning/journeys/${journeyId}`),
  );
  expect(journeyDetailResponse.ok()).toBeTruthy();
  const journeyDetail = await journeyDetailResponse.json();
  await expect(
    page.getByRole("heading", { name: "Objective progress" }),
  ).toBeVisible();
  for (const objective of journeyDetail.objectives) {
    await expect(
      page.getByTestId(`objective-progress-${objective.id}`),
    ).toContainText(objective.statement);
  }
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
  await expect(page.getByText("Course path", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Course progress", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("progressbar", { name: "Course progress" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Continue learning" }),
  ).toHaveAttribute("href", /#activity-[0-9a-f-]+$/);
  await expect(page.getByText("1. Foundations", { exact: true })).toBeVisible();
  const firstDeskActivity = page
    .locator('[data-testid^="desk-activity-"]')
    .first();
  await expect(firstDeskActivity).toBeVisible();
  await firstDeskActivity.click();
  await expect(page).toHaveURL(
    /\/learning\/journeys\/[0-9a-f-]+#activity-[0-9a-f-]+$/,
  );
  await expect(
    page.getByRole("heading", { name: "Foundations", exact: true }),
  ).toBeVisible();
});

test("learner sees a completed chapter state", async ({ page }) => {
  const email = `completed-chapter-uiux-${Date.now()}@example.com`;
  await page.goto("/start");
  await page
    .getByLabel("What would you like to learn?")
    .fill("I would like to learn Flink checkpointing");
  await page.getByLabel("Your name").fill("Chapter Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill("chapter-local-2026");
  await page.getByRole("button", { name: "Create my journey" }).click();
  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);
  const journeyId = page
    .url()
    .match(/\/learning\/journeys\/([0-9a-f-]+)$/)?.[1];
  expect(journeyId).toBeTruthy();
  const apiBase = process.env.E2E_API_URL ?? "";

  for (let index = 0; index < 2; index += 1) {
    const journeyResponse = await page.request.get(
      `${apiBase}/api/v1/learning/journeys/${journeyId}`,
    );
    const currentJourney = await journeyResponse.json();
    const readyActivity = currentJourney.activities.find(
      (activity: { status: string }) => activity.status === "ready",
    );
    if (!readyActivity) break;
    const started = await page.request.post(
      `${apiBase}/api/v1/learning/journeys/${journeyId}/activities/${readyActivity.id}/start`,
    );
    expect(started.ok()).toBeTruthy();
    const session = await started.json();
    const finished = await page.request.post(
      `${apiBase}/api/v1/learning/sessions/${session.id}/finish`,
      { data: { completed: true, responses: [] } },
    );
    expect(finished.ok()).toBeTruthy();
  }

  await page.goto(`/learning/journeys/${journeyId}`);
  await expect(page.getByTestId("journey-detail-progress")).toContainText(
    "40%",
  );
  await expect(page.locator('[data-complete="true"]')).toHaveCount(1);
  await expect(
    page.locator('[data-complete="true"]').getByText("Complete", {
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByTestId("chapter-handoff")).toContainText(
    "Next up: Practice and application",
  );
});

test("learner can answer the first application task in a journey", async ({
  page,
}) => {
  const prompt = "I would like to learn Flink event-time processing";
  const email = `task-uiux-${Date.now()}@example.com`;

  await page.goto("/start");
  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByLabel("Your name").fill("Task Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill("task-local-2026");
  await page.getByRole("button", { name: "Create my journey" }).click();
  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);

  await page.getByRole("button", { name: "Begin" }).click();
  await page.getByRole("button", { name: "new to me" }).click();
  await page
    .getByLabel(/What would you like to .+\?/)
    .fill("Understand event time and watermarks");
  await page.getByRole("button", { name: "Mark activity complete" }).click();
  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();

  const explanationTitle =
    "Build a clear starting model for Flink event-time processing";
  await expect(
    page.getByRole("heading", { name: explanationTitle }).first(),
  ).toBeVisible();
  await page
    .getByRole("article")
    .filter({ hasText: explanationTitle })
    .getByRole("button", { name: "Begin" })
    .click();
  await expect(page.getByText("Explanation", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Mark activity complete" }).click();
  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();

  const exampleTitle =
    "Walk through a worked example for Flink event-time processing";
  await expect(
    page.getByRole("heading", { name: exampleTitle }).first(),
  ).toBeVisible();
  await page
    .getByRole("article")
    .filter({ hasText: exampleTitle })
    .getByRole("button", { name: "Begin" })
    .click();
  await expect(page.getByText("Worked example", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Mark activity complete" }).click();
  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();

  const taskTitle = "Try a short first task for Flink event-time processing";
  await expect(
    page.getByRole("heading", { name: taskTitle }).first(),
  ).toBeVisible();
  await page
    .getByRole("article")
    .filter({ hasText: taskTitle })
    .getByRole("button", { name: "Begin" })
    .click();
  await expect(page.getByText("Scenario", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Make the core idea explicit before acting" })
    .click();
  await page.getByRole("button", { name: "Submit task" }).click();
  await expect(
    page.getByText("Task submitted. Your answer is now part of this journey."),
  ).toBeVisible();
});

test("admin can review a pending learner task from the console", async ({
  page,
}) => {
  const prompt = "I would like to learn Flink checkpointing";
  const email = `review-queue-uiux-${Date.now()}@example.com`;

  await page.goto("/start");
  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByLabel("Your name").fill("Queue Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill("queue-local-2026");
  await page.getByRole("button", { name: "Create my journey" }).click();
  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);
  const journeyId = page
    .url()
    .match(/\/learning\/journeys\/([0-9a-f-]+)$/)?.[1];
  expect(journeyId).toBeTruthy();
  const apiBase = process.env.E2E_API_URL ?? "";

  const journeyResponse = await page.request.get(
    `${apiBase}/api/v1/learning/journeys/${journeyId}`,
  );
  expect(journeyResponse.ok()).toBeTruthy();
  const journey = await journeyResponse.json();
  for (const activity of journey.activities.slice(0, 3)) {
    const started = await page.request.post(
      `${apiBase}/api/v1/learning/journeys/${journeyId}/activities/${activity.id}/start`,
    );
    expect(started.ok()).toBeTruthy();
    const session = await started.json();
    const finished = await page.request.post(
      `${apiBase}/api/v1/learning/sessions/${session.id}/finish`,
      { data: { completed: true, responses: [] } },
    );
    expect(finished.ok()).toBeTruthy();
  }

  const refreshed = await page.request.get(
    `${apiBase}/api/v1/learning/journeys/${journeyId}`,
  );
  const packageAfterProgress = await refreshed.json();
  const task = packageAfterProgress.activities.find(
    (activity: { kind: string; status: string }) =>
      activity.kind === "diagnostic" && activity.status === "ready",
  );
  expect(task).toBeTruthy();
  if (!task) throw new Error("pending review task was not ready");

  const startedTask = await page.request.post(
    `${apiBase}/api/v1/tasks/${task.id}/submissions`,
    {
      data: {
        contentVersion: task.contentVersion,
        response: { optionId: "make_model" },
        evaluationMethod: "agent",
      },
    },
  );
  expect(startedTask.ok()).toBeTruthy();
  const submission = await startedTask.json();
  const submittedTask = await page.request.post(
    `${apiBase}/api/v1/task-submissions/${submission.id}/submit`,
  );
  expect(submittedTask.ok()).toBeTruthy();

  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page).toHaveURL(/\/login$/);
  const adminLogin = await page.request.post(
    `${apiBase}/public/v1/auth/login`,
    { data: { email: "admin@example.com", password: "password123" } },
  );
  expect(adminLogin.ok()).toBeTruthy();
  await page.goto("/admin");
  await expect(page).toHaveURL(/\/admin$/);
  await page.goto("/admin/tasks");
  await expect(
    page.getByRole("heading", { name: "Task review queue" }),
  ).toBeVisible();
  await expect(page.getByText(submission.id, { exact: true })).toBeVisible();
  await page.getByLabel(`Score ${submission.id}`).fill("0.9");
  await page
    .getByLabel(`Feedback ${submission.id}`)
    .fill("Strong checkpointing reasoning.");
  await page
    .getByRole("article")
    .filter({ hasText: submission.id })
    .getByRole("button", { name: "Review task" })
    .click();
  await expect(page.getByText(submission.id, { exact: true })).toHaveCount(0);
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
    apiUrl(`/api/v1/learning/journeys/${journeyId}`),
  );
  expect(journeyResponse.ok()).toBeTruthy();
  const journey = await journeyResponse.json();
  const activity = journey.activities[0];
  const objective = journey.objectives[0];
  const sessionResponse = await page.request.post(
    apiUrl(
      `/api/v1/learning/journeys/${journeyId}/activities/${activity.id}/start`,
    ),
  );
  expect(sessionResponse.ok()).toBeTruthy();
  const session = await sessionResponse.json();
  const questionGenerationRunId = await publishedGenerationRun(
    page,
    "question.compose",
  );

  const questionResponse = await page.request.post(
    apiUrl("/api/v1/questions"),
    {
      data: {
        generationRunId: questionGenerationRunId,
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
    },
  );
  expect(questionResponse.ok()).toBeTruthy();
  const question = await questionResponse.json();

  const assessmentResponse = await page.request.post(
    apiUrl("/api/v1/assessments"),
    {
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
    },
  );
  expect(assessmentResponse.ok()).toBeTruthy();
  const assessment = await assessmentResponse.json();
  const attemptResponse = await page.request.post(
    apiUrl(`/api/v1/assessments/${assessment.id}/attempts`),
    { data: { learningSessionId: session.id } },
  );
  expect(attemptResponse.ok()).toBeTruthy();
  const attempt = await attemptResponse.json();
  const item = assessment.items[0];
  const answerResponse = await page.request.post(
    apiUrl(`/api/v1/attempts/${attempt.id}/answers`),
    {
      data: {
        assessmentItemId: item.id,
        questionVersionId: item.questionVersionId,
        response: { option_id: "right" },
      },
    },
  );
  expect(answerResponse.ok()).toBeTruthy();
  const finishResponse = await page.request.post(
    apiUrl(`/api/v1/attempts/${attempt.id}/finish`),
  );
  expect(finishResponse.ok()).toBeTruthy();

  const evidenceResponse = await page.request.post(
    apiUrl("/api/v1/progress/evidence"),
    {
      data: {
        journeyId,
        objectiveId: objective.id,
        activityId: activity.id,
        attemptId: attempt.id,
        contentVersion: 1,
        value: 0.5,
        derivationVersion: 1,
      },
    },
  );
  expect(evidenceResponse.ok()).toBeTruthy();
  const evidence = await evidenceResponse.json();
  const deepDiveGenerationRunId = await publishedGenerationRun(
    page,
    "deep_dive.create",
  );

  const deepDiveResponse = await page.request.post(
    apiUrl("/api/v1/deep-dives"),
    {
      data: {
        generationRunId: deepDiveGenerationRunId,
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
    },
  );
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

  await expect(page.getByText("Graded · 100%", { exact: true })).toBeVisible();
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
    apiUrl(`/api/v1/learning/journeys/${journeyId}`),
  );
  expect(refreshedJourneyResponse.ok()).toBeTruthy();
  const refreshedJourney = await refreshedJourneyResponse.json();
  const nextActivity = refreshedJourney.activities.find(
    (candidate: { status: string }) => candidate.status === "ready",
  );
  expect(nextActivity).toBeTruthy();
  const gradedAssessmentResponse = await page.request.post(
    apiUrl("/api/v1/assessments"),
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
  await expect(page.getByText("Graded · 100%", { exact: true })).toBeVisible();
});

test("learner can finish a manual-review assessment without false progress", async ({
  page,
}) => {
  const prompt = "I would like to understand a subject deeply";
  const email = `manual-review-uiux-${Date.now()}@example.com`;

  await page.goto("/start");
  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByLabel("Your name").fill("Manual Review Learner");
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill("manual-review-local-2026");
  await page.getByRole("button", { name: "Create my journey" }).click();
  await expect(page).toHaveURL(/\/learning\/journeys\/[0-9a-f-]+$/);

  const journeyId = page
    .url()
    .match(/\/learning\/journeys\/([0-9a-f-]+)$/)?.[1];
  expect(journeyId).toBeTruthy();
  const journeyResponse = await page.request.get(
    apiUrl(`/api/v1/learning/journeys/${journeyId}`),
  );
  expect(journeyResponse.ok()).toBeTruthy();
  const journey = await journeyResponse.json();
  const activity = journey.activities[0];
  const objective = journey.objectives[0];
  const generationRunId = await publishedGenerationRun(
    page,
    "question.compose",
  );
  const questionResponse = await page.request.post(
    apiUrl("/api/v1/questions"),
    {
      data: {
        generationRunId,
        kind: "essay",
        prompt: "Explain the first principle in your own words.",
        points: 2,
        reviewStatus: "approved",
        sourceReferences: ["https://example.com/learning"],
      },
    },
  );
  expect(questionResponse.ok()).toBeTruthy();
  const question = await questionResponse.json();
  const assessmentResponse = await page.request.post(
    apiUrl("/api/v1/assessments"),
    {
      data: {
        activityId: activity.id,
        mode: "graded",
        status: "published",
        items: [
          {
            objectiveId: objective.id,
            questionId: question.questionId,
            questionVersion: question.version,
            orderIndex: 0,
            points: 2,
          },
        ],
      },
    },
  );
  expect(assessmentResponse.ok()).toBeTruthy();

  await page.getByRole("button", { name: "Begin" }).click();
  await expect(
    page.getByText("Graded assessment", { exact: true }),
  ).toBeVisible();
  await page
    .getByLabel("Answer question 1")
    .fill("I can explain the principle and apply it to a new example.");
  await page.getByRole("button", { name: "Submit assessment" }).click();

  await expect(
    page.getByText("Submitted · pending review", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Complete. Your next activity is now ready."),
  ).toBeVisible();
  await expect(
    page.getByText("Could not record assessment evidence"),
  ).toHaveCount(0);
});
