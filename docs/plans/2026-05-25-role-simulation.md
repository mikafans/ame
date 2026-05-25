# Role Simulation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify the learner + agent flows end-to-end via three simulation layers: API scripts, MCP manifest, and Playwright role specs.

**Architecture:** Fix two API gaps (MCP tools, explanation display), add a study plan page, then build three simulation layers that each target a different failure surface (API logic, MCP schema, browser rendering). The layers are independent but ordered by dependency.

**Tech Stack:** Rust/Axum (MCP manifest), Next.js/TypeScript (plan page + results fix), Python PEP 723 (API scripts), Playwright (browser specs).

**Spec:** `docs/specs/2026-05-25-role-simulation-design.md`

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `api/src/http/agents.rs` | Modify | Add 5 MCP tools: question.list/create/update/promote + stats.user |
| `web/app/(learner)/sessions/[id]/results/page.tsx` | Modify | Surface `explanation` from API instead of placeholder |
| `web/app/(learner)/plans/[id]/page.tsx` | Create | Study plan detail page |
| `scripts/simulate/learner.py` | Create | Learner role API simulation |
| `scripts/simulate/instructor.py` | Create | Instructor role API simulation |
| `scripts/simulate/agent.py` | Create | Agent role API simulation |
| `Makefile` | Modify | Add `simulate` target |
| `web/e2e/roles/learner.spec.ts` | Create | Learner browser simulation |
| `web/e2e/roles/instructor.spec.ts` | Create | Instructor browser simulation |
| `web/e2e/roles/agent-api.spec.ts` | Create | Agent API-only Playwright spec |
| `docs/specs/mcp-prompts.md` | Create | Sample MCP prompts per role |

---

## Task 1: Add question tools to MCP manifest

**Files:**
- Modify: `api/src/http/agents.rs` (inside the `tools: Vec<Value> = vec![...]` block, after the `exam.stats` entry)

- [ ] **Step 1: Locate the insertion point**

Open `api/src/http/agents.rs`. Find the line containing `"exam.stats"` — the new question tools go immediately after its closing `),`.

- [ ] **Step 2: Add the five tools**

```rust
        // Questions
        tool(
            "question.list",
            "List questions in the bank. Filter by tag, status, or rating range.",
            json!({"type":"object","properties":{"tag":{"type":"string"},"status":{"type":"string","enum":["draft","live","archived"]},"limit":{"type":"integer"},"offset":{"type":"integer"}}}),
            "GET",
            "/v1/questions",
            Some("quiz.read"),
            strict,
        ),
        tool(
            "question.create",
            "Batch-create one or more questions in the bank.",
            json!({"type":"object","required":["questions"],"properties":{"questions":{"type":"array","items":{"type":"object","required":["kind","prompt","payload"],"properties":{"kind":{"type":"string","enum":["mc","tf","short","essay","code"]},"prompt":{"type":"string"},"payload":{"type":"object"},"explanation":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"points":{"type":"integer"}}}}}}),
            "POST",
            "/v1/questions",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "question.update",
            "Patch a question's prompt, explanation, payload, or tags.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"prompt":{"type":"string"},"explanation":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"points":{"type":"integer"}}}),
            "PATCH",
            "/v1/questions/{id}",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "question.promote",
            "Promote a draft question to live status.",
            id_only(),
            "POST",
            "/v1/questions/{id}/promote",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "stats.user",
            "Get personal learning stats for the authenticated user: avg score, streak, mastery topics, hours spent.",
            json!({"type":"object","properties":{"window":{"type":"string","enum":["4w","12w","all"]}}}),
            "GET",
            "/v1/me/stats",
            Some("stats.read"),
            strict,
        ),
```

- [ ] **Step 3: Verify the manifest compiles and contains the tools**

```bash
cd api && mise exec -- cargo check 2>&1 | tail -5
```
Expected: `Finished` with no errors.

```bash
# With API running: make dev-env in another terminal
curl -s http://localhost:8080/v1/agents/mcp.json | jq '[.tools[].name] | sort'
```
Expected output includes: `"question.list"`, `"question.create"`, `"question.update"`, `"question.promote"`, `"stats.user"`.

- [ ] **Step 4: Commit**

```bash
git add api/src/http/agents.rs
git commit -m "feat: add question and stats.user tools to MCP manifest"
```

---

## Task 2: Surface explanation in session results

**Files:**
- Modify: `web/app/(learner)/sessions/[id]/results/page.tsx`

- [ ] **Step 1: Add `explanation` to the `Answer` interface**

Find the `Answer` interface (around line 8) and add one field:

