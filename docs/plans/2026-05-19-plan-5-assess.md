# Plan 5: Assess (Exams)

**Goal:** Implement first-class Exams, Blueprint resolution (static vs. dynamic), and structured result generation.

**Prerequisites:** Plan 4 (Engine) completed.

---

## Task 1: Domain Types (Exams)

- [ ] **Step 1: `domain/exam.rs`**
  Define `Exam`, `ExamBlueprint` (Enum: Static, Dynamic).
  Define `StaticSection`, `DynamicSection`.
  Define `ExamResult` and `LevelMapping`.

## Task 2: Blueprint Resolution

- [ ] **Step 1: `assess/blueprint.rs`**
  Implement logic for resolving a blueprint into a `QuestionPlan`.
  - Static: directly map IDs. Check for archived.
  - Dynamic: sample without replacement per section. If pool is insufficient, yield `ExamPoolInsufficient` error.
- [ ] **Step 2: `assess/results.rs`**
  Implement `compute_exam_result` to be called on session finish.
  Calculate `score_overall`, section scores. Resolve `level_label` against `level_mappings` table based on weight.

## Task 3: Admin Configuration & HTTP Handlers

- [ ] **Step 1: Exams config (`http/exams.rs`)**
  `GET /exams`, `GET /exams/:id`
  `POST /admin/exams`
  `PATCH /admin/exams/:id`
  `POST /admin/exams/:id/publish`
  `POST /admin/exams/:id/archive`
- [ ] **Step 2: Exam taking (`http/sessions.rs` & `http/exams.rs`)**
  `POST /exams/:id/start`
  `POST /sessions/:id/finish` (force finish an exam, compute results).
  Ensure `POST /sessions/:id/answer` suppresses feedback if `show_results_during = false`.
- [ ] **Step 3: Level Mappings (`http/admin.rs`)**
  CRUD for `level_mappings` table.

## Task 4: Testing

- [ ] **Step 1: Integration tests**
  Test dynamic blueprint pool insufficiency error.
  Test static blueprint referencing an archived question (forces draft).
  Test level label resolution weight logic.
- [ ] **Step 2: Run `make check`**
