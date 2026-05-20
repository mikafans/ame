# Plan 4: Engine (Sessions & Grading)

**Goal:** Implement quizzes, attempting questions, grading (MCQ, Free Text, Cloze), and the core Elo update mathematics.

**Prerequisites:** Plan 3 (Question Bank) completed.

---

## Task 1: Domain Types (Sessions & Attempts)

- [ ] **Step 1: `domain/session.rs`**
  Define `Session`, `SessionKind` (Quiz, Exam), `SessionStatus`.
  Define `QuestionPlan` and `PlanItem`.
- [ ] **Step 2: `domain/attempt.rs`**
  Define `Attempt`, `AttemptResponse`.

## Task 2: Core Engine Logic (Graders & Elo)

- [ ] **Step 1: `engine/graders.rs`**
  Implement grading logic for `Mcq` (resolve `selected_position` against `presentation.option_order`), `FreeText` (handle `normalize` string operations), and `Cloze`.
- [ ] **Step 2: `engine/elo.rs`**
  Implement the exact mathematics from the spec:
  - user_avg calculation.
  - Calibration phase switch (attempts < 20 -> K_user=0, K_question=48).
  - Mature phase K-factor logic.
  - `user_tag_ratings` splitting.
- [ ] **Step 3: Property tests**
  Write `proptest` suites for `engine/elo.rs` to guarantee ratings stay bounded, sign is correct, and updates are energy-conserving.

## Task 3: Quiz Flow & Planners

- [ ] **Step 1: `engine/planner.rs`**
  Implement quiz planner. Query `live` questions matching tags/difficulty, avoiding `exclude_recent_hours`. Shuffle up to `count`. Generate `option_order` for MCQs here.
- [ ] **Step 2: `engine/session.rs`**
  Implement session lifecycle.
  - `start_quiz`: invoke planner, snapshot user ratings, create session.
  - `record_answer`: grade response, invoke Elo update (if `affects_rating`), insert attempt, check if finished.

## Task 4: HTTP Handlers

- [ ] **Step 1: Quiz start (`http/quiz.rs`)**
  `POST /quiz` -> returns `session_id`, `remaining`, and first `question`.
- [ ] **Step 2: Session answer (`http/sessions.rs`)**
  `GET /sessions/:id` -> resume.
  `POST /sessions/:id/answer` -> apply idempotency. Returns `is_correct`, `score`, `deltas`, and `next_question`.
- [ ] **Step 3: Wiring**
  Add to main router.

## Task 5: Testing

- [ ] **Step 1: E2E Quiz Test**
  Write an integration test that creates a quiz session, submits answers, and validates the Elo updates correctly hit the database.
- [ ] **Step 2: Run `make check`**
