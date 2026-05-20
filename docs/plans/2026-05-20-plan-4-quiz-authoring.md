# Plan 4 (2026-05-20): Quiz authoring

**Goal:** Quizzes table + ordered join with questions + full CRUD + publish gating + import/generate routes. New phase in the 10-phase plan — previously fused into May-19 P3.

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §4.6 (Quiz), §6.1 quiz surface, §9.7 (Author studio screen — UI lives in Plan 10).

**Prerequisites:** Plan 3 (question bank with all 5 kinds + points).

**Out of scope:** Author studio UI (Plan 10), exam composition (Plan 6), grading (Plan 5).

---

## Task 1: Schema (already migrated in Plan 2, but Plan 4 owns the join)

- [ ] **Step 1:** Confirm `quizzes` columns: `id, title, course_id, author_id, difficulty, duration_min, objectives text[], status enum, created_at, updated_at`. Status enum: `draft | active | archived` (one-way except archived is terminal — spec §5.2).
- [ ] **Step 2:** `quiz_questions` join — `quiz_id, question_id, order_index int, points_override int?` (per-quiz override of question's default points).
- [ ] **Step 3:** Unique `(quiz_id, order_index)` and `(quiz_id, question_id)`.

## Task 2: Repository layer

- [ ] **Step 1: `api/src/quiz/mod.rs`.** Exposes `QuizRepository` with typed methods. Module rule: `quiz` may depend on `bank` and `domain`; not vice versa.
- [ ] **Step 2: Methods.** `create_quiz`, `get_quiz`, `list_quizzes`, `update_quiz`, `add_question`, `remove_question`, `reorder`, `set_status`.
- [ ] **Step 3: Status transition guard.** Repo method, not a trigger. `draft → active → archived`; `draft → scheduled → active → archived`. Anything else is `ApiError::Validation`.
- [ ] **Step 4: Publish gating.** `set_status(active)` rejects if `quiz_questions.count == 0` or any included question has `status != active`.

## Task 3: HTTP routes (spec §6.1 quiz surface)

| Method | Path                | Tool            | Scope        |
| ------ | ------------------- | --------------- | ------------ |
| GET    | `/quizzes`          | `quiz.list`     | `quiz.read`  |
| POST   | `/quizzes`          | `quiz.import`   | `quiz.write` |
| POST   | `/quizzes/generate` | `quiz.generate` | `quiz.write` |
| GET    | `/quizzes/{id}`     | `quiz.get`      | `quiz.read`  |
| PATCH  | `/quizzes/{id}`     | `quiz.update`   | `quiz.write` |

- [ ] **Step 1:** Idempotency middleware attached to all POST/PATCH routes.
- [ ] **Step 2:** `POST /quizzes` body: `{ source, format: 'json' | 'md', courseId? }`. Returns `{ quizId, questionCount, warnings[] }`. Soft-warn if `objectives.length > 6` (spec §5.2).
- [ ] **Step 3:** `POST /quizzes/generate` body: `{ source, questionCount, types?, difficulty?, objectives?: string[] }`. Agent SHOULD emit 3-4 objectives best-effort; if source material is insufficient, emit a `warnings[]` entry rather than failing (spec §4.6).
- [ ] **Step 4:** OpenAPI snapshot updated.

## Task 4: Objectives (P8 sees this via MCP tools)

- [ ] **Step 1:** `objectives` is `text[]` in DB; round-trips as `string[]` in JSON.
- [ ] **Step 2:** No hard limit; soft-warn at `> 6`. Empty array is legal.
- [ ] **Step 3:** Editor UI lands in Plan 10 §9.7; backend just stores + roundtrips here.

## Task 5: Tests

- [ ] Create quiz with 5 questions, all 5 kinds — read back, verify order_index.
- [ ] Publish gating: empty quiz rejected; quiz with draft question rejected.
- [ ] Status transition: `active → draft` rejected; `archived → *` rejected.
- [ ] Import JSON + Markdown — both parse to the same row count.
- [ ] `objectives.length > 6` returns `warnings[]` not 422.
- [ ] OpenAPI snapshot drift test passes.

## Definition of done

- `make check` + `make test-quiz` pass.
- All P4 ledger rows are `done` in `docs/plans/tasks/plan-4-quiz-authoring.jsonl` (file to be created).
- Quiz module compiles without referencing engine, assess, or session types.
- A new quiz with all 5 question kinds can be created → published → archived end-to-end via HTTP.
