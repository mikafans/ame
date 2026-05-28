# UI/UX Contract Audit — 2026-05-25

## Context

Branch audited: `feat/quiz-sessions-squash`

Recent relevant commit:

- `1b9c3f5 test: add uiux spec coverage`

Local verification already performed before this audit:

- `rtk make check` passed.
- `cd web && E2E_BASE_URL=http://localhost:3000 mise exec -- bunx @playwright/test test e2e/uiux-spec.spec.ts --project=chromium` passed.
- Playwright screenshots were generated under `.tmp/uiux-*.png`.

The app is functional enough for a learner demo, but the main risk is contract drift between the Rust routes, OpenAPI export, generated TypeScript schema, and frontend page assumptions.

## Executive Summary

The project has solid pieces: DB-backed Rust routes, a seeded local stack, useful page-level UI, and meaningful Playwright coverage. However, it is not yet contract-stable. Several frontend flows work because pages bypass generated types with `as any` / `as never`, or because UI fallbacks fill missing product data. This makes the current UI look closer to the design than the API contract actually supports.

The next agent should treat this as a contract hardening pass, not a styling-only pass.

Recommended priority:

1. Complete OpenAPI export for implemented routes.
2. Remove `as any` / `as never` API call bypasses from learner-critical pages.
3. Fix broken session UX paths: save/abandon, manual grading display, result score math.
4. Decide which demo-only UI fields should become real API fields.
5. Keep the UIUX spec, but make it reset-seed friendly before CI.

## Findings

### 1. High — OpenAPI is not the real backend contract

Evidence:

- `api/src/http/quizzes.rs` implements:
  - `POST /v1/quizzes`
  - `GET /v1/quizzes/{id}`
  - `PATCH /v1/quizzes/{id}`
  - `POST /v1/quizzes/{id}/questions`
  - `GET /v1/quizzes/count`
  - `POST /v1/quizzes/generate`
- `api/src/http/sessions.rs` implements:
  - `PATCH /v1/sessions/{id}/answers`
  - `GET /v1/attempts/pending`
  - `PATCH /v1/attempts/{id}/grade`
- `api/src/http/openapi.rs` only includes `crate::http::quizzes::list_quizzes` for quizzes and omits many implemented route handlers.
- `web/src/api/generated/schema.d.ts` therefore marks real routes as nonexistent. For example, `/v1/quizzes` has `post?: never` even though the backend routes `post(create_quiz)`.

Impact:

- The generated client is not authoritative.
- Frontend pages are forced into casts such as `(makeClient(token) as any)` and route strings cast `as never`.
- `make openapi` and `bun run api:check` can pass while the frontend still depends on undocumented backend behavior.
- Agents relying on OpenAPI will miss available routes or call stale shapes.

Suggested fix:

- Add every implemented route handler to `ApiDoc::paths(...)` in `api/src/http/openapi.rs`.
- Add missing component schemas for route bodies/responses where needed.
- Run `make openapi`.
- Replace page-level `as any` / `as never` calls with generated-client typed calls.
- Add a lint or review rule: no new frontend API calls may bypass generated types unless a short comment links to an explicit OpenAPI gap task.

Suggested verification:

- `make openapi`
- `cd web && mise exec -- bun run api:check`
- `rtk make check`
- UIUX spec

### 2. High — Results page misrepresents manual grading and score math

Evidence:

- Backend uses `grade_status = 'pending_manual'` for essay/manual-review attempts.
- `web/app/(learner)/sessions/[id]/results/page.tsx` checks `attempt?.grade_status === "pending"`.
- The same page computes per-item points with `Math.round(attempt.score)`, but backend `attempt.score` is a ratio/fraction, not awarded points. The result summary itself uses `session.result.points_awarded` and `max_points`, but the item rows are not using the item max correctly.

Impact:

- Essay/manual-review items can display as ordinary failed or empty items instead of pending review.
- Per-item score rows can show wrong awarded points.
- Learners see misleading assessment feedback.
- The current UIUX spec checks that the Results page renders, but it does not assert pending-manual semantics or per-item score math.

Suggested fix:

