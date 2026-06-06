/**
 * Account deactivation (AI-4.3 behavior).
 *
 * PATCH /v1/admin/users/{id} {"disabled":true} sets deactivated_at.
 * A deactivated user cannot log in (401).
 * An agent whose owner is deactivated is also rejected (401).
 * Re-enabling ({"disabled":false}) restores access for both.
 *
 * NOTE: the old /deactivate and /role endpoints are removed; everything goes
 * through PATCH.
 */
import { test, expect } from "@playwright/test";
import { API_URL, loginAs, registerUserFull } from "./helpers";

test.describe("account deactivation", () => {
  test.describe.configure({ mode: "serial" });

  let adminToken: string;
  let targetUserId: string;
  let targetEmail: string;
  let targetPassword: string;
  let agentKey: string;

  test.beforeAll(async ({ request }) => {
    // Log in as admin
    adminToken = await loginAs(request, "admin@example.com");

    // Register a fresh owner + their agent
    const ts = Date.now();
    targetEmail = `e2e-deact-${ts}@example.com`;
    targetPassword = "password123";

    const { token: ownerToken, id } = await registerUserFull(request, {
      email: targetEmail,
      name: "Deactivation Target",
      password: targetPassword,
    });
    targetUserId = id;

    // Create an agent under this owner
    const agentResp = await request.post(`${API_URL}/v1/me/agents`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: {
        label: `e2e-deact-agent-${ts}`,
        scopes: ["assessment.read"],
      },
    });
    expect(agentResp.status()).toBe(201);
    const agentBody = await agentResp.json();
    agentKey = agentBody.apiKey ?? agentBody.api_key;
    expect(agentKey).toBeTruthy();
  });

  test("owner can log in before deactivation", async ({ request }) => {
    // Use loginAs helper (retries 429s from the auth rate-limiter)
    const token = await loginAs(request, targetEmail, targetPassword);
    expect(token).toBeTruthy();
  });

  test("agent can call API before owner deactivation", async ({ request }) => {
    const r = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: { tool: "assessment.list", params: {} },
    });
    // 200 (empty or populated list) means the agent token is valid and owner is active
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.ok).toBe(true);
  });

  test("admin can deactivate the user via PATCH", async ({ request }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${targetUserId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { disabled: true },
    });
    expect(r.status()).toBe(204);
  });

  test("deactivated user cannot log in (401)", async ({ request }) => {
    // Retry on 429 (rate-limiter) — we want the 401 from deactivated_at, not a
    // rate-limit rejection which would mask whether deactivation actually works.
    let status = 0;
    for (let i = 0; i < 5; i++) {
      const r = await request.post(`${API_URL}/v1/auth/login`, {
        data: { email: targetEmail, password: targetPassword },
      });
      status = r.status();
      if (status !== 429) break;
      await new Promise((res) => setTimeout(res, 2000 * (i + 1)));
    }
    expect(status).toBe(401);
  });

  test("agent whose owner is deactivated is rejected (401)", async ({
    request,
  }) => {
    const r = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: { tool: "assessment.list", params: {} },
    });
    expect(r.status()).toBe(401);
  });

  test("admin can re-enable the user via PATCH", async ({ request }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${targetUserId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { disabled: false },
    });
    expect(r.status()).toBe(204);
  });

  test("re-enabled user can log in again", async ({ request }) => {
    // Use loginAs helper (retries 429s from the auth rate-limiter)
    const token = await loginAs(request, targetEmail, targetPassword);
    expect(token).toBeTruthy();
  });

  test("agent can call API again after owner is re-enabled", async ({
    request,
  }) => {
    // The disable operation sets deactivated_at on the owner but does NOT
    // revoke agent sub-account tokens — those remain in tb_api_tokens.
    // Re-enabling the owner clears deactivated_at, so the extractor's
    // owner_deactivated_at check passes again and agent calls succeed.
    const r = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: { tool: "assessment.list", params: {} },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.ok).toBe(true);
  });
});