```typescript
interface Answer {
  qid: string;
  correct: boolean;
  points: number;
  max: number;
  type: "mc" | "tf" | "short" | "essay" | "code";
  prompt: string;
  given: string;
  note: string;
  gradeStatus: string;
  explanation?: string;   // add this line
}
```

- [ ] **Step 2: Map explanation from the API response**

Find the `questions.map((q: any) => {` block (around line 86). The returned object currently ends with `note: ...`. Add `explanation` to the returned object:

```typescript
            return {
              qid: q.questionId,
              correct: attempt?.is_correct ?? false,
              points: points,
              max: q.points,
              type: q.kind,
              prompt: q.prompt,
              given: typeof body === "string" ? body : "",
              gradeStatus: status,
              note:
                status === "pending_manual"
                  ? "Pending manual review"
                  : status === "graded"
                    ? ""
                    : "Not graded yet",
              explanation: q.explanation ?? undefined,   // add this line
            };
```

- [ ] **Step 3: Replace the placeholder with actual explanation**

Find the `{/* Expanded explanation */}` block (around line 503). Replace the hardcoded string:

Before:
```tsx
              {expandedItems.has(answer.qid) && (
                <div ...>
                  Explanation would appear here.
                </div>
              )}
```

After:
```tsx
              {expandedItems.has(answer.qid) && answer.explanation && (
                <div
                  style={{
                    marginTop: 12,
                    padding: "10px 14px",
                    background: "var(--surface-2)",
                    borderRadius: 4,
                    fontSize: 13,
                    lineHeight: 1.6,
                    color: "var(--text-2)",
                    fontFamily: "var(--serif)",
                  }}
                >
                  {answer.explanation}
                </div>
              )}
```

- [ ] **Step 4: Type-check**

```bash
cd web && mise exec -- bun run type-check 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add web/app/\(learner\)/sessions/\[id\]/results/page.tsx
git commit -m "fix: surface question explanation in session results"
```

---

## Task 3: Add study plan detail page

**Files:**
- Create: `web/app/(learner)/plans/[id]/page.tsx`

The `StudyPlan` API response shape (from `api/src/engine/planner.rs`):
```
{ id, user_id, goal, lookback_days, generated_at, weeks: [{ week_num, focus, items: [{ kind, ref_id, hours_est }] }] }
```

- [ ] **Step 1: Create the page file**

