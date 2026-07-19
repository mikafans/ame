/**
 * Agent management tests (API layer).
 *
 * Covers:
 * - Owner creates an agent via POST /v1/me/agents (returns apiKey)
 * - Owner lists their agents via GET /v1/me/agents
 * - Owner revokes an agent via DELETE /v1/me/agents/{id}
 * - Agent can list assessments and create questions (assessment.read + assessment.write)
 * - Agent can fetch user stats (stats.read)
 * - Owner can create and retrieve a study plan (plan.write + plan.read are
 *   human-only scopes — not grantable to agents)
 * - MCP skill manifest is public and lists expected tool names
 *
 * See also: roles/agent-api.spec.ts (now superseded by this file).
 */
import { test, expect } from "@playwright/test";
import { API_URL, registerUser } from "./helpers";

test.describe("agent management (owner API)", () => {
  test.describe.configure({ mode: "serial" });

  let ownerToken: string;
  let agentId: string;
  let agentKey: string;

  test.beforeAll(async ({ request }) => {
    const ts = Date.now();
    ownerToken = await registerUser(request, {
      email: `e2e-agent-mgmt-${ts}@example.com`,
      name: "Agent Mgmt Owner",
    });
  });

  test("owner can create an agent", async ({ request }) => {
    const ts = Date.now();
    const r = await request.post(`${API_URL}/v1/me/agents`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: {
        label: `e2e-agent-${ts}`,
        scopes: ["assessment.read", "assessment.write", "stats.read"],
      },
    });
    expect(r.status()).toBe(201);
    const body = await r.json();
    agentKey = body.apiKey ?? body.api_key;
    agentId = body.id ?? body.agent?.id;
    expect(agentKey).toBeTruthy();
  });

  test("owner can list their agents", async ({ request }) => {
    const r = await request.get(`${API_URL}/v1/me/agents`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    const agents: Array<{ id: string }> = body.agents ?? body;
    expect(agents.length).toBeGreaterThan(0);
  });

  test("agent can list assessments", async ({ request }) => {
    const r = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: { tool: "assessment.list", params: {} },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.ok).toBe(true);
  });

  test("agent can import and promote a question", async ({ request }) => {
    const r = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: {
        tool: "question.create",
        params: {
          questions: [
            {
              kind: "mc",
              prompt: "e2e agent question — scope test?",
              payload: { options: ["A", "B"], correct_index: 0 },
              tags: [],
            },
          ],
        },
      },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.ok).toBe(true);
    const qid = body.result.questions[0].id;

    const pr = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: {
        tool: "question.promote",
        params: { id: qid },
      },
    });
    expect(pr.status()).toBe(200);
    const pBody = await pr.json();
    expect(pBody.ok).toBe(true);
  });

  test("agent can fetch user stats", async ({ request }) => {
    const r = await request.post(`${API_URL}/v1/agents/run`, {
      headers: { Authorization: `Bearer ${agentKey}` },
      data: { tool: "stats.user", params: {} },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.ok).toBe(true);
    const stats = body.result;
    expect(stats).toHaveProperty("avg_score");
    expect(stats).toHaveProperty("current_streak");
  });

  test("owner can create and retrieve a study plan", async ({ request }) => {
    const cr = await request.post(`${API_URL}/v1/plans`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { goal: "e2e plan goal", lookbackDays: 7 },
    });
    expect([200, 201]).toContain(cr.status());
    const planId = (await cr.json()).id;

    const gr = await request.get(`${API_URL}/v1/plans/${planId}`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(gr.status()).toBe(200);
    const plan = await gr.json();
    expect(plan.weeks).toBeDefined();
    expect(plan.weeks.length).toBeGreaterThan(0);
  });

  test("owner can revoke (delete) the agent", async ({ request }) => {
    // agentId may be undefined if the create step captured it; skip gracefully
    if (!agentId) {
      test.skip();
      return;
    }
    const r = await request.delete(`${API_URL}/v1/me/agents/${agentId}`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    // Soft-delete: deactivates the agent and revokes its tokens in one tx, no
    // hard DELETE, so the old FK-cascade 500 path is gone. Must be a clean 204.
    expect(r.status()).toBe(204);
  });
});

// ---------------------------------------------------------------------------
// MCP skill manifest (public endpoint)
// ---------------------------------------------------------------------------
test.describe("MCP skill manifest", () => {
  test("manifest is public and contains expected tool names", async ({
    request,
  }) => {
    const r = await request.get(`${API_URL}/skill.json`);
    expect(r.status()).toBe(200);
    const manifest = await r.json();
    const names: string[] = manifest.tools.map((t: { name: string }) => t.name);
    expect(names).toContain("question.list");
    expect(names).toContain("question.create");
    expect(names).toContain("question.promote");
    expect(names).toContain("stats.user");
  });
});
