# Plan 3 (2026-05-20): Question bank (5 kinds: mc, tf, short, essay, code)

**Goal:** Land CRUD + payload validators for all five question kinds in the new spec, plus per-question `points`. Extends May-19 P3 (which covered mc, free-text, cloze under the older schema).

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §4.5 (Question), §6.1 quiz surface (note: `POST /questions` lives here, not in Plan 4).

**Prerequisites:** Plan 2 schema landed (specifically `questions.points` from P2-012; the `questions` table itself was created in May-19 P3 and migrated forward in P2).

**Module rule (spec §3 / AGENTS.md):** the `bank` module MUST NOT depend on `engine`, `assess`, or attempt/session types. Bank knows about Question and Tag only.

---

## Task 1: Payload kinds (spec §4.5)

- [ ] **Step 1: Domain types.** Replace the May-19 `McqPayload | FreeTextPayload | ClozePayload` triad with the spec's 5 kinds:
  - `mc` — multiple choice. `{ options: [{id, text}], correct: string | string[], shuffle: bool }`.
  - `tf` — true/false. `{ correct: bool }`.
  - `short` — short answer. `{ answers: string[], case_sensitive: bool, max_length: int }`.
  - `essay` — essay. `{ min_words: int, rubric?: string }`.
  - `code` — code. `{ language: string, starter: string, exemplar: string, tests?: string }`.
- [ ] **Step 2: Discriminator.** `QuestionKind` enum stays the discriminator on the `kind` column. `payload` jsonb is validated at the HTTP boundary via `validate_payload(kind, payload)`; no DB-level json schema check (spec §5.2 line 185).
- [ ] **Step 3: Backwards compat.** Existing rows under May-19 `cloze` payloads need a migration path — recommended: move `cloze` to `short` with `answers[]` extracted from the cloze blanks. Document the mapping in the migration.

## Task 2: Points (P3-008)

- [ ] **Step 1:** Add `points` to `QuestionInsert` / `QuestionPatch`.
- [ ] **Step 2:** `POST /questions` and `PATCH /questions/{id}` accept `points` (default 1).
- [ ] **Step 3:** Validator rejects `points < 0`. `points == 0` is legal (ungraded/practice items).
- [ ] **Step 4:** OpenAPI snapshot updated; drift test still passes.

## Task 3: Per-kind validators

- [ ] **Step 1:** A `validate_payload(kind, payload) -> Result<TypedPayload, ApiError::Validation>` function. Each kind has its own typed validator with serde + post-deserialize checks.
- [ ] **Step 2:** `mc` validator: at least 2 options; `correct` references existing option ids; if `correct` is `string[]` then `multiple_select` mode (front-end concern, but the model accepts it).
- [ ] **Step 3:** `code` validator: `language` is one of the supported list (`rust, python, js, ts, go` — extend over time); `starter` and `exemplar` must compile-format-only-check pass (defer real eval to P5 graders).
- [ ] **Step 4:** Error shape: `ApiError::Validation { field, reason }`.

## Task 4: CRUD routes (May-19 carry-over + extensions)

- [ ] **Step 1:** Existing routes (list/get/batch-create/patch/promote/archive) — unchanged.
- [ ] **Step 2:** Versioning rule (live-vs-draft) — unchanged from May-19 P3-003.
- [ ] **Step 3:** Tag routes — unchanged.
- [ ] **Step 4:** OpenAPI snapshot regenerated.

## Task 5: Tests

- [ ] One golden-path test per kind: insert + read back + validate payload shape.
- [ ] Negative tests per kind (missing field, wrong type, invalid reference).
- [ ] Versioning test: live edit creates `question_versions` row; draft edit does not.
- [ ] Points test: default 1, negative rejected, zero accepted.
- [ ] Snapshot test `openapi_yaml_snapshot_matches` still passes.

## Definition of done

- `make check` + `make test-bank` pass (gated `AME_RUN_DB_TESTS=1`).
- `docs/plans/tasks/plan-3-question-bank.jsonl` rows are `done` including P3-008.
- All 5 kinds appear in the OpenAPI snapshot.
- Bank module compiles without referencing `engine`, `assess`, or attempt/session types.
