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

async function openActivity(page: Page) {
  await page
    .getByRole("button", { name: /^(Begin|Resume)$/ })
    .last()
    .click();
}

async function onboardIntoJourney(
  page: Page,
  prompt: string,
  name: string,
  email: string,
  password: string,
  catalogId?: string,
) {
  await page.goto(catalogId ? `/start?catalogId=${catalogId}` : "/start");
  if (!catalogId) {
    await page.getByLabel("What would you like to learn?").fill(prompt);
  }
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

test("native catalog lists eight reviewed journeys and previews each deterministically", async ({
  page,
}) => {
  const catalogResponse = await page.request.get(
    apiUrl("/public/v1/catalog/journeys"),
  );
  expect(catalogResponse.ok()).toBeTruthy();
  const catalog = await catalogResponse.json();
  const ids = catalog.map((entry: { id: string }) => entry.id);
  expect(ids).toEqual([
    "learning-science-starter",
    "rust-ownership-starter",
    "python-foundations-starter",
    "sql-foundations-starter",
    "linear-algebra-starter",
    "technical-writing-starter",
    "flink-cs-starter",
    "physics-mechanics-starter",
  ]);
  expect(
    catalog.every(
      (entry: { reviewStatus: string; outcomes: string[] }) =>
        entry.reviewStatus === "reviewed" && entry.outcomes.length > 0,
    ),
  ).toBeTruthy();

  for (const id of ids) {
    const preview = await page.request.post(
      apiUrl("/public/v1/onboarding/preview"),
      { data: { prompt: "", catalogId: id } },
    );
    expect(preview.ok(), id).toBeTruthy();
    const body = await preview.json();
    expect(body.catalogId).toBe(id);
    expect(body.catalogVersion).toBe(1);
    expect(body.objectives.length).toBeGreaterThan(0);
  }
});

test("new learner discovers a native journey card from the learning desk", async ({
  page,
}) => {
  const email = `native-catalog-${Date.now()}@example.com`;
  const registration = await page.request.post(
    apiUrl("/public/v1/auth/register"),
    { data: { email, name: "Catalog Learner", password: "catalog-2026" } },
  );
  expect(registration.ok()).toBeTruthy();
  const { token } = await registration.json();
  await page.context().addCookies([
    {
      name: "ame_token",
      value: token,
      url: process.env.E2E_BASE_URL ?? "http://localhost:23000",
    },
  ]);

  await page.goto("/learning");
  const catalog = page.getByTestId("native-journey-catalog");
  await expect(catalog).toBeVisible();
  await expect(
    catalog.getByRole("heading", { name: "SQL foundations" }),
  ).toBeVisible();
  await expect(catalog.getByText("sql-foundations-starter")).toBeVisible();
  await catalog
    .locator('a[href="/start?catalogId=learning-science-starter"]')
    .click();
  await expect(page).toHaveURL(/\/start\?catalogId=learning-science-starter$/);
});

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
  await openActivity(page);
  const editor = page.getByLabel("Private note");
  await expect(editor).toBeVisible();
  await editor.fill("Velocity is a vector.");
  await page.getByRole("button", { name: "Add note" }).click();
  await expect(page.getByText("Velocity is a vector.")).toBeVisible();

  await page.reload();
  await openActivity(page);
  await expect(page.getByText("Velocity is a vector.")).toBeVisible();
  await page.getByRole("button", { name: "Edit" }).click();
  await editor.fill("Velocity includes direction.");
  await page.getByRole("button", { name: "Save edit" }).click();
  await expect(page.getByText("Velocity includes direction.")).toBeVisible();
  await page.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByText("Velocity includes direction.")).toHaveCount(0);
});

test("owner inspects an export and repeated import does not duplicate learner state", async ({
  page,
}) => {
  await onboardIntoJourney(
    page,
    "I would like to learn physics",
    "Portable Learner",
    `portable-golden-${Date.now()}@example.com`,
    "portable-golden-2026",
  );
  await page.goto("/learning");
  await page.getByRole("button", { name: "Export JSON" }).click();
  const artifact = page.getByLabel("Journey portability artifact");
  await expect(artifact).toBeVisible();
  await expect(artifact).toHaveValue(/ame\.journey-history\.v1/);
  await expect(artifact).toHaveValue(/"checksum"/);
  await page.getByRole("button", { name: "Import JSON" }).click();
  await expect(
    page.getByText("Already imported; no learner state was duplicated."),
  ).toBeVisible();
});

