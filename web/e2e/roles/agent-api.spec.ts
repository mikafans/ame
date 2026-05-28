import { test, expect } from "@playwright/test";

const API = process.env.E2E_API_URL ?? "http://localhost:8080";

test.describe("Agent API surface", () => {
  test.describe.configure({ mode: "serial" });

  let agentKey: string;

  test.beforeAll(async ({ request }) => {
    const ts = Date.now();
    const ar = await request.post(`${API}/v1/agents/register`, {
      data: {
        label: `e2e-agent-${ts}`,
        access_code: process.env.AME_AGENT_ACCESS_CODE ?? "e2e-access-code",
        scopes: [
          "quiz.read",
          "quiz.write",
          "stats.read",
          "plan.write",
          "plan.read",
        ],
      },
    });
    expect([200, 201]).toContain(ar.status());
    const body = await ar.json();
    agentKey = body.key ?? body.apiKey;
    expect(agentKey).toBeTruthy();
  });

  test("MCP manifest contains question and stats tools", async ({
    request,
  }) => {
    const r = await request.get(`${API}/v1/agents/mcp.json`);
    expect(r.status()).toBe(200);
    const manifest = await r.json();
    const names = manifest.tools.map((t: { name: string }) => t.name);
    expect(names).toContain("question.list");
    expect(names).toContain("question.create");
    expect(names).toContain("question.update");
    expect(names).toContain("question.promote");
    expect(names).toContain("stats.user");
  });

  test("can import and promote questions", async ({ request }) => {
    const r = await request.post(`${API}/v1/questions`, {
      data: {
        questions: [
          {
            kind: "mc",
            prompt: "e2e agent question?",
            payload: { options: ["A", "B"], correct_index: 0 },
            tags: [],
          },
        ],
      },
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect([200, 201]).toContain(r.status());
    const qid = (await r.json()).questions[0].id;

    const pr = await request.post(`${API}/v1/questions/${qid}/promote`, {
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect([200, 204]).toContain(pr.status());
  });

  test("can fetch user stats", async ({ request }) => {
    const r = await request.get(`${API}/v1/me/stats`, {
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect(r.status()).toBe(200);
    const stats = await r.json();
    expect(stats).toHaveProperty("avg_score");
    expect(stats).toHaveProperty("current_streak");
  });

  test("can create and retrieve a study plan", async ({ request }) => {
    const cr = await request.post(`${API}/v1/plans`, {
      data: { goal: "e2e plan goal", lookbackDays: 7 },
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect([200, 201]).toContain(cr.status());
    const planId = (await cr.json()).id;

    const gr = await request.get(`${API}/v1/plans/${planId}`, {
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect(gr.status()).toBe(200);
    const plan = await gr.json();
    expect(plan.weeks).toBeDefined();
    expect(plan.weeks.length).toBeGreaterThan(0);
  });
});