- Change status check to `pending_manual`.
- Compute per-item awarded points as `Math.round(attempt.score * q.points)` or preferably expose awarded/max from API.
- Preserve the question prompt separately from the answer. The page currently uses `answer.given.substring(...)` as the visible prompt fallback, which is not a real question prompt.
- Add a focused Playwright/API-backed result test with at least one essay attempt.

Suggested verification:

- Add/adjust DB-backed session test for essay pending manual result shape.
- Add UI test assertion for `Pending manual review`.
- Run UIUX spec.

### 3. High — “Save & exit” active-session path calls an unimplemented route

Evidence:

- `web/app/(learner)/sessions/[id]/page.tsx` calls `PATCH /v1/sessions/{id}` with `{ status: "abandoned" }`.
- `api/src/http/sessions.rs` routes only:
  - `GET /v1/sessions/{id}`
  - `POST /v1/sessions/{id}/answer`
  - `PATCH /v1/sessions/{id}/answers`
  - `POST /v1/sessions/{id}/finish`
- There is no `PATCH /v1/sessions/{id}` route.

Impact:

- The visible `Save & exit` action fails.
- Learners cannot intentionally abandon/save a session through the UI.
- This was not caught by the UIUX spec because it does not click `Save & exit`.

Suggested fix options:

- Backend-first: implement `PATCH /v1/sessions/{id}` with allowed transition to `abandoned`, ownership check, and OpenAPI annotation.
- Frontend-short-term: if no abandon behavior is wanted yet, change the button to navigate back to Library without pretending to persist state.

Suggested verification:

- API test for abandoning own session.
- UI test clicking `Save & exit` and asserting redirect/no console error.
- OpenAPI generation.

### 4. Medium — Demo fallbacks hide missing product fields

Evidence:

Recent UI alignment added design-grade fallbacks:

- Library due date falls back to `Due in 2 days`.
- Library duration is estimated from question count.
- Attempts show `0 / 2`.
- Recommended prep is hardcoded.
- Active session timer is inferred from question count when no `deadline_at` exists.
- Allowed materials fall back to a fixed list.

Impact:

- The UI now matches the design better for screenshots, but product truth is not always coming from API data.
- Agents or future developers may assume these are real persisted values.
- Production behavior will diverge when real course/assignment scheduling is introduced.

Suggested fix:

- Decide whether each field is product data or demo-only:
  - `dueAt` / assignment due date
  - `durationMin`
  - attempt limits
  - recommended prep resources
  - allowed materials
- If product data: add DB/API fields and return them from list/detail/session routes.
- If demo-only: gate behind explicit demo fixtures or comments so production code does not silently fabricate them.

Suggested verification:

- Typed schema includes the fields.
- UI uses API-provided values when present.
- Tests assert fallback behavior only where intentionally demo-mode.

### 5. Medium — UIUX spec is useful but not a visual regression test

Evidence:

- `web/e2e/uiux-spec.spec.ts` verifies presence of key text/controls and no fatal console errors.
- It takes screenshots under `.tmp/uiux-*.png`.
- It does not compare screenshots to baselines.
- It mutates backend state by creating/finishing a session.

Impact:

- Good smoke coverage for broken routes and missing design-critical controls.
- Does not catch layout drift, visual spacing/color regressions, overlap, or mobile issues.
- Running repeatedly accumulates session/attempt state unless the DB is reset or seed is controlled.

Suggested fix:

- Keep the spec as a fast audit test.
- Add a reset/seed fixture path for deterministic CI.
- Consider separate snapshot/visual tests after layout stabilizes.
- Add mobile viewport coverage for Library, Active Session, Results.

Suggested verification:

- `make db-reset`
- `uv run --script scripts/seed.py`
- UIUX spec
- Optional: Playwright screenshot diff with explicit baseline directory.

### 6. Medium — Frontend API code is type-eroded

Evidence:

- Many pages use `(makeClient(token) as any)` or route casts `as never`.
- Examples include Library, Practice, Progress, Author, Agent, Sessions, Results.
- This is downstream of incomplete OpenAPI export, but it is now an independent maintenance risk.