```typescript
// web/app/(learner)/plans/[id]/page.tsx
"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { Button, Card } from "@/components/ui";

interface PlanItem {
  kind: string;
  ref_id: string;
  hours_est: number;
}

interface PlanWeek {
  week_num: number;
  focus: string;
  items: PlanItem[];
}

interface StudyPlan {
  id: string;
  goal: string;
  lookback_days: number;
  generated_at: string;
  weeks: PlanWeek[];
}

export default function PlanPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const router = useRouter();
  const [plan, setPlan] = useState<StudyPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/plans/{id}", { params: { path: { id } } })
      .then(({ data, error }: { data?: StudyPlan; error?: unknown }) => {
        if (error || !data) {
          setNotFound(true);
        } else {
          setPlan(data);
        }
      })
      .catch(() => setNotFound(true))
      .finally(() => setLoading(false));
  }, [token, id]);

  if (loading) {
    return (
      <div style={{ padding: "28px 36px", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 13 }}>
        Loading plan…
      </div>
    );
  }

  if (notFound || !plan) {
    return (
      <div style={{ padding: "28px 36px" }}>
        <div style={{ color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 13, marginBottom: 16 }}>
          Plan not found.
        </div>
        <Button variant="ghost" size="md" onClick={() => router.push("/progress")}>
          ← Back to progress
        </Button>
      </div>
    );
  }

  const totalHours = plan.weeks
    .flatMap((w) => w.items)
    .reduce((sum, item) => sum + item.hours_est, 0);

  return (
    <div style={{ padding: "28px 36px 56px", maxWidth: 760 }}>
      <div style={{ marginBottom: 28 }}>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.3,
            textTransform: "uppercase",
            color: "var(--muted)",
            marginBottom: 6,
          }}
        >
          Study plan · {plan.weeks.length} weeks · {totalHours.toFixed(1)}h est.
        </div>
        <h1
          style={{
            margin: "0 0 8px",
            fontFamily: "var(--serif)",
            fontSize: 32,
            fontWeight: 500,
            letterSpacing: -0.5,
            color: "var(--text)",
          }}
        >
          {plan.goal}
        </h1>
        <div style={{ fontSize: 12, color: "var(--muted)" }}>
          Generated {new Date(plan.generated_at).toLocaleDateString()} · based on last {plan.lookback_days} days
        </div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {plan.weeks.map((week) => (
          <Card key={week.week_num} style={{ padding: 0 }}>
            <div
              style={{
                padding: "14px 20px",
                borderBottom: "1px solid var(--border)",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <div>
                <span
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 10,
                    letterSpacing: 1.2,
                    textTransform: "uppercase",
                    color: "var(--muted)",
                    marginRight: 10,
                  }}
                >
                  Week {week.week_num}
                </span>
                <span style={{ fontWeight: 600, fontSize: 14, color: "var(--text)" }}>
                  {week.focus}
                </span>
              </div>
              <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>
                {week.items.reduce((s, i) => s + i.hours_est, 0).toFixed(1)}h
              </span>
            </div>
            <div style={{ padding: "12px 20px", display: "flex", flexDirection: "column", gap: 8 }}>
              {week.items.map((item, idx) => (
                <div
                  key={idx}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "8px 12px",
                    background: "var(--surface-2)",
                    borderRadius: 4,
                  }}
                >
                  <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                    <span
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 9,
                        letterSpacing: 1,
                        textTransform: "uppercase",
                        color: "var(--accent)",
                        background: "var(--accent-dim)",
                        padding: "2px 6px",
                        borderRadius: 3,
                      }}
                    >
                      {item.kind}
                    </span>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)" }}>
                      {item.ref_id}
                    </span>
                  </div>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>
                    {item.hours_est}h
                  </span>
                </div>
              ))}
            </div>
          </Card>
        ))}
      </div>

      <div style={{ marginTop: 28 }}>
        <Button variant="ghost" size="md" onClick={() => router.push("/progress")}>
          ← Back to progress
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Type-check**

```bash
cd web && mise exec -- bun run type-check 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/app/\(learner\)/plans/\[id\]/page.tsx
git commit -m "feat: add study plan detail page at /plans/[id]"
```

---

## Task 4: Write learner simulation script

**Files:**
- Create: `scripts/simulate/learner.py`

- [ ] **Step 1: Create the script**

```python
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx"]
# ///
"""Learner role simulation — walks the full learner flow against a live API."""

import sys
import time
import httpx

BASE = "http://localhost:8080"
EMAIL = f"sim-learner-{int(time.time())}@example.com"
PASSWORD = "sim-password-123"

def step(label: str, ok: bool, detail: str = "") -> None:
    status = "✓" if ok else "✗"
    print(f"  {status} {label}" + (f": {detail}" if detail else ""))
    if not ok:
        sys.exit(1)