test("learner sees variant failure, retries, and uses reviewed source-backed content without mastery", async ({
  page,
}) => {
  const { journey } = await onboardIntoJourney(
    page,
    "I would like to learn physics",
    "Variant Learner",
    `variant-golden-${Date.now()}@example.com`,
    "variant-golden-2026",
  );
  const activity = journey.activities[0];
  const objective = journey.objectives[0];
  const masteryBefore = await page.request.get(
    apiUrl(`/api/v1/progress/${journey.id}/objectives/${objective.id}`),
  );
  const failedRequest = await page.request.post(
    apiUrl("/api/v1/learning-variants"),
    {
      data: {
        sourceActivityId: activity.id,
        objectiveId: objective.id,
        variantKind: "explanation",
        recommendationReason: "I need another mental model.",
        retryKey: `failed-${Date.now()}`,
      },
    },
  );
  expect(failedRequest.ok()).toBeTruthy();
  const failed = await failedRequest.json();
  for (const [status, error] of [
    ["running", undefined],
    ["failed", { code: "provider_unavailable" }],
  ] as const) {
    const transition = await page.request.patch(
      apiUrl(`/api/v1/generation-runs/${failed.generationRunId}`),
      { data: { status, error } },
    );
    expect(transition.ok()).toBeTruthy();
  }
  await page.reload();
  await openActivity(page);
  await expect(page.getByText(/explanation · failed/)).toBeVisible();
  await page.getByRole("button", { name: "Retry" }).click();
  await expect(page.getByText(/explanation · requested/)).toBeVisible();

  const availableRequest = await page.request.post(
    apiUrl("/api/v1/learning-variants"),
    {
      data: {
        sourceActivityId: activity.id,
        objectiveId: objective.id,
        variantKind: "example",
        recommendationReason: "A concrete case will connect the equation.",
        retryKey: `available-${Date.now()}`,
      },
    },
  );
  expect(availableRequest.ok()).toBeTruthy();
  const available = await availableRequest.json();
  for (const status of ["running", "review_required", "published"]) {
    const transition = await page.request.patch(
      apiUrl(`/api/v1/generation-runs/${available.generationRunId}`),
      { data: { status } },
    );
    expect(transition.ok()).toBeTruthy();
  }
  const citationId = await supportedCitation(
    page,
    "A car accelerating at two meters per second squared gains two meters per second of velocity each second.",
    "gains two meters per second of velocity each second",
  );
  const publish = await page.request.patch(
    apiUrl(`/api/v1/learning-variants/${available.id}`),
    {
      data: {
        content: {
          heading: "A concrete acceleration example",
          body: "After three seconds, the velocity change is six m/s.",
        },
        sourceReferences: [citationId],
      },
    },
  );
  expect(publish.ok()).toBeTruthy();
  await page.reload();
  await openActivity(page);
  await expect(page.getByText(/example · available/)).toBeVisible();
  await expect(page.getByText(/velocity change is six m\/s/)).toBeVisible();
  const masteryAfter = await page.request.get(
    apiUrl(`/api/v1/progress/${journey.id}/objectives/${objective.id}`),
  );
  expect(masteryAfter.status()).toBe(masteryBefore.status());
  if (masteryBefore.ok()) {
    expect(await masteryAfter.json()).toEqual(await masteryBefore.json());
  }
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
    "physics-mechanics-starter",
  );

  expect(journey.goal.catalogEntryId).toBe("physics-mechanics-starter");
  expect(journey.goal.catalogEntryVersion).toBe(1);

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
  const evidence = await page.request.post(
    apiUrl("/api/v1/progress/evidence"),
    {
      data: {
        journeyId: journey.id,
        objectiveId: objective.id,
        activityId: activity.id,
        attemptId: attempt.id,
        contentVersion: activity.contentVersion,
        value: 0.5,
        derivationVersion: 1,
      },
    },
  );
  expect(evidence.ok()).toBeTruthy();
  const due = await page.request.get(apiUrl("/api/v1/reviews/due"));
  const dueReviews = await due.json();
  const review = dueReviews.find(
    (candidate: { activityId: string }) => candidate.activityId === activity.id,
  );
  expect(review).toBeTruthy();
  const rating = await page.request.post(
    apiUrl(`/api/v1/reviews/${review.id}/ratings`),
    { data: { rating: "good", learnerTimezone: "Asia/Tokyo" } },
  );
  expect(rating.ok()).toBeTruthy();
  const dayParts = new Intl.DateTimeFormat("en-US", {
    timeZone: "Asia/Tokyo",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  })
    .formatToParts(new Date())
    .reduce<Record<string, string>>((parts, part) => {
      parts[part.type] = part.value;
      return parts;
    }, {});
  const qualifyingDay = `${dayParts.year}-${dayParts.month}-${dayParts.day}`;
  const streak = await page.request.post(apiUrl("/api/v1/progress/streaks"), {
    data: {
      journeyId: journey.id,
      activityId: activity.id,
      qualifyingEventKey: `attempt:${attempt.id}`,
      learnerTimezone: "Asia/Tokyo",
      qualifyingDay,
    },
  });
  expect(streak.ok()).toBeTruthy();
  const finishedSession = await page.request.post(
    apiUrl(`/api/v1/learning/sessions/${session.id}/finish`),
    { data: { completed: true, responses: [] } },
  );
  expect(finishedSession.ok()).toBeTruthy();
  const analytics = await page.request.get(
    apiUrl(
      `/api/v1/learning/journeys/${journey.id}/analytics?timezone=Asia%2FTokyo`,
    ),
  );
  expect(analytics.ok()).toBeTruthy();
  const metrics = await analytics.json();
  expect(metrics.averageScore.denominator).toBe(1);
  expect(metrics.averageScore.value).toBeCloseTo(0.5, 5);
  expect(metrics.attempts).toBe(1);
  expect(metrics.reviewHistory).toHaveLength(1);
  expect(metrics.streak.currentDays).toBe(1);
  await page.goto("/learning");
  const analyticsPanel = page.getByTestId("learner-analytics");
  await expect(analyticsPanel).toBeVisible();
  const averageScoreCard = analyticsPanel
    .locator("div")
    .filter({ hasText: "Average score" })
    .last();
  await expect(averageScoreCard.getByText("50%")).toBeVisible();
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
    "flink-cs-starter",
  );

  expect(journey.goal.catalogEntryId).toBe("flink-cs-starter");
  expect(journey.goal.catalogEntryVersion).toBe(1);

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
