/**
 * Owner isolation spec: verify that users cannot see other owners' assessments.
 *
 * This tests the core invariant that assessments are strictly owner-scoped.
 * A registered user must not be able to access, list, or create sessions
 * for another user's assessments, regardless of status.
 */

import { test, expect } from "@playwright/test";
import { API_URL, registerUser } from "./helpers";

test("owner isolation: stranger cannot access another owner's assessment", async ({
  request,
}) => {
  // Owner A registers and creates an assessment
  const ownerA = await registerUser(request, {
    name: "Owner A",
    email: `owner-a-${Date.now()}@example.com`,
  });

  const createResp = await request.post(`${API_URL}/v1/assessments`, {
    headers: { Authorization: `Bearer ${ownerA}` },
    data: {
      title: "Owner A's Private Assessment",
      mode: "practice",
      objectives: [],
      course: "Test Course",
    },
  });
  expect(createResp.ok(), "Owner A create assessment").toBeTruthy();
  const assessmentData = await createResp.json();
  const assessmentId = assessmentData.id as string;

  // Owner A publishes (activates) the assessment
  const publishResp = await request.patch(
    `${API_URL}/v1/assessments/${assessmentId}`,
    {
      headers: { Authorization: `Bearer ${ownerA}` },
      data: { status: "active" },
    },
  );
  expect(publishResp.ok(), "Owner A publish assessment").toBeTruthy();

  // Stranger B registers
  const strangerB = await registerUser(request, {
    name: "Stranger B",
    email: `stranger-b-${Date.now()}@example.com`,
  });

  // Stranger B tries to GET the assessment — should 404
  const getResp = await request.get(
    `${API_URL}/v1/assessments/${assessmentId}`,
    {
      headers: { Authorization: `Bearer ${strangerB}` },
    },
  );
  expect(getResp.status()).toBe(404);
  const getBody = await getResp.json();
  expect(getBody.error?.code).toBe("not_found");
});

test("owner isolation: stranger's list does not include other owner's active assessment", async ({
  request,
}) => {
  // Owner A registers and creates an active assessment
  const ownerA = await registerUser(request, {
    name: "Owner A",
    email: `owner-a-${Date.now()}@example.com`,
  });

  const createResp = await request.post(`${API_URL}/v1/assessments`, {
    headers: { Authorization: `Bearer ${ownerA}` },
    data: {
      title: "Owner A's Listed Assessment",
      mode: "practice",
      objectives: [],
    },
  });
  expect(createResp.ok()).toBeTruthy();
  const assessmentData = await createResp.json();
  const assessmentId = assessmentData.id as string;

  // Activate it
  await request.patch(`${API_URL}/v1/assessments/${assessmentId}`, {
    headers: { Authorization: `Bearer ${ownerA}` },
    data: { status: "active" },
  });

  // Stranger B registers
  const strangerB = await registerUser(request, {
    name: "Stranger B",
    email: `stranger-b-${Date.now()}@example.com`,
  });

  // Stranger B lists assessments — Owner A's must not be in the list
  const listResp = await request.get(`${API_URL}/v1/assessments`, {
    headers: { Authorization: `Bearer ${strangerB}` },
  });
  expect(listResp.ok()).toBeTruthy();
  const listData = await listResp.json();
  const assessments = listData.assessments as Array<{ id: string }>;
  const leaked = assessments.some((a) => a.id === assessmentId);
  expect(leaked, "Stranger sees Owner A's assessment in list").toBeFalsy();
});

test("owner isolation: stranger cannot create session for other owner's assessment", async ({
  request,
}) => {
  // Owner A registers and creates an active assessment
  const ownerA = await registerUser(request, {
    name: "Owner A",
    email: `owner-a-${Date.now()}@example.com`,
  });

  const createResp = await request.post(`${API_URL}/v1/assessments`, {
    headers: { Authorization: `Bearer ${ownerA}` },
    data: {
      title: "Owner A's Session Test Assessment",
      mode: "practice",
      objectives: [],
    },
  });
  expect(createResp.ok()).toBeTruthy();
  const assessmentData = await createResp.json();
  const assessmentId = assessmentData.id as string;

  // Activate it
  await request.patch(`${API_URL}/v1/assessments/${assessmentId}`, {
    headers: { Authorization: `Bearer ${ownerA}` },
    data: { status: "active" },
  });

  // Stranger B registers
  const strangerB = await registerUser(request, {
    name: "Stranger B",
    email: `stranger-b-${Date.now()}@example.com`,
  });

  // Stranger B tries to create a session — should 404 (assessment not found)
  const sessionResp = await request.post(`${API_URL}/v1/sessions`, {
    headers: { Authorization: `Bearer ${strangerB}` },
    data: { assessmentId },
  });
  expect(sessionResp.status()).toBe(404);
});