def main() -> None:
    print("=== Learner simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # 1. Register
    r = c.post("/v1/auth/register", json={"email": EMAIL, "password": PASSWORD, "displayName": "Sim Learner"})
    step("register", r.status_code == 201, str(r.status_code))
    token = r.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # 2. List quizzes
    r = c.get("/v1/quizzes", params={"status": "active"}, headers=headers)
    step("list quizzes", r.status_code == 200)
    quizzes = r.json().get("quizzes", [])
    step("at least one active quiz", len(quizzes) > 0, f"found {len(quizzes)}")
    quiz_id = quizzes[0]["id"]

    # 3. Start quiz session
    r = c.post("/v1/sessions", json={"quizId": quiz_id}, headers=headers)
    step("start quiz session", r.status_code == 200, str(r.status_code))
    session_id = r.json()["sessionId"]
    questions = r.json().get("questions", [])
    step("session has questions", len(questions) > 0, f"{len(questions)} questions")

    # 4. Answer all questions
    for q in questions:
        kind = q["kind"]
        if kind in ("mc", "mcq"):
            response = 0
        elif kind == "tf":
            response = True
        else:
            response = "simulation answer"
        r = c.post(f"/v1/sessions/{session_id}/answer",
                   json={"questionId": q["questionId"], "response": response},
                   headers=headers)
        step(f"answer {kind} question", r.status_code == 200, str(r.status_code))

    # 5. Finish quiz session
    r = c.post(f"/v1/sessions/{session_id}/finish", headers=headers)
    step("finish quiz session", r.status_code == 200, str(r.status_code))

    # 6. Fetch session results
    r = c.get(f"/v1/sessions/{session_id}", headers=headers)
    step("fetch session results", r.status_code == 200)
    result = r.json().get("session", {}).get("result", {})
    step("result has score", "points_awarded" in result, str(result.get("points_awarded")))

    # 7. List exams and start one
    r = c.get("/v1/exams", headers=headers)
    step("list exams", r.status_code == 200)
    exams = r.json().get("exams", [])
    step("at least one active exam", len(exams) > 0, f"found {len(exams)}")
    exam_id = next((e["id"] for e in exams if e.get("status") == "published"), None)
    step("published exam found", exam_id is not None)

    r = c.post("/v1/sessions", json={"examId": exam_id}, headers=headers)
    step("start exam session", r.status_code == 200, str(r.status_code))
    exam_session_id = r.json()["sessionId"]
    exam_questions = r.json().get("questions", [])
    step("exam session has questions", len(exam_questions) > 0, f"{len(exam_questions)} questions")

    # 8. Answer exam questions and finish
    for q in exam_questions:
        kind = q["kind"]
        if kind in ("mc", "mcq"):
            response = 0
        elif kind == "tf":
            response = True
        else:
            response = "simulation answer"
        r = c.post(f"/v1/sessions/{exam_session_id}/answer",
                   json={"questionId": q["questionId"], "response": response},
                   headers=headers)
        step(f"answer exam {kind}", r.status_code == 200, str(r.status_code))

    r = c.post(f"/v1/sessions/{exam_session_id}/finish", headers=headers)
    step("finish exam session", r.status_code == 200, str(r.status_code))

    # 9. Personal stats
    r = c.get("/v1/me/stats", headers=headers)
    step("fetch personal stats", r.status_code == 200)
    stats = r.json()
    step("stats has avg_score", "avg_score" in stats, str(stats.get("avg_score")))

    print("\n✓ Learner simulation complete")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run it (requires `make dev-env` + `make db-seed` in another terminal)**

```bash
uv run scripts/simulate/learner.py
```
Expected: all steps print `✓`, final line `✓ Learner simulation complete`.

- [ ] **Step 3: Commit**

```bash
git add scripts/simulate/learner.py
git commit -m "feat: add learner role API simulation script"
```

---

## Task 5: Write instructor simulation script

**Files:**
- Create: `scripts/simulate/instructor.py`

- [ ] **Step 1: Create the script**

```python
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx"]
# ///
"""Instructor role simulation — creates content, grades essays, checks stats."""

import sys
import time
import httpx

BASE = "http://localhost:8080"
INSTRUCTOR_EMAIL = "instructor@example.com"
INSTRUCTOR_PASSWORD = "password123"

def step(label: str, ok: bool, detail: str = "") -> None:
    status = "✓" if ok else "✗"
    print(f"  {status} {label}" + (f": {detail}" if detail else ""))
    if not ok:
        sys.exit(1)

def main() -> None:
    print("=== Instructor simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # 1. Login
    r = c.post("/v1/auth/login", json={"email": INSTRUCTOR_EMAIL, "password": INSTRUCTOR_PASSWORD})
    step("login as instructor", r.status_code == 200, str(r.status_code))
    token = r.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # 2. Create a quiz
    r = c.post("/v1/quizzes", json={"title": f"Sim Quiz {int(time.time())}", "status": "draft"}, headers=headers)
    step("create quiz", r.status_code in (200, 201), str(r.status_code))
    quiz_id = r.json()["id"]

    # 3. Add questions
    r = c.post("/v1/questions", json={"questions": [
        {"kind": "mc", "prompt": "Sim MC question?",
         "payload": {"options": [{"text": "A"}, {"text": "B"}, {"text": "C"}], "correct_index": 0},
         "explanation": "A is correct.", "tags": []},
        {"kind": "tf", "prompt": "Is simulation useful?",
         "payload": {"correct": True}, "tags": []},
        {"kind": "essay", "prompt": "Describe simulation in one sentence.",
         "payload": {"min_words": 5}, "tags": []},
    ]}, headers=headers)
    step("create questions", r.status_code in (200, 201), str(r.status_code))
    q_ids = [q["id"] for q in r.json()["questions"]]

    # 4. Add questions to quiz
    for qid in q_ids:
        r = c.post(f"/v1/quizzes/{quiz_id}/questions", json={"questionId": qid}, headers=headers)
        step(f"add question to quiz", r.status_code in (200, 201, 204), str(r.status_code))

    # 5. Publish quiz
    r = c.patch(f"/v1/quizzes/{quiz_id}", json={"status": "active"}, headers=headers)
    step("publish quiz", r.status_code in (200, 204), str(r.status_code))

    # 6. Compose exam from the quiz
    r = c.post("/v1/exams", json={
        "name": f"Sim Exam {int(time.time())}",
        "durationMin": 30,
        "sections": [{"title": "Section 1", "weight": 1, "questionIds": q_ids}],
    }, headers=headers)
    step("compose exam", r.status_code in (200, 201), str(r.status_code))
    exam_id = r.json()["examId"]

    # 7. Publish exam
    r = c.patch(f"/v1/exams/{exam_id}", json={"status": "published"}, headers=headers)
    step("publish exam", r.status_code in (200, 204), str(r.status_code))

    # 8. Quiz stats
    r = c.get(f"/v1/quizzes/{quiz_id}/stats", headers=headers)
    step("fetch quiz stats", r.status_code == 200, str(r.status_code))

    # 9. Pending essays (may be empty — just check endpoint)
    r = c.get("/v1/attempts/pending", headers=headers)
    step("list pending essays", r.status_code == 200, f"{len(r.json())} pending")

    print("\n✓ Instructor simulation complete")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run it**

```bash
uv run scripts/simulate/instructor.py
```
Expected: all steps `✓`.

- [ ] **Step 3: Commit**

```bash
git add scripts/simulate/instructor.py
git commit -m "feat: add instructor role API simulation script"
```

---

## Task 6: Write agent simulation script

**Files:**
- Create: `scripts/simulate/agent.py`

- [ ] **Step 1: Create the script**

```python
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx"]
# ///
"""Agent role simulation — registers an agent, manages knowledge base, generates quiz, fetches stats, creates plan."""

import sys
import time
import httpx

BASE = "http://localhost:8080"
INSTRUCTOR_EMAIL = "instructor@example.com"
INSTRUCTOR_PASSWORD = "password123"

def step(label: str, ok: bool, detail: str = "") -> None:
    status = "✓" if ok else "✗"
    print(f"  {status} {label}" + (f": {detail}" if detail else ""))
    if not ok:
        sys.exit(1)

def main() -> None:
    print("=== Agent simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # Bootstrap: need an instructor token to register the agent
    r = c.post("/v1/auth/login", json={"email": INSTRUCTOR_EMAIL, "password": INSTRUCTOR_PASSWORD})
    step("bootstrap instructor login", r.status_code == 200)
    inst_token = r.json()["token"]
    inst_headers = {"Authorization": f"Bearer {inst_token}"}

    # 1. Register agent
    r = c.post("/v1/agents/register",
               json={"label": f"sim-agent-{int(time.time())}", "scopes": ["quiz.read", "quiz.write", "stats.read", "plan.write", "plan.read"]},
               headers=inst_headers)
    step("register agent", r.status_code in (200, 201), str(r.status_code))
    agent_key = r.json().get("key") or r.json().get("apiKey")
    step("received API key", bool(agent_key))
    agent_headers = {"Authorization": f"Bearer {agent_key}"}

    # 2. Import questions
    r = c.post("/v1/questions", json={"questions": [
        {"kind": "mc", "prompt": "Which Rust keyword creates a new scope?",
         "payload": {"options": [{"text": "let"}, {"text": "fn"}, {"text": "mod"}, {"text": "use"}], "correct_index": 2},
         "explanation": "`mod` declares a module, creating a new scope.",
         "tags": ["rust", "modules"]},
        {"kind": "tf", "prompt": "Rust guarantees memory safety at compile time.",
         "payload": {"correct": True},
         "explanation": "The borrow checker enforces this at compile time.",
         "tags": ["rust", "memory"]},
        {"kind": "short", "prompt": "What does `&mut` mean in Rust?",
         "payload": {"accepted": ["mutable reference", "mutable borrow"], "normalize": "case_insensitive_strip_accents"},
         "tags": ["rust", "borrowing"]},
    ]}, headers=agent_headers)
    step("import questions", r.status_code in (200, 201), str(r.status_code))
    q_ids = [q["id"] for q in r.json()["questions"]]
    step("3 questions created", len(q_ids) == 3, str(len(q_ids)))

    # 3. Promote questions to live
    for qid in q_ids:
        r = c.post(f"/v1/questions/{qid}/promote", headers=agent_headers)
        step(f"promote question", r.status_code in (200, 204), str(r.status_code))

    # 4. List questions
    r = c.get("/v1/questions", params={"status": "live", "tag": "rust"}, headers=agent_headers)
    step("list live rust questions", r.status_code == 200)
    step("at least 3 questions in bank", len(r.json()["questions"]) >= 3,
         str(len(r.json()["questions"])))

    # 5. Generate quiz
    r = c.post("/v1/quizzes/generate", json={
        "source": "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It achieves memory safety through its ownership system.",
        "questionCount": 3,
        "types": ["mc", "tf"],
        "difficulty": "inter",
    }, headers=agent_headers)
    step("generate quiz", r.status_code in (200, 201), str(r.status_code))
    gen_quiz_id = r.json().get("id") or r.json().get("quizId")
    step("quiz id returned", bool(gen_quiz_id))

    # 6. Fetch user stats (as instructor, since agent stats may be empty)
    r = c.get("/v1/me/stats", headers=inst_headers)
    step("fetch user stats", r.status_code == 200)
    step("stats has avg_score field", "avg_score" in r.json())

    # 7. Create study plan
    r = c.post("/v1/plans", json={"goal": "Master Rust ownership and borrowing in 4 weeks", "lookbackDays": 30},
               headers=inst_headers)
    step("create study plan", r.status_code in (200, 201), str(r.status_code))
    plan_id = r.json()["id"]
    step("plan id returned", bool(plan_id))

    # 8. Fetch study plan
    r = c.get(f"/v1/plans/{plan_id}", headers=inst_headers)
    step("fetch study plan", r.status_code == 200)
    plan = r.json()
    step("plan has weeks", len(plan.get("weeks", [])) > 0, f"{len(plan.get('weeks', []))} weeks")

    # 9. Activity log
    r = c.get("/v1/agents/activity", headers=agent_headers)
    step("fetch agent activity", r.status_code == 200)

    print("\n✓ Agent simulation complete")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run it**

```bash
uv run scripts/simulate/agent.py
```
Expected: all steps `✓`.

- [ ] **Step 3: Commit**

```bash
git add scripts/simulate/agent.py
git commit -m "feat: add agent role API simulation script"
```

---

## Task 7: Add `make simulate` target

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Add the target and update `.PHONY`**

Find the `.PHONY` line at the top of the Makefile and add `simulate` to it. Then find a logical location (after `db-seed`) and add:

```makefile
simulate: ## Run all three role simulation scripts against local API (requires make dev-env + make db-seed)
	uv run scripts/simulate/learner.py
	uv run scripts/simulate/instructor.py
	uv run scripts/simulate/agent.py
```

- [ ] **Step 2: Verify help output**

```bash
make help | grep simulate
```
Expected: `simulate         Run all three role simulation scripts...`

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore: add make simulate target for role simulation scripts"
```

---

## Task 8: Write learner Playwright spec

**Files:**
- Create: `web/e2e/roles/learner.spec.ts`

Check existing e2e specs in `web/e2e/` for auth helpers and base URL patterns before writing.

- [ ] **Step 1: Check existing e2e patterns**

```bash
ls web/e2e/
head -40 web/e2e/*.spec.ts 2>/dev/null | head -80
```

- [ ] **Step 2: Create the spec**

```typescript
// web/e2e/roles/learner.spec.ts
import { test, expect } from "@playwright/test";

const BASE = process.env.E2E_BASE_URL ?? "http://localhost:3000";
const API = process.env.E2E_API_URL ?? "http://localhost:8080";

test.describe("Learner role", () => {
  let token: string;
  let sessionId: string;

  test.beforeAll(async ({ request }) => {
    const ts = Date.now();
    const r = await request.post(`${API}/v1/auth/register`, {
      data: { email: `e2e-learner-${ts}@example.com`, password: "e2e-password-123", displayName: "E2E Learner" },
    });
    expect(r.status()).toBe(201);
    token = (await r.json()).token;
  });

  test("library page loads quizzes", async ({ page }) => {
    await page.goto(`${BASE}/login`);
    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/library`);
    await expect(page.locator("h1, h2")).toBeVisible();
  });

  test("can start a quiz session and answer questions", async ({ page, request }) => {
    // Get an active quiz via API
    const r = await request.get(`${API}/v1/quizzes?status=active`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    const quizzes = (await r.json()).quizzes ?? [];
    expect(quizzes.length).toBeGreaterThan(0);

    // Start session via API, then navigate to UI
    const sr = await request.post(`${API}/v1/sessions`, {
      data: { quizId: quizzes[0].id },
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(sr.status()).toBe(200);
    sessionId = (await sr.json()).sessionId;

    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/sessions/${sessionId}`);
    await expect(page.locator("text=/question/i").first()).toBeVisible({ timeout: 8000 });
  });

  test("session results page shows score", async ({ page, request }) => {
    // Finish the session via API
    const questions = await request
      .get(`${API}/v1/sessions/${sessionId}`, { headers: { Authorization: `Bearer ${token}` } })
      .then((r) => r.json())
      .then((d) => d.questions ?? []);

    for (const q of questions) {
      const response = q.kind === "mc" || q.kind === "mcq" ? 0 : q.kind === "tf" ? true : "e2e answer";
      await request.post(`${API}/v1/sessions/${sessionId}/answer`, {
        data: { questionId: q.questionId, response },
        headers: { Authorization: `Bearer ${token}` },
      });
    }
    await request.post(`${API}/v1/sessions/${sessionId}/finish`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/sessions/${sessionId}/results`);
    await expect(page.locator("text=/%|correct|score/i").first()).toBeVisible({ timeout: 8000 });
  });

  test("progress page renders stats", async ({ page }) => {
    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/progress`);
    await expect(page.locator("text=/progress|avg score|streak/i").first()).toBeVisible({ timeout: 8000 });
  });
});
```

- [ ] **Step 3: Run the spec**

```bash
cd web && E2E_API_TOKEN=unused E2E_BASE_URL=http://localhost:3000 \
  mise exec -- bunx @playwright/test test e2e/roles/learner.spec.ts --project=chromium 2>&1 | tail -20
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add web/e2e/roles/learner.spec.ts
git commit -m "test: add learner role Playwright spec"
```

---

## Task 9: Write instructor Playwright spec

**Files:**
- Create: `web/e2e/roles/instructor.spec.ts`

- [ ] **Step 1: Create the spec**

```typescript
// web/e2e/roles/instructor.spec.ts
import { test, expect } from "@playwright/test";

const BASE = process.env.E2E_BASE_URL ?? "http://localhost:3000";
const API = process.env.E2E_API_URL ?? "http://localhost:8080";

async function instructorToken(request: Parameters<Parameters<typeof test>[1]>[0]["request"]) {
  const r = await request.post(`${API}/v1/auth/login`, {
    data: { email: "instructor@example.com", password: "password123" },
  });
  return (await r.json()).token as string;
}

test.describe("Instructor role", () => {
  test("grading page accessible and shows queue", async ({ page, request }) => {
    const token = await instructorToken(request);
    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/grading`);
    await expect(
      page.locator("text=/essay grading|no essays/i").first()
    ).toBeVisible({ timeout: 8000 });
  });

  test("exams page accessible and shows compose button", async ({ page, request }) => {
    const token = await instructorToken(request);
    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/exams`);
    await expect(
      page.locator("text=/compose exam|\\+ compose/i").first()
    ).toBeVisible({ timeout: 8000 });
  });

  test("agent page accessible and shows API keys tab", async ({ page, request }) => {
    const token = await instructorToken(request);
    await page.evaluate((t) => {
      document.cookie = `ame_token=${t}; path=/`;
    }, token);
    await page.goto(`${BASE}/agent`);
    await expect(
      page.locator("text=/api key|keys/i").first()
    ).toBeVisible({ timeout: 8000 });
  });
});
```

- [ ] **Step 2: Run it**

```bash
cd web && E2E_API_TOKEN=unused E2E_BASE_URL=http://localhost:3000 \
  mise exec -- bunx @playwright/test test e2e/roles/instructor.spec.ts --project=chromium 2>&1 | tail -20
```
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/roles/instructor.spec.ts
git commit -m "test: add instructor role Playwright spec"
```

---

## Task 10: Write agent API Playwright spec

**Files:**
- Create: `web/e2e/roles/agent-api.spec.ts`

This spec uses Playwright's `request` fixture only (no browser).

- [ ] **Step 1: Create the spec**

```typescript
// web/e2e/roles/agent-api.spec.ts
import { test, expect } from "@playwright/test";

const API = process.env.E2E_API_URL ?? "http://localhost:8080";

test.describe("Agent API surface", () => {
  let agentKey: string;
  let instructorToken: string;

  test.beforeAll(async ({ request }) => {
    const r = await request.post(`${API}/v1/auth/login`, {
      data: { email: "instructor@example.com", password: "password123" },
    });
    expect(r.status()).toBe(200);
    instructorToken = (await r.json()).token;

    const ar = await request.post(`${API}/v1/agents/register`, {
      data: { label: "e2e-agent", scopes: ["quiz.read", "quiz.write", "stats.read", "plan.write", "plan.read"] },
      headers: { Authorization: `Bearer ${instructorToken}` },
    });
    expect(ar.status()).toBeOneOf([200, 201]);
    const body = await ar.json();
    agentKey = body.key ?? body.apiKey;
    expect(agentKey).toBeTruthy();
  });

  test("MCP manifest contains question and stats tools", async ({ request }) => {
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
          { kind: "mc", prompt: "e2e agent question?",
            payload: { options: [{ text: "A" }, { text: "B" }], correct_index: 0 }, tags: [] },
        ],
      },
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect(r.status()).toBeOneOf([200, 201]);
    const qid = (await r.json()).questions[0].id;

    const pr = await request.post(`${API}/v1/questions/${qid}/promote`, {
      headers: { Authorization: `Bearer ${agentKey}` },
    });
    expect(pr.status()).toBeOneOf([200, 204]);
  });

  test("can fetch user stats", async ({ request }) => {
    const r = await request.get(`${API}/v1/me/stats`, {
      headers: { Authorization: `Bearer ${instructorToken}` },
    });
    expect(r.status()).toBe(200);
    const stats = await r.json();
    expect(stats).toHaveProperty("avg_score");
    expect(stats).toHaveProperty("current_streak");
  });

  test("can create and retrieve a study plan", async ({ request }) => {
    const cr = await request.post(`${API}/v1/plans`, {
      data: { goal: "e2e plan goal", lookbackDays: 7 },
      headers: { Authorization: `Bearer ${instructorToken}` },
    });
    expect(cr.status()).toBeOneOf([200, 201]);
    const planId = (await cr.json()).id;

    const gr = await request.get(`${API}/v1/plans/${planId}`, {
      headers: { Authorization: `Bearer ${instructorToken}` },
    });
    expect(gr.status()).toBe(200);
    const plan = await gr.json();
    expect(plan.weeks).toBeDefined();
    expect(plan.weeks.length).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Run it**

```bash
cd web && E2E_API_TOKEN=unused E2E_BASE_URL=http://localhost:3000 \
  mise exec -- bunx @playwright/test test e2e/roles/agent-api.spec.ts --project=chromium 2>&1 | tail -20
```
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/roles/agent-api.spec.ts
git commit -m "test: add agent API Playwright spec"
```

---

## Task 11: Write MCP prompts doc and commit spec

**Files:**
- Create: `docs/specs/mcp-prompts.md`

- [ ] **Step 1: Create the prompts doc**

```markdown
# MCP Role Prompts

Sample prompts for validating ame's MCP surface in a live Claude session.
Prerequisite: ame running at http://localhost:8080 with MCP configured.

## Setup

Add to Claude's MCP config:
```json
{
  "mcpServers": {
    "ame": {
      "url": "http://localhost:8080/v1/agents/mcp.json",
      "headers": { "Authorization": "Bearer <your-api-key>" }
    }
  }
}
```

## Learner prompt

> "List the available quizzes, start a session on the first active one, answer each question (pick a reasonable answer for each type), finish the session, and tell me my final score."

Expected: Claude calls `quiz.list` → `session.create` → `session.answer` × N → `session.finish` → `session.get` and reports score.

## Instructor prompt

> "Create a 3-question multiple-choice quiz on Rust ownership, add an explanation to each question, publish it, then show me its stats."

Expected: Claude calls `question.create` → `quiz.import` (or `quiz.update`) → `quiz.update` (status=active) → `stats.cohort`.

## Agent knowledge-base prompt

> "Import these 3 questions into the bank, promote them to live, then generate a 10-question quiz from this source text: 'Rust uses an ownership model to manage memory without a garbage collector. Each value has a single owner. When the owner goes out of scope, the value is dropped.' Finally, create a study plan with the goal: improve Rust memory management in 3 weeks."

Expected: Claude calls `question.create` → `question.promote` × 3 → `quiz.generate` → `plan.create`.

## Stats prompt

> "Fetch my personal learning stats for the last 12 weeks and summarize my progress."

Expected: Claude calls `stats.user` with `window=12w` and returns a human-readable summary.
```

- [ ] **Step 2: Commit everything**

```bash
git add docs/specs/mcp-prompts.md
git commit -m "docs: add MCP role prompts for manual validation"
```

---

## Verification

Full end-to-end check after all tasks:

```bash
# 1. API scripts
make simulate
# Expected: all three scripts complete with ✓

# 2. Playwright role specs (requires make dev-env running)
cd web && E2E_API_TOKEN=unused E2E_BASE_URL=http://localhost:3000 \
  mise exec -- bunx @playwright/test test e2e/roles/ --project=chromium
# Expected: all tests pass

# 3. MCP manifest tool count
curl -s http://localhost:8080/v1/agents/mcp.json | jq '[.tools[].name] | length'
# Expected: number increased by 5 vs before (question.list/create/update/promote + stats.user)

# 4. Full pre-PR gate
make validate
# Expected: exits 0
```
