/**
 * Assessment visibility.
 *
 * Covers:
 * - An assessment created with visibility=private is NOT returned to other users
 * - An assessment created with visibility=public appears in /v1/assessments/explore
 * - Cross-owner answer: a user can start a session on another user's public assessment
 *
 * All tests are pure API (no browser) for speed and determinism.
 */
import { test, expect } from "@playwright/test";
import { API_URL, registerUser } from "./helpers";

test.describe("assessment visibility", () => {
  test.describe.configure({ mode: "serial" });

  let ownerToken: string;
  let otherToken: string;

  // IDs created in setup
  let privateAssessmentId: string;
  let publicAssessmentId: string;

  test.beforeAll(async ({ request }) => {
    const ts = Date.now();
    ownerToken = await registerUser(request, {
      email: `e2e-vis-owner-${ts}@example.com`,
      name: "Visibility Owner",
    });
    otherToken = await registerUser(request, {
      email: `e2e-vis-other-${ts}@example.com`,
      name: "Other User",
    });

    async function createAssessment(
      visibility: string,
      title: string,
    ): Promise<string> {
      const r = await request.post(`${API_URL}/v1/assessments`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
        data: {
          title,
          visibility,
          mode: "practice",
          objectives: [],
          method: "manual",
        },
      });
      expect(r.status(), `create ${visibility} assessment`).toBe(201);
      const body = await r.json();
      const id: string = body.id;

      // Add a minimal inline question
      await request.post(`${API_URL}/v1/assessments/${id}/questions`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
        data: {
          kind: "mc",
          prompt: `e2e ${visibility} assessment question?`,
        },
      });

      // Publish
      const pr = await request.patch(`${API_URL}/v1/assessments/${id}`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
        data: { status: "active" },
      });
      expect(pr.status(), `publish ${visibility} assessment`).toBe(200);
      return id;
    }

    privateAssessmentId = await createAssessment(
      "private",
      `e2e-private-${ts}`,
    );
    publicAssessmentId = await createAssessment("public", `e2e-public-${ts}`);
  });

  test("owner can list their own private assessment", async ({ request }) => {
    const r = await request.get(
      `${API_URL}/v1/assessments/${privateAssessmentId}`,
      {
        headers: { Authorization: `Bearer ${ownerToken}` },
      },
    );
    expect(r.status()).toBe(200);
  });

  test("other user CANNOT see a private assessment in the listing", async ({
    request,
  }) => {
    const r = await request.get(`${API_URL}/v1/assessments?status=active`, {
      headers: { Authorization: `Bearer ${otherToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    const assessments: Array<{ id: string }> = body.assessments ?? body;
    expect(assessments.map((a) => a.id)).not.toContain(privateAssessmentId);
  });

  test("public assessment appears in GET /v1/assessments/explore", async ({
    request,
  }) => {
    const r = await request.get(`${API_URL}/v1/assessments/explore`, {
      headers: { Authorization: `Bearer ${otherToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    const assessments: Array<{ id: string }> = body.assessments ?? body;
    expect(assessments.map((a) => a.id)).toContain(publicAssessmentId);
  });

  test("private assessment does NOT appear in GET /v1/assessments/explore", async ({
    request,
  }) => {
    const r = await request.get(`${API_URL}/v1/assessments/explore`, {
      headers: { Authorization: `Bearer ${otherToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    const assessments: Array<{ id: string }> = body.assessments ?? body;
    expect(assessments.map((a) => a.id)).not.toContain(privateAssessmentId);
  });

  test("authenticated user can start a session on another user's public assessment", async ({
    request,
  }) => {
    const r = await request.post(`${API_URL}/v1/sessions`, {
      headers: { Authorization: `Bearer ${otherToken}` },
      data: { assessmentId: publicAssessmentId },
    });
    expect(r.status()).toBe(201);
  });
});
