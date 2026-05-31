# Assessment Unification — quiz + exam → `assessment`

Status: DRAFT · 2026-05-31 · branch `refactor/agent-identity`

## Goal

Collapse the two definition-layer entities (`tb_quizzes`, `tb_exams`) into one
**`assessment`** entity. A quiz and an exam stop being separate types; they become
the same structure under different **modes**. The runtime layer (`tb_sessions`,
`tb_attempts`) already unifies them — this finishes the job at the definition layer
and removes the duplicate runner / share / preview / results code paths.

## Design decision

Separate **structure** from **policy** (decided 2026-05-31):

- **Structure** — an assessment is a titled, ordered list of **sections**; each
  section holds questions. A "quiz" is the preset: exactly **one** section. Sections
  are always present, so there is no `if has_sections` branch anywhere.
- **Policy** — timing, passing score, results-visibility, rating impact, and
  generative composition are gated by a **`mode`** enum, *not* by entity type.

`mode`:

- `practice` — no timer, no passing score, results shown immediately. (former quiz)
- `graded` — timer / passing score / results-visibility policy applies. (former exam)

Tags remain the category/mastery axis — untouched. `course` (freeform text) carries
over unchanged; the "Course as a first-class owning entity" question is explicitly
**out of scope**.

## Schema

### `tb_assessments`

```
id                  uuid PK
title               text NOT NULL
description         text
mode                text NOT NULL  CHECK (mode IN ('practice','graded'))
status              text NOT NULL DEFAULT 'draft'
                       CHECK (status IN ('draft','active','archived'))
objectives          text[] NOT NULL DEFAULT '{}'
course              text
-- policy (meaningful only when mode='graded'; NULL/false for practice)
duration_min        integer
time_limit_seconds  integer
total_points        integer NOT NULL DEFAULT 0
passing_points      integer
show_results_during boolean NOT NULL DEFAULT FALSE
affects_rating      boolean NOT NULL DEFAULT TRUE
method              text NOT NULL DEFAULT 'manual'
                       CHECK (method IN ('manual','agent'))
composition_trace   jsonb
created_by          uuid NOT NULL REFERENCES tb_users(id)
created_at          timestamptz NOT NULL DEFAULT now()
updated_at          timestamptz NOT NULL DEFAULT now()
```

Notes:
- `status`: keep the quiz vocabulary (`draft/active/archived`). The exam
  `published` status maps to `active` on migration.
- Policy columns stay nullable rather than split into a side table — simpler, and
  `mode` is the single source of truth for whether they apply. Validation that
  graded assessments have a timer/passing score (if required) lives in the API, not
  a DB CHECK, to avoid brittle cross-column constraints.

### `tb_assessment_sections`

```
id            uuid PK
assessment_id uuid NOT NULL REFERENCES tb_assessments(id) ON DELETE CASCADE
title         text NOT NULL
order_index   integer NOT NULL
weight        double precision NOT NULL DEFAULT 1.0
mix           jsonb            -- generative composition (graded only); else NULL
items_count   integer NOT NULL
UNIQUE (assessment_id, order_index)
CHECK ((mix IS NULL) OR (mix IS NOT NULL))  -- mix XOR explicit items, enforced via items table
```

Explicit-vs-generative rule: a section is **either** explicit (rows in
`tb_assessment_items`) **or** generative (`mix` set, no item rows). Enforced in API.
A practice quiz is one section with `mix = NULL` and explicit items.

### `tb_assessment_items`

Replaces both `tb_quiz_questions` and the exam-section `question_ids[]` array.
Carries quiz's `points_override` uniformly.

```
section_id      uuid NOT NULL REFERENCES tb_assessment_sections(id) ON DELETE CASCADE
question_id     uuid NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE
order_index     integer NOT NULL
points_override integer
PRIMARY KEY (section_id, question_id)
UNIQUE (section_id, order_index)
```

### `tb_sessions` changes

