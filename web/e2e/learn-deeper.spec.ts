import { test, expect } from "@playwright/test";
import { API_URL, loginAs, setAuthCookie } from "./helpers";

test.describe("Learn Deeper flow", () => {
  test.describe.configure({ mode: "serial" });

  let token: string;
  let userEmail: string;

  test.beforeAll(async ({ request }) => {
    // Generate a unique email to avoid collision on parallel runs
    userEmail = `deeper-${Date.now()}@example.com`;
    // Register user
    const resp = await request.post(`${API_URL}/v1/auth/register`, {
      data: {
        email: userEmail,
        name: "Deeper Tester",
        password: "password123",
        role: "user",
      },
    });
    expect(resp.status()).toBe(201);
    token = (await resp.json()).token;
  });

  test.beforeEach(async ({ page }) => {
    await setAuthCookie(page, token);
  });

  test("author and learner journey for learn-deeper features", async ({
    page,
  }) => {
    // 1. Navigate to Author list and create a new assessment
    await page.goto("/author");
    await page.getByRole("button", { name: "New assessment" }).click();
    await expect(page).toHaveURL(/\/author\/[0-9a-fA-F-]+/);

    // Get assessment ID from URL
    const url = page.url();
    const match = url.match(/\/author\/([0-9a-fA-F-]+)/);
    expect(match).not.toBeNull();
    const assessmentId = match![1];

    // 2. Set title of assessment
    const titleInput = page.locator('label:has-text("Title") input');
    await titleInput.fill("Geography of Europe");
    await titleInput.blur();
    await page.waitForTimeout(500);

    // 3. Add Question 1: Paris T/F
    await page.getByRole("button", { name: "Add" }).first().click();
    await page.getByRole("button", { name: "T/F" }).click();
    await page.waitForTimeout(500);

    // Wait for the prompt inputs to render/update for Q1
    const promptInput = page.locator(
      'label:has-text("Question prompt") textarea',
    );
    // Fill Prompt and wait for save
    await promptInput.fill("Paris is the capital of France.");
    let savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await promptInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    const refUrlInput = page.locator('label:has-text("Reference URL") input');
    // Fill Reference URL and wait for save
    await refUrlInput.fill("https://en.wikipedia.org/wiki/Paris");
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await refUrlInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    const deepDiveInput = page.locator(
      'label:has-text("Deep Dive Study Notes") textarea',
    );
    // Fill Deep Dive and wait for save
    await deepDiveInput.fill(
      "### Paris Deep Dive\nParis has been a major settlement for over two millennia.",
    );
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await deepDiveInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    const tagInput = page.locator('label:has-text("Tag") input');
    // Fill Tag and wait for save
    await tagInput.fill("geography, europe");
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await tagInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    // Set correct answer as True
    await page.getByRole("button", { name: "True" }).click();
    await page.waitForTimeout(500);

    // Verify live preview shows
    await expect(page.getByText("Paris Deep Dive")).toBeVisible();

    // 4. Add Question 2: Rome T/F
    await page.getByRole("button", { name: "Add" }).first().click();
    await page.getByRole("button", { name: "T/F" }).click();
    await page.waitForTimeout(500);

    // Wait and write Q2 prompt
    await promptInput.fill("Rome is the capital of Italy.");
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await promptInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    await refUrlInput.fill("https://en.wikipedia.org/wiki/Rome");
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await refUrlInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    await deepDiveInput.fill(
      "### Rome Deep Dive\nRome has a history spanning 28 centuries.",
    );
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await deepDiveInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    await tagInput.fill("geography, europe");
    savePromise = page.waitForResponse(
      (r) =>
        r.url().includes("/v1/assessments/") &&
        r.request().method() === "GET" &&
        r.status() === 200,
    );
    await tagInput.blur();
    await savePromise;
    await page.waitForTimeout(500);

    // Set correct answer as True
    await page.getByRole("button", { name: "True" }).click();
    await page.waitForTimeout(500);

    // Verify live preview shows for Q2
    await expect(page.getByText("Rome Deep Dive")).toBeVisible();

    // 5. Publish the assessment
    await page.getByRole("button", { name: "Publish" }).click();
    await expect(page).toHaveURL(/\/assessments\/[0-9a-fA-F-]+\/preview/);

    // 6. Create a session on the published assessment via API
    const sr = await page.request.post(`${API_URL}/v1/sessions`, {
      data: { assessmentId },
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(sr.status()).toBe(201);
    const sessionId = (await sr.json()).sessionId;

    // Retrieve questions for the session
    const sessionDetail = await page.request.get(
      `${API_URL}/v1/sessions/${sessionId}`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    expect(sessionDetail.ok()).toBeTruthy();
    const sessionDetailData = await sessionDetail.json();
    const questions = sessionDetailData.questions ?? [];
    expect(questions.length).toBe(2);

    // Answer Q1 correctly (True) and Q2 incorrectly (False)
    for (const q of questions) {
      const isParis = q.prompt.includes("Paris");
      const answerVal = isParis ? true : false;
      const answerResp = await page.request.post(
        `${API_URL}/v1/sessions/${sessionId}/answer`,
        {
          data: { questionId: q.questionId, response: { answer: answerVal } },
          headers: { Authorization: `Bearer ${token}` },
        },
      );
      expect(answerResp.status()).toBe(200);
    }

    // Finish session
    const finishResp = await page.request.post(
      `${API_URL}/v1/sessions/${sessionId}/finish`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    expect(finishResp.status()).toBe(200);

    // 7. Open results page and use "Dive deeper" Drawer
    await page.goto(`/sessions/${sessionId}/results`);
    await expect(page.getByText("Quiz Results")).toBeVisible({
      timeout: 10000,
    });

    // Open drawer for Q1 (Paris)
    const parisCard = page.locator(
      'div.MuiCard-root:has-text("Paris is the capital of France.")',
    );
    await expect(parisCard).toBeVisible();
    await parisCard.getByRole("button", { name: "Dive deeper" }).click();

    // Verify Drawer displays Q1 info
    await expect(
      page.getByRole("heading", { name: "Dive Deeper" }),
    ).toBeVisible({ timeout: 5000 });
    await expect(
      page.getByText("Paris is the capital of France."),
    ).toBeVisible();
    await expect(page.getByText("Deep Dive Study Notes")).toBeVisible();
    await expect(
      page.getByText("Paris has been a major settlement"),
    ).toBeVisible();
    await expect(page.getByText("geography").first()).toBeVisible();
    await expect(page.getByText("europe").first()).toBeVisible();

    const visitSourceBtn = page.getByRole("link", {
      name: "Visit External Source",
    });
    await expect(visitSourceBtn).toBeVisible();
    await expect(visitSourceBtn).toHaveAttribute(
      "href",
      "https://en.wikipedia.org/wiki/Paris",
    );

    // Verify related questions displays Rome
    const relatedCard = page.locator(
      'div.MuiCard-root:has-text("Rome is the capital of Italy.")',
    );
    await expect(relatedCard).toBeVisible();

    // Navigate to Rome question in Drawer
    await relatedCard.click();

    // Verify Rome details in drawer
    await expect(page.getByText("Rome is the capital of Italy.")).toBeVisible();
    await expect(
      page.getByText("Rome has a history spanning 28 centuries."),
    ).toBeVisible();

    // Back button should be visible
    const backBtn = page.getByRole("button", { name: "Back" });
    await expect(backBtn).toBeVisible();
    await backBtn.click();

    // Should return to Paris details
    await expect(
      page.getByText("Paris is the capital of France."),
    ).toBeVisible();

    // Close the drawer
    await page
      .locator('button:has(svg[data-testid="CloseIcon"])')
      .first()
      .click();
    await expect(
      page.getByRole("heading", { name: "Dive Deeper" }),
    ).not.toBeVisible();
  });
});
