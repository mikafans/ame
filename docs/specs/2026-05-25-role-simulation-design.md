# Role Simulation Design

**Status:** Approved  
**Date:** 2026-05-25

---

## Goal

Verify the platform end-to-end for two target flows — learner (quiz + exam + results) and agent (knowledge base management + stats) — using three simulation layers that each catch a different class of bug.

---

## MVP gap summary (as of 2026-05-25)

These gaps were identified from reading the actual code, not the spec.

| Gap | File | Fix needed |
|-----|------|-----------|
| No question tools in MCP manifest | `api/src/http/agents.rs` | Add `question.list`, `question.create`, `question.update`, `question.promote` |
| No `stats.user` MCP tool | `api/src/http/agents.rs` | Add tool pointing to `GET /v1/me/stats` |
| Study plan never displayed | `web/app/(learner)/sessions/[id]/results/page.tsx:586` | "View 6-week plan" links to `/progress`; needs `/plans/[id]` page |
| Explanations are a placeholder | `web/app/(learner)/sessions/[id]/results/page.tsx:515` | `"Explanation would appear here."` — data exists in API response |
| Exam session untested | — | No code change needed; just needs a run-through |

---

## Three simulation layers

### Layer 1 — API scripts (`scripts/simulate/`)

Python scripts (PEP 723, `uv run`) that call the REST API directly. Each script walks one role's full flow and prints pass/fail per step.

**`learner.py`**
1. Register new learner account
2. List quizzes, pick first active one
3. Start quiz session → `/v1/sessions`
4. Answer all questions
5. Finish session
6. Fetch session results, assert score present
7. Start exam session → `/v1/sessions { examId }`
8. Answer all questions, finish
9. Fetch results
10. Fetch personal stats → `/v1/me/stats`

**`instructor.py`**
1. Login as instructor
2. Create quiz with 3 questions (mc + tf + essay)
3. Publish quiz
4. Compose exam from that quiz
5. Publish exam
6. Fetch quiz stats → `/v1/quizzes/{id}/stats`
7. Fetch pending essays → `/v1/attempts/pending`
8. Grade one essay → `PATCH /v1/attempts/{id}/grade`

**`agent.py`**
1. Register agent → `/v1/agents/register`
2. Import 5 questions → `POST /v1/questions`
3. Promote one question → `POST /v1/questions/{id}/promote`
4. List questions → `GET /v1/questions`
5. Generate quiz from text → `POST /v1/quizzes/generate`
6. Fetch user stats → `GET /v1/me/stats`
7. Create study plan → `POST /v1/plans`
8. Fetch plan → `GET /v1/plans/{id}`

Add `make simulate` target that runs all three scripts in sequence.

---

### Layer 2 — MCP session (Claude wired to manifest)

**Prerequisite:** MCP gap fixes must land first (question tools + stats.user).

Setup: add ame as an MCP server pointing at `http://localhost:8080`. Claude then walks each role's flow in a conversation using the registered tools.

Each role gets a sample prompt in `docs/specs/mcp-prompts.md`:
- Learner: "List available quizzes, start a session on the first one, answer each question, finish the session, and tell me my score."
- Instructor: "Create a 5-question multiple-choice quiz on Rust ownership, publish it, then show me its stats."
- Agent: "Import these 3 questions into the bank, promote them to live, then generate a 10-question quiz from this source text: ..."

This layer surfaces MCP schema mismatches, missing required fields, and tool description clarity issues.

---

### Layer 3 — Playwright role specs (`web/e2e/roles/`)

Browser-driven specs that simulate a real user clicking through the UI. Extend the existing Playwright setup.

**`learner.spec.ts`**
- Login as learner
- Library page loads, quizzes visible
- Click "Start" on a quiz → session page loads with questions
- Answer all questions → finish → results page shows score and per-question breakdown
- Explanations visible on each answered question
- Navigate to Progress → stats render (not empty)
- Navigate to Exams → start an exam session → finish → results

**`instructor.spec.ts`**
- Login as instructor
- Author page: create quiz, add mc + essay question, publish
- Grading page: essay appears in queue, grade it
- Exams page: compose exam, publish

**`agent-api.spec.ts`** (API-only, no browser)
- Uses a registered API key
- Import questions, generate quiz, fetch stats, create plan
- Asserts response shapes match expected schema

---

## Dependency order

1. Fix MCP gaps (`agents.rs`) — unblocks Layer 2 and agent.py
2. Fix explanation display in results page — unblocks learner.spec.ts assertion
3. Add study plan page (`/plans/[id]`) — unblocks plan step in learner.py and Playwright
4. Write Layer 1 scripts
5. Write Layer 3 Playwright specs
6. Wire Layer 2 MCP prompts doc + validate
7. Add `make simulate` target

---

## Success criteria

- `make simulate` exits 0: all three API scripts complete without assertion errors
- `make validate` (existing gate) includes the new role Playwright specs and passes
- A Claude MCP session can complete the instructor role prompt end-to-end without tool errors