Impact:

- TypeScript cannot protect request/response shape changes.
- Bugs like `session_id` vs `sessionId` can compile.
- Runtime data bugs become page-level surprises instead of compile failures.

Suggested fix:

- Fix OpenAPI first.
- Then remove casts page by page, starting with learner-critical routes:
  - Library
  - Quiz Preview
  - Active Session
  - Results
  - Progress
  - Exams
- Keep one small compatibility helper if API migration needs a temporary dual-shape bridge.

### 7. Low/Medium — Current branch is ahead of origin and docs/session state are stale

Evidence:

- Branch `feat/quiz-sessions-squash` is ahead of origin by one commit.
- `.agents/CURRENT_TASK.md` previously described old in-progress state even after the branch was clean and rosemary marked the work done.

Impact:

- Handoff agents may read stale `.agents` files and choose the wrong next task.
- CI/PR reviewers will not see the latest UIUX spec until pushed.

Suggested fix:

- Push the branch when ready.
- Update session state at wrap-up using rosemary.
- Prefer task ledgers and current git state over stale `.agents/CURRENT_TASK.md`.

## Suggested Next Task Breakdown

### Task A — OpenAPI completion

Scope:

- `api/src/http/openapi.rs`
- `api/src/http/quizzes.rs`
- `api/src/http/sessions.rs`
- `api/src/http/auth.rs`
- `api/src/http/me.rs`
- `api/src/http/admin.rs`
- `api/src/http/shares.rs`
- `api/src/http/agents.rs`
- `web/src/api/generated/schema.d.ts`

Acceptance:

- All implemented routes appear in `api/openapi.yaml`.
- `web/src/api/generated/schema.d.ts` reflects those routes.
- Existing frontend calls can be typed without `as any` for learner-critical flows.
- `make openapi` and `rtk make check` pass.

### Task B — Session lifecycle UX fix

Scope:

- `api/src/http/sessions.rs`
- `api/tests/sessions.rs`
- `web/app/(learner)/sessions/[id]/page.tsx`
- OpenAPI/schema regeneration.

Acceptance:

- `Save & exit` succeeds.
- Session status transitions to `abandoned` or intentionally resumes without a broken API call.
- UI test covers the button.

### Task C — Results correctness fix

Scope:

- `web/app/(learner)/sessions/[id]/results/page.tsx`
- `api/src/http/sessions.rs` if response enrichment is preferred.
- `api/tests/sessions.rs`
- `web/e2e/uiux-spec.spec.ts`

Acceptance:

- Manual essays show pending manual review.
- Per-question awarded points are correct.
- Results page displays question prompts, not answer bodies.
- UIUX spec or a focused results spec catches these semantics.

### Task D — Demo fallbacks audit

Scope:

- `web/app/(learner)/library/page.tsx`
- `web/app/(learner)/sessions/[id]/page.tsx`
- DB/API schema if fields become product-backed.

Acceptance:

- Every fallback is either API-backed, explicitly demo-mode, or removed.
- No production page silently invents due dates, attempt limits, prep resources, or allowed materials without a clear source.

## Commands Used During Audit

```bash
rtk make check
cd web && E2E_BASE_URL=http://localhost:3000 mise exec -- bunx @playwright/test test e2e/uiux-spec.spec.ts --project=chromium
rtk rg -n "TODO|FIXME|as any|as never|catch\\(console\\.error\\)|session_id|deadline_at|allowed_materials|grade_status|pending" web/app web/src api/src api/tests web/e2e
rtk rg -n "create_quiz|get_quiz|add_quiz_question|count_quizzes|patch_quiz|generate_quiz|CreateSessionResponse|path = \\\"/v1/quizzes" api/src/http/quizzes.rs api/src/http/openapi.rs api/openapi.yaml web/src/api/generated/schema.d.ts
```

## Final Assessment

This is a strong prototype with a credible backend and useful UI, but its biggest risk is hidden contract drift. The fastest path to product-grade reliability is not more visual polish; it is making OpenAPI complete, making the generated client authoritative, and removing page-level casts/fallbacks that hide mismatches.

