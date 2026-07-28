/**
 * Golden field-journey acceptance — issue #36 P2.
 *
 * The audit behind #36 found the golden Flink (CS) and physics journeys were
 * never seeded and asserted end to end on a clean stack — milestones were
 * judged done by plumbing existing, not by these journeys actually working.
 * This spec seeds each field deterministically (onboarding alias match) and
 * proves field-specialized grading works on top of it: numeric-tolerance
 * grading for physics (P3), and the explicit manual-review routing for code
 * questions (P5 is not built yet, so this pins today's real, correct
 * behavior rather than a stub).
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

async function supportedCitation(page: Page, content: string, quote: string) {
  const source = await page.request.post(apiUrl("/api/v1/sources/imports"), {
    data: {
      kind: "document",
      locator: `golden/${Date.now()}-${Math.random()}.txt`,
      mediaType: "text/plain",
      content,
      retryKey: `golden-source-${Date.now()}-${Math.random()}`,
    },
  });
  expect(source.ok()).toBeTruthy();
  const snapshot = await source.json();
  const startByte = new TextEncoder().encode(
    content.slice(0, content.indexOf(quote)),
  ).length;
  const endByte = startByte + new TextEncoder().encode(quote).length;
  const certificate = await page.request.post(apiUrl("/api/v1/citations"), {
    data: {
      snapshotId: snapshot.id,
      startByte,
      endByte,
      quote,
      extractionMethod: "exact_quote",
      groundingStatus: "supported",
      groundingNote: "Seeded golden source directly supports the tested item.",
      licenseStatus: "allowed",
      licenseName: "Test fixture",
    },
  });
  expect(certificate.ok()).toBeTruthy();
  return (await certificate.json()).id as string;
}

async function onboardIntoJourney(
  page: Page,
  prompt: string,
  name: string,
  email: string,
  password: string,
) {
  await page.goto("/start");
  await page.getByLabel("What would you like to learn?").fill(prompt);
  await page.getByLabel("Your name").fill(name);
  await page.getByLabel("Email identifier").fill(email);
  await page.getByLabel("Password").fill(password);
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
  return { journeyId: journeyId as string, journey };
}

test("learner writes, resumes, edits, and deletes a version-anchored private note", async ({
  page,
}) => {
  await onboardIntoJourney(
    page,
    "I would like to learn physics",
    "Note Learner",
    `note-golden-${Date.now()}@example.com`,
    "note-golden-2026",
  );
  const editor = page.getByLabel("Private note");
  await expect(editor).toBeVisible();
  await editor.fill("Velocity is a vector.");
  await page.getByRole("button", { name: "Add note" }).click();
  await expect(page.getByText("Velocity is a vector.")).toBeVisible();

  await page.reload();
  await expect(page.getByText("Velocity is a vector.")).toBeVisible();
  await page.getByRole("button", { name: "Edit" }).click();
  await editor.fill("Velocity includes direction.");
  await page.getByRole("button", { name: "Save edit" }).click();
  await expect(page.getByText("Velocity includes direction.")).toBeVisible();
  await page.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByText("Velocity includes direction.")).toHaveCount(0);
});

test("physics golden journey seeds deterministically and grades numeric answers by tolerance", async ({
  page,
}) => {
  const email = `physics-golden-${Date.now()}@example.com`;
  const { journey } = await onboardIntoJourney(
    page,
    "I would like to learn physics",
    "Physics Learner",
    email,
    "physics-golden-2026",
  );

  expect(
    journey.chapters.map((chapter: { title: string }) => chapter.title),
  ).toEqual([
    "Kinematics",
    "Forces and Newton's laws",
    "Energy and work",
    "Momentum and collisions",
  ]);

  const objective = journey.objectives[0];
  const activity = journey.activities[0];
  const generationRunId = await publishedGenerationRun(
    page,
    "question.compose",
  );
  const kinematicsCitation = await supportedCitation(
    page,
    "For constant acceleration, velocity equals acceleration multiplied by elapsed time.",
    "velocity equals acceleration multiplied by elapsed time",
  );
  const forceCitation = await supportedCitation(
    page,
    "Net force equals mass multiplied by acceleration.",
    "Net force equals mass multiplied by acceleration",
  );

  const withinToleranceQuestion = await page.request.post(
    apiUrl("/api/v1/questions"),
    {
      data: {
        generationRunId,
        kind: "numeric",
        prompt:
          "A ball falls from rest for 2 s under gravity g = 9.8 m/s^2. What is its velocity in m/s (v = g * t)?",
        acceptedAnswers: ["19.6", "0.5"],
        points: 1,
        reviewStatus: "approved",
        sourceReferences: [kinematicsCitation],
      },
    },
  );
  expect(withinToleranceQuestion.ok()).toBeTruthy();
  const inTolerance = await withinToleranceQuestion.json();

  const outsideToleranceQuestion = await page.request.post(
    apiUrl("/api/v1/questions"),
    {
      data: {
        generationRunId,
        kind: "numeric",
        prompt:
          "What is the net force in newtons on a 100 kg mass (unrelated to the answer given)?",
        acceptedAnswers: ["100", "1"],
        points: 1,
        reviewStatus: "approved",
        sourceReferences: [forceCitation],
      },
    },
  );
  expect(outsideToleranceQuestion.ok()).toBeTruthy();
  const outOfTolerance = await outsideToleranceQuestion.json();

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
            questionId: inTolerance.questionId,
            questionVersion: inTolerance.version,
            orderIndex: 0,
            points: 1,
          },
          {
            objectiveId: objective.id,
            questionId: outOfTolerance.questionId,
            questionVersion: outOfTolerance.version,
            orderIndex: 1,
            points: 1,
          },
        ],
      },
    },
  );
  expect(assessmentResponse.ok()).toBeTruthy();
  const assessment = await assessmentResponse.json();

  const sessionResponse = await page.request.post(
    apiUrl(
      `/api/v1/learning/journeys/${journey.id}/activities/${activity.id}/start`,
    ),
  );
  expect(sessionResponse.ok()).toBeTruthy();
  const session = await sessionResponse.json();

  const attemptResponse = await page.request.post(
    apiUrl(`/api/v1/assessments/${assessment.id}/attempts`),
    { data: { learningSessionId: session.id } },
  );
  expect(attemptResponse.ok()).toBeTruthy();
  const attempt = await attemptResponse.json();

  const [inToleranceItem, outOfToleranceItem] = assessment.items;
  const inToleranceAnswer = await page.request.post(
    apiUrl(`/api/v1/attempts/${attempt.id}/answers`),
    {
      data: {
        assessmentItemId: inToleranceItem.id,
        questionVersionId: inToleranceItem.questionVersionId,
        response: { value: 19.8 },
      },
    },
  );
  expect(inToleranceAnswer.ok()).toBeTruthy();

  const outOfToleranceAnswer = await page.request.post(
    apiUrl(`/api/v1/attempts/${attempt.id}/answers`),
    {
      data: {
        assessmentItemId: outOfToleranceItem.id,
        questionVersionId: outOfToleranceItem.questionVersionId,
        response: { value: 5 },
      },
    },
  );
  expect(outOfToleranceAnswer.ok()).toBeTruthy();

  const finishResponse = await page.request.post(
    apiUrl(`/api/v1/attempts/${attempt.id}/finish`),
  );
  expect(finishResponse.ok()).toBeTruthy();
  const finished = await finishResponse.json();

  expect(finished.status).toBe("graded");
  const gradedInTolerance = finished.items.find(
    (item: { assessmentItemId: string }) =>
      item.assessmentItemId === inToleranceItem.id,
  );
  const gradedOutOfTolerance = finished.items.find(
    (item: { assessmentItemId: string }) =>
      item.assessmentItemId === outOfToleranceItem.id,
  );
  expect(gradedInTolerance.evaluationStatus).toBe("correct");
  expect(gradedOutOfTolerance.evaluationStatus).toBe("incorrect");
  expect(finished.score).toBeCloseTo(0.5, 5);
});

test("flink golden journey seeds deterministically and routes a code submission to manual review", async ({
  page,
}) => {
  const email = `flink-golden-${Date.now()}@example.com`;
  const { journey } = await onboardIntoJourney(
    page,
    "I would like to learn flink",
    "Flink Learner",
    email,
    "flink-golden-2026",
  );

  expect(
    journey.chapters.map((chapter: { title: string }) => chapter.title),
  ).toEqual([
    "Streams and event time",
    "State and fault tolerance",
    "Windows and watermarks",
    "Flink on Kubernetes",
  ]);

  const objective = journey.objectives[0];
  const activity = journey.activities[0];
  const generationRunId = await publishedGenerationRun(
    page,
    "question.compose",
  );
  const codeCitation = await supportedCitation(
    page,
    "A Python function can return the sum of its two integer parameters.",
    "return the sum of its two integer parameters",
  );

  const codeQuestion = await page.request.post(apiUrl("/api/v1/questions"), {
    data: {
      generationRunId,
      kind: "code",
      prompt:
        "Write a Python function `add(a, b)` that returns the sum of two integers.",
      points: 1,
      reviewStatus: "approved",
      sourceReferences: [codeCitation],
    },
  });
  expect(codeQuestion.ok()).toBeTruthy();
  const question = await codeQuestion.json();

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

  const sessionResponse = await page.request.post(
    apiUrl(
      `/api/v1/learning/journeys/${journey.id}/activities/${activity.id}/start`,
    ),
  );
  expect(sessionResponse.ok()).toBeTruthy();
  const session = await sessionResponse.json();

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
        response: { code: "def add(a, b):\n    return a + b" },
      },
    },
  );
  expect(answerResponse.ok()).toBeTruthy();

  const finishResponse = await page.request.post(
    apiUrl(`/api/v1/attempts/${attempt.id}/finish`),
  );
  expect(finishResponse.ok()).toBeTruthy();
  const finished = await finishResponse.json();

  expect(finished.status).toBe("submitted");
  expect(finished.reviewStatus).toBe("pending");
  expect(finished.score).toBeNull();
  expect(finished.items[0].evaluationStatus).toBe("manual_review");
});
