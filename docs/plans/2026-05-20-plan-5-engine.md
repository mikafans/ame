# Plan 5 (2026-05-20): Engine — sessions, graders, Elo, planner

**Goal:** Land the session lifecycle, per-kind graders, Elo rating math, and the study planner. Ports the `feat/foundation` P4 spike (`engine/elo.rs`, `engine/graders.rs`, `engine/planner.rs`, `tests/planner.rs`) into a real phase.

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §4.9 (Session), §4.10 (Attempt), §4.14 (StudyPlan), §6.1 sessions/attempts surface.

**Prerequisites:** Plan 4 (quiz authoring) so sessions have a quiz/exam to draw from.

**Module rule:** `engine` may depend on `bank` and `domain`; never the reverse. `engine` MAY depend on `quiz` (read-only).

---

## Task 1: Session lifecycle (spec §6.1)

`POST /sessions` accepts three body flavours (per spec §6.1 and `design/api.md` oneOf):

| Body                                                       | Source       | Notes                                                                                                  |
| ---------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------ |
| `{ quizId }`                                               | Quiz attempt | Server hydrates `quiz_questions` in order.                                                             |
| `{ examId }`                                               | Exam attempt | Server hydrates per-section composition (calls into Plan 6 once that lands; behind a stub until then). |
| `{ cats[], tags[], types[], diff, count, duration, mode }` | Practice     | Ad-hoc draw from the item bank.                                                                        |

- [ ] **Step 1: `domain/session.rs`.** Already drafted in the P4 spike — promote and reconcile with the new spec's `Session` shape.
- [ ] **Step 2: `affects_rating` rule.** True for `practice` and `quiz` sessions; false for `exam` sessions whose containing exam has `method='agent'` (spec §5.2 line 187). Server-side only; never user-toggled.
- [ ] **Step 3: `deadline_at`** = `started_at + duration_min` for timed sessions. Answers after deadline return 410 `exam_expired`.
- [ ] **Step 4:** `POST /sessions/{id}/answer` is idempotent on `(sessionId, questionId)`.
- [ ] **Step 5:** `POST /sessions/{id}/finish` marks `finished`, computes `result`, triggers grader pipeline.

## Task 2: Graders (5 kinds)

- [ ] **Step 1: Grader registry.** `trait Grader { fn grade(payload: &TypedPayload, response: &Value) -> GradeOutcome }`. One implementation per kind.
- [ ] **Step 2: Deterministic graders** — `mc`, `tf`, `short`.
  - `mc`: exact match on correct option id(s).
  - `tf`: bool match.
  - `short`: case-sensitive or insensitive depending on payload; substring/regex options reserved.
- [ ] **Step 3: Pending-manual grader** — `essay`. Returns `GradeOutcome::PendingManual` with rubric stub. Surfaces to instructor via `POST /attempts/{id}/grade`.
- [ ] **Step 4: Stub grader** — `code`. v1 compares response against `exemplar` exact-match. Real code execution sandbox is out of scope.
- [ ] **Step 5: GradeOutcome shape.** `{ correct: bool, points_awarded: int, max: int, note?: string }`. Aggregates into `attempts.score` + `attempts.percent`.

## Task 3: Elo (port from spike)

- [ ] **Step 1: Two-phase model.** Calibration (first N attempts per item) uses a wider K-factor; mature phase narrows. Spec §4.10 documents the constants.
- [ ] **Step 2: Tests.** Port `api/tests/planner.rs` calibration and mature-phase coverage.
- [ ] **Step 3:** Sessions with `affects_rating=false` skip Elo updates entirely.

## Task 4: Planner (port from spike)

- [ ] **Step 1: `POST /plans`** body: `{ userId, goal, lookbackDays? }`. Returns `StudyPlan` (spec §4.14).
- [ ] **Step 2: Algorithm.** Pull weakest tags from attempts; emit a sequence of suggested quizzes/practice sessions. Algorithm details live in `engine/planner.rs`.
- [ ] **Step 3:** Webhook `plan.created` fires on success (spec §8). Plumbing exists in P8; reference here as a producer.

## Task 5: Tests

- [ ] One end-to-end test per body flavour of `POST /sessions`.
- [ ] One golden-path grading test per kind.
- [ ] Idempotent answer replay.
- [ ] Deadline expiry returns 410.
- [ ] Calibration vs mature Elo curve checked against fixtures.
- [ ] Planner produces a non-empty plan from synthetic attempt history.

## Definition of done

- `make check` + `make test-engine` pass.
- All 5 grader kinds have at least one positive + one negative test.
- Spike files under `api/src/engine/` are integrated and the spike's `pub mod` additions in `api/src/engine/mod.rs` are reconciled (no duplicate modules).
- Plan 5 ledger rows are `done`.