- Add `assessment_id uuid REFERENCES tb_assessments(id)`.
- Backfill from `quiz_id` / `exam_id`.
- Replace the `tb_sessions_kind_link` CHECK with one tying every non-practice
  session to `assessment_id`. Keep `kind` for now (`practice` vs assessment-backed),
  or fold to `mode` — see Open Questions.
- Drop `quiz_id`, `exam_id` after backfill.

## Data migration

1. Each `tb_quizzes` row → `tb_assessments` (`mode='practice'`, status carried;
   `active` stays `active`). One `tb_assessment_sections` row (`order_index=0`,
   `items_count = count`). `tb_quiz_questions` → `tb_assessment_items`
   (preserve `order_index`, `points_override`).
2. Each `tb_exams` row → `tb_assessments` (`mode='graded'`; `published`→`active`;
   copy policy columns + `composition_trace`). `tb_exam_sections` → sections;
   `question_ids[]` → `tb_assessment_items` rows (synthesise `order_index`); `mix`
   sections copy `mix` and create no item rows.
3. Repoint `tb_sessions.assessment_id`, drop old FKs/columns.
4. Drop `tb_quizzes`, `tb_quiz_questions`, `tb_exams`, `tb_exam_sections`,
   `tb_quiz_item_stats` (re-create the stats view/table against assessments).

UUIDs: reuse the existing quiz/exam `id` as the new assessment `id` so existing
session FKs and any external share links stay valid.

## API

- New `/v1/assessments` router replacing `/v1/quizzes` and `/v1/exams`.
  - `GET /v1/assessments?mode=practice|graded&status=...` — list.
  - `POST /v1/assessments`, `GET/PATCH/DELETE /v1/assessments/{id}`.
  - Section + item sub-resources (mirror existing quiz `POST .../questions` which
    accepts `questionId` link or inline `kind+prompt` draft — preserve both modes).
  - `POST /v1/assessments/{id}/preview`, `.../sessions` (start).
- Collapse `http/quizzes.rs` (862) + exam handlers into `http/assessments.rs`;
  split if it exceeds the hotspot threshold. `http/sessions.rs` (1176) loses the
  quiz/exam branch — single assessment path.
- Regenerate `api/openapi.yaml` (`make openapi`); update integration tests
  (auth/sessions/exams/shares/stats) to the new routes. Remember: test tokens hash
  via `ame_api::auth::token::hash_secret` (sha256), not Argon2.
- `shares` `kind`: add/normalise to `assessment`; keep `quiz`/`exam` accepted as
  aliases for one release if any link exists in the wild.

## Frontend

- `/quizzes` and exam pages → `/assessments`, filtered by `mode`. The learner
  library, author flow, preview route, and results history all point at the new
  endpoints.
- Reuse the existing session runner UI — it's already mode-agnostic at runtime.
- MUI only; watch the dead Tailwind-era CSS vars noted in project memory.

## Out of scope

- Course-as-entity / curriculum / enrollment.
- Tag/mastery changes.
- The Share viewer public page and results-history page (separate gaps; this spec
  only keeps them working against the renamed routes).

## Open questions

1. `tb_sessions.kind`: keep `practice|quiz|exam` (mapped) or replace with
   `mode`? Leaning replace with `assessment_id` + the assessment's `mode`.
2. Do practice assessments ever need >1 section? If yes, "quiz = 1 section" is a
   UI default, not a constraint. Assume default-only for now.
3. Backwards-compat window for `quiz`/`exam` share `kind` aliases — one release or
   hard cut?

## Phased dispatch (TL plan)

1. **DB migration** (single migration file) + data backfill — `haiku-developer`.
   Verify with `make db-reset && make dev`, then `make db-seed`.
2. **API** — `http/assessments.rs`, sessions simplification, openapi, integration
   tests — `haiku-developer`; review for the auth/token + CHECK details.
3. **Frontend** — routes, library, author, preview — `haiku-developer`.
4. **e2e + seed** — update specs and `scripts/seed.py`; run `make ci`.

Each phase gated by foreground verification before commit. `make check` must pass;
ask before committing.
