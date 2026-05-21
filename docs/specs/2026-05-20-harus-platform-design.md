# Harus — Assessment Platform Design Spec

**Status:** Canonical. Replaces `2026-05-19-question-exam-platform-design.md` (kept on disk as historical reference; do not edit).
**Owner:** rebuild track (`feat/harus-redesign-docs` → `feat/harus-redesign-code`).
**Source of truth:** this file, plus `design/README.md`, `design/api.md`, `design/tokens.md`, and `design/source/src/*.jsx` for visual fidelity.
**Last updated:** 2026-05-20.

---

## 1. North star

**Harus** is an assessment platform with three first-class audiences sharing one data model:

1. **Learners** take quizzes and composed exams; review per-item results; track progress over time.
2. **Authors / instructors** compose individual quizzes and bundle them into weighted exams.
3. **AI agents** read and write the same objects through an OpenAPI / MCP surface — import quizzes, generate quizzes from source documents, compose exams from existing quizzes, fetch attempts, query cohort stats, send feedback, and create study plans.

**Organizing principle:** every screen a human uses is backed by the same endpoint an agent uses. No duplicate state, no scraping. The Author studio writes through `quiz.update`. The Quiz setup screen reads from `quiz.get` and posts a `session.create`. The Dashboard's "6-week plan" is a stored `plan.create` response.

**v1 scope:** the eight screens described in `design/README.md`, the API surface in `design/api.md`, and the MCP manifest export described below. Multi-tenant org isolation, billing, real proctor integrations, and mobile layouts are explicitly out of scope.

---

## 2. Stack

| Layer                            | Choice                                                              | Notes                                                                                            |
| -------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Backend language                 | Rust (edition 2021)                                                 |                                                                                                  |
| HTTP framework                   | Axum 0.8 + Tokio                                                    |                                                                                                  |
| Database                         | Postgres 18                                                         | `db/docker-compose.yml` for local dev.                                                           |
| ORM / query                      | `sqlx`                                                              | Migrations under `db/migrations/`.                                                               |
| Frontend framework               | Next.js 15 App Router                                               | Single deployment; `/admin/*` role-gated.                                                        |
| Frontend language                | TypeScript, strict                                                  | React 19.                                                                                        |
| Styling                          | Tailwind 3                                                          | Tokens from `design/tokens.md` exposed as CSS variables and `tailwind.config.ts` `theme.extend`. |
| Component primitives             | Radix UI                                                            | Dialog, Tabs, Tooltip, RadioGroup.                                                               |
| Forms                            | React Hook Form + Zod                                               |                                                                                                  |
| Charts                           | Recharts                                                            | Tokens drive grid/axis/line colors.                                                              |
| Code editor (Code question type) | Monaco                                                              |                                                                                                  |
| Server state                     | TanStack Query                                                      | Reads. Server Actions for mutations where the route is colocated.                                |
| Theme switching                  | `data-theme` attribute on `<html>`                                  | Three themes: slate (default), paper, cobalt.                                                    |
| Toolchain                        | `mise` for runtimes (rust, bun, uv); `uvx pgcli` for SQL; `uvx sqlfluff` for SQL linting | `make check` before commits, `make validate` before PRs.                                         |

---

## 3. Vocabulary (precise terms)

These terms are used consistently in the schema, API, UI copy, and code. **Do not use them interchangeably.**

| Term                   | Definition                                                                                                                                                                  |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Question**           | A single item in the bank. Has a `kind` (mc / tf / short / essay / code), a prompt, type-specific payload, optional explanation, tags, difficulty, and a rating. Versioned. |
| **Quiz**               | An author-curated, ordered set of questions belonging to one course. The reusable unit instructors publish and learners take directly.                                      |
| **Exam**               | A _composed_ assessment: a bundle of one or more quiz sections with weights, a window, and a passing mark. Authored manually or by an agent.                                |
| **ExamSection**        | One row inside an exam: `{ quizId, weight, items?, mix? }`. Items can be drawn dynamically from the referenced quiz.                                                        |
| **Session**            | A learner's runtime instance of an assessment. Three kinds: `quiz` (take a specific quiz), `exam` (take an exam), `practice` (ad-hoc draw from the item bank).              |
| **Attempt**            | A single submitted response inside a session — one row per question answered. The session's `result` aggregates these.                                                      |
| **Course**             | Organizational container ("CS101"). A quiz belongs to one course; an exam belongs to one course; a tag may be global or course-scoped.                                      |
| **Cohort**             | The set of learners enrolled in a course for a term. Quizzes and exams target cohorts.                                                                                      |
| **Tag**                | Topic / skill label. Used to filter the item bank and to compute per-tag mastery.                                                                                           |
| **ApiKey**             | A bearer token with scopes. Format `hk_<env>_<id>`.                                                                                                                         |
| **Scope**              | Permission verb attached to a key. See §6.2.                                                                                                                                |
| **Webhook**            | Outbound HTTPS POST fired on lifecycle events.                                                                                                                              |
| **ActivityLog**        | Append-only audit record of agent-driven mutations.                                                                                                                         |
| **StudyPlan**          | The output of `plan.create`: a week-by-week recovery plan stored against a learner.                                                                                         |
| **Message**            | A one-off feedback note sent via `feedback.send` — in-app today, email channel reserved.                                                                                    |
| **ShareLink**          | A read-only public or cohort-scoped reference to a Quiz, Exam, or single Question (with optional explanation). Powers the Share modal and embed iframes.                    |
| **LearningObjectives** | A 3-4 bullet `string[]` field on Quiz and Exam ("what you'll learn"). Rendered on the Library hero and Exam detail panel; editable in the Author studio.                    |

### What's different from the May-19 spec

The May-19 spec treated _every_ assessment as a `Session` with `kind ∈ {quiz, exam}` and had only three question kinds (`mcq`, `free_text`, `cloze`). This redesign:

- **Promotes Quiz and Exam to first-class persistent entities.** Sessions become the runtime layer only.
- **Replaces the three question kinds with five:** `mc` (multi-choice), `tf` (true/false), `short` (free text), `essay` (long-form, optional rubric), `code` (editor + tests).
- **Adds Course, Cohort, ApiKey, Webhook, ActivityLog, StudyPlan, Message, ShareLink** as named entities.
- **Adds per-question `points`, and `objectives: string[]` on Quiz and Exam.**
- **Adds the "agent / manual" composition method** on exams, with a stored composition trace.
- **Adds a public sharing surface** (`/v1/shares`, `/v1/{kinds}/{id}/embed`) with anonymous-attempt semantics on `?interactive=1` embeds.
- **Replaces the catch-all "scope vocabulary" with the scope table in §6.2.**

Existing migrations and Rust domain types from Plans 1-3 (`api/src/domain/{question,session,user,attempt}.rs`, `db/migrations/20260519092355_init.sql`) are **the starting point**, not the destination. Plan 2 in the new ledger covers the schema rewrite.

---

## 4. Entity model

Required fields shown. All entities carry `id: uuid v7`, `created_at`, `updated_at` unless noted. `*` marks a field added by this redesign and not present in current code.

### 4.1 User

`{ id, email?, display_name, role: 'learner' | 'instructor' | 'admin' | 'agent', avatar_url? }`. Agents have `role='agent'` and no email; they authenticate by ApiKey only. _Old code has `role ∈ {admin, user}` — this is widened._

### 4.2 Course `*`

`{ id, code, title, term, owner_id (User), description? }`. Owner is an instructor or admin.

### 4.3 Cohort `*`

`{ id, course_id, label, term, member_count }`. Junction table `cohort_members(cohort_id, user_id)` keeps enrolment lists. A user can belong to multiple cohorts.

### 4.4 Tag

`{ id, name, description?, course_id? }`. `course_id` null means global.

### 4.5 Question

`{ id, kind: 'mc'|'tf'|'short'|'essay'|'code', prompt, code_snippet?, payload (jsonb), explanation?, status: 'draft'|'live'|'archived', source?, points: int (default 1, ≥0), rating (Elo), attempts_count, version, created_by, course_id?, difficulty: 'intro'|'inter'|'adv' }`. `points` is the per-question weight inside a quiz; the same question can carry different effective weights when referenced from different exam sections, but its bank-level `points` is the default.

Per-kind payload shapes (typed at the wire boundary, jsonb in the DB):

- **mc:** `{ options: string[], correct_index: usize }` — server-side shuffle; the wire `correct_index` always refers to the canonical (unshuffled) order. Persisted `option_order` per attempt makes grading deterministic.
- **tf:** `{ correct: bool }`
- **short:** `{ accepted: string[], normalize: 'exact' | 'case_insensitive_strip_accents', judge: 'exact' }` (LLM judge reserved).
- **essay:** `{ min_words?: int, rubric?: string, judge: 'manual' | 'llm' }` — v1 stores the response and routes to manual grading; LLM judge reserved.
- **code:** `{ language: string, starter: string, tests: { name: string, body: string }[], time_limit_ms?: int }` — runner is a sandboxed worker; v1 may stub the runner and accept a stored exemplar match.

`QuestionVersion` retains historical payloads when a `live` question is edited (see §"Versioning" in May-19 spec — rule unchanged).

### 4.6 Quiz `*`

`{ id, title, course_id, author_id, description?, objectives: string[], difficulty: 'intro'|'inter'|'adv'|'mixed', duration_min: int, status: 'draft'|'active'|'archived', color: 'accent'|'blue'|'amber', tags: [Tag], questions: ordered Question[] }`. The ordered-questions join is `quiz_questions(quiz_id, question_id, position)`. Publishing locks `quiz_questions` rows (questions can still version independently).

`objectives` are 3-4 outcome bullets ("what you'll learn") rendered on the Library hero and editable in the Author studio. Stored as `text[]`. Agents producing quizzes via `quiz.generate` SHOULD emit objectives best-effort; if the source doesn't yield enough material the response includes a `warnings[]` entry rather than failing.

### 4.7 Exam `*`

`{ id, title, course_id, composed_by_id (User), method: 'manual'|'agent', status: 'draft'|'scheduled'|'active'|'archived', window: { open?, close? }, duration_min, total_points, passing_points, objectives: string[], sections: ExamSection[], composition_trace? }`.

`objectives` follows the same convention as Quiz (3-4 bullets, `text[]`, rendered between the stat strip and the Composition section on the Exam detail panel).

`composition_trace` is non-null when `method='agent'`. Shape: `{ tool: 'exam.compose', inputs: { … }, seed: string, strategy: string, confidence: float }`. Rendered verbatim in the Exam detail screen.

### 4.8 ExamSection `*`

`{ id, exam_id, quiz_id, title?, weight: int (1..100), items?: int, mix?: string, position: int }`. Weights are stored as integers; they need not sum to 100 — `weight / sum(weights)` is the runtime ratio. `items` null means "all of the quiz"; non-null means "draw N from the quiz with the `mix` strategy".

### 4.9 Session

`{ id, user_id, kind: 'quiz'|'exam'|'practice', quiz_id?, exam_id?, filter? (jsonb for practice), question_plan (jsonb, QuestionPlan[]), status: 'in_progress'|'finished'|'abandoned', affects_rating: bool, rating_snapshot, deadline_at?, started_at, finished_at?, result? }`.

`QuestionPlan` is unchanged from current code: `{ items: [{ question_id, version, section?, option_order? }] }`.

### 4.10 Attempt

`{ id, session_id, user_id, question_id, question_version, response (jsonb tagged by kind), presentation: { option_order? }, is_correct, score, time_to_answer_ms?, rating_before_user_avg, rating_before_question, user_tag_deltas, question_delta }`.

Response shapes mirror question kinds:

- mc: `{ selected_position: usize }` (the _shuffled_ position, deshuffled server-side using `presentation.option_order`).
- tf: `{ answer: bool }`
- short: `{ answer: string }`
- essay: `{ body: string, word_count: int }`
- code: `{ source: string, language: string }`

### 4.11 ApiKey `*`

`{ id, user_id, label, prefix (`hk*<env>*<6char>`), hash, scopes: Scope[], created_at, last_used_at?, revoked_at? }`. Only the prefix is shown after creation; the full key is returned once on `POST /v1/agents/register`.

### 4.12 Webhook `*`

`{ id, user_id, url, secret_hash, events: WebhookEvent[], status: 'active'|'paused', last_delivery_at?, last_status? }`. See §8 for events.

### 4.13 ActivityLog `*`

`{ id, ts, agent_id (User), tool_name (string, e.g. `quiz.import`), method, path, status: int, note?, target_id? }`. Append-only; powers the Agent screen's activity table.

### 4.14 StudyPlan `*`

`{ id, user_id, goal: string, lookback_days: int, generated_at, generated_by: 'plan.create', weeks: [{ week_num, focus, items: [{ kind, ref_id, hours_est }] }] }`.

### 4.15 Message `*`

`{ id, from_user_id (agent or instructor), to_user_id (learner), channel: 'in_app'|'email', body, link_quiz_id?, sent_at, read_at? }`.

### 4.16 ShareLink `*`

`{ id, kind: 'quiz'|'exam'|'item', target_id, created_by_user_id, visibility: 'public'|'cohort', include_explanation: bool (default false), include_score: bool (default false), include_attribution: bool (default true), og_image_url?, created_at, revoked_at? }`.

A `ShareLink` resolves to a stripped read-only view of its `target_id`. Three privacy rules are enforced server-side at resolution time:

1. Shared links never reveal other learners' attempts or scores.
2. `include_score` is opt-in only (default false). When false, the resolved view shows the question / quiz / exam content but no score number.
3. `visibility: 'cohort'` restricts resolution to authenticated members of any cohort the target's course is assigned to.

The `?interactive=1` query parameter on the embed route (§6.1 Share) allows **anonymous attempts**: the attempt is logged (rate-limited per IP) but not associated with any user, never affects ratings, and never appears in cohort stats. Anonymous attempts live in a separate `anonymous_attempts` table with `(share_id, question_id, ts, ip_hash, response, is_correct)`.

---

## 5. Schema overview

The full schema lives in `db/migrations/`. This section calls out the constraints that the API and UI both rely on; the migration in Plan 2 of the new ledger is the canonical source.

### 5.1 Naming & types

- All ids are `uuid` (v7) — server-generated; never trust client ids.
- All timestamps are `timestamptz`, stored UTC, serialised RFC 3339.
- Enums are `text` columns with `CHECK` constraints, _not_ Postgres enum types — easier to evolve. The Rust `FromStr` impl and the `CHECK` list are the **single source of truth pair** and must change in lockstep.
- All foreign keys cascade carefully: `ON DELETE CASCADE` on dependent rows (e.g. `exam_sections → exams`), `ON DELETE RESTRICT` on entities referenced from history (e.g. `attempts.question_id`).

### 5.2 Critical constraints

- **Question.payload jsonb must match Question.kind.** Enforced at the HTTP boundary via typed payloads and a `validate_payload(kind, payload)` helper. No DB-level json schema check — too brittle.
- **Quiz/Exam status transitions** are one-way except for `archived` (terminal): `draft → active → archived`, `draft → scheduled → active → archived`. Enforced in repository methods, not triggers.
- **Session.affects_rating** is `true` for `practice` and `quiz` sessions; `false` for `exam` sessions whose containing exam has `method='agent'` (an agent-composed exam acts as a probe, not a graded event). The Quiz setup screen never lets the learner toggle this; the rule is server-side.
- **Idempotency-Key** on all `POST` and `PATCH` endpoints. Stored in `idempotency_keys(key, request_hash, response_body, created_at)`. Conflicting body with the same key returns 409 `idempotency_conflict`.
- **Session deadline.** `deadline_at` is computed at session start as `started_at + duration_min` for timed sessions; the server rejects answers after the deadline with 410 `exam_expired`.
- **Objectives length is unbounded but the UI is designed around 3-4 entries.** Servers SHOULD return a `warnings[]` entry when a Quiz or Exam is saved with `objectives.length > 6`. Not a hard limit — instructor escapes are valid.
- **Anonymous attempts are rate-limited.** `?interactive=1` embed attempts are bucketed per IP (suggested: 30 attempts / 10 minutes / IP). Over-limit requests return 429.

### 5.3 Indexes (load-bearing)

- `questions (status, kind)` for the bank browser.
- `questions (tags GIN)` for tag filters.
- `attempts (user_id, created_at DESC)` for the learner's history.
- `attempts (session_id, question_id)` unique.
- `sessions (user_id, status, started_at DESC)` for the "in progress" badge.
- `exam_sections (exam_id, position)` unique.
- `api_keys (user_id, revoked_at)` partial where revoked_at is null.
- `share_links (target_id, kind)` for resolution; `share_links (created_by_user_id, created_at DESC)` for the user's share history.
- `anonymous_attempts (share_id, ts DESC)` for rate-limit lookups and basic analytics.

---

## 6. API surface

All endpoints are prefixed `/v1`. Bearer auth via `Authorization: Bearer hk_<env>_<id>`. Errors are RFC 7807 problem+json (current `ApiError` enum is close — add `not_authorised`, `webhook_failed`, and reshape under `application/problem+json`). Rate limit: 120 req/min per key.

### 6.1 Endpoint table

This is the authoritative list. Tool names map 1:1 to MCP tool ids.

#### Quiz

| Method | Path                  | Tool            | Scope        | Notes                                                                                                                                                                                                                                |
| ------ | --------------------- | --------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| GET    | `/quizzes`            | `quiz.list`     | `quiz.read`  | Filters: `course`, `tag`, `status`.                                                                                                                                                                                                  |
| GET    | `/quizzes/{id}`       | `quiz.get`      | `quiz.read`  | Full questions + rubric.                                                                                                                                                                                                             |
| POST   | `/quizzes`            | `quiz.import`   | `quiz.write` | Body: `{ source, format: 'json'\|'md', courseId? }`. Incoming sources MAY include an `objectives: string[]` field — preserved when present. Returns `{ quizId, questionCount, warnings[] }`.                                         |
| POST   | `/quizzes/generate`   | `quiz.generate` | `quiz.write` | Body: `{ source, questionCount, types?, difficulty? }`. Returns `{ quizId, questions[], objectives[] }` — agents SHOULD emit objectives best-effort; insufficient source material produces a `warnings[]` entry rather than failing. |
| PATCH  | `/quizzes/{id}`       | `quiz.update`   | `quiz.write` | Locked once `status='active'` (only metadata patchable).                                                                                                                                                                             |
| DELETE | `/quizzes/{id}`       | `quiz.delete`   | `quiz.write` | Soft delete → status='archived'.                                                                                                                                                                                                     |
| GET    | `/quizzes/{id}/stats` | `stats.cohort`  | `stats.read` | `{ avg, median, distribution[], items[] }`.                                                                                                                                                                                          |

#### Exam

| Method | Path                | Tool           | Scope        | Notes                                                                                                                                                                                                                                     |
| ------ | ------------------- | -------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| GET    | `/exams`            | `exam.list`    | `quiz.read`  | Filters: `course`, `status`.                                                                                                                                                                                                              |
| GET    | `/exams/{id}`       | `exam.get`     | `quiz.read`  | Returns `{ exam, sections[], cohortStatus, compositionTrace? }`.                                                                                                                                                                          |
| POST   | `/exams`            | `exam.compose` | `quiz.write` | Body: `{ title, courseId, sections: [{ quizId, weight, items? }], duration, window? }`. Returns `{ examId, totalPoints, sections[], warnings[] }`. `method` is inferred from the calling token's role (`agent` ⇒ `agent`, else `manual`). |
| PATCH  | `/exams/{id}`       | `exam.update`  | `quiz.write` | Window + status only after `scheduled`.                                                                                                                                                                                                   |
| GET    | `/exams/{id}/stats` | `exam.stats`   | `stats.read` | `{ passRate, sectionAvgs[], timeP50, timeP95 }`.                                                                                                                                                                                          |

#### Session & attempt

| Method | Path                    | Tool             | Scope           | Notes                                                                                                                                                                                         |
| ------ | ----------------------- | ---------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| POST   | `/sessions`             | `session.create` | `attempt.write` | Body matches the Quiz setup form. Three flavours: `{ quizId }`, `{ examId }`, or `{ cats[], tags[], types[], diff, count, duration, mode }` (practice). Returns `{ sessionId, questions[] }`. |
| GET    | `/sessions/{id}`        | `session.get`    | `attempt.read`  | For resume.                                                                                                                                                                                   |
| POST   | `/sessions/{id}/answer` | `session.answer` | `attempt.write` | Body: `{ questionId, response, timeToAnswerMs? }`. Idempotent on `(sessionId, questionId)`.                                                                                                   |
| POST   | `/sessions/{id}/finish` | `session.finish` | `attempt.write` | Marks `finished`, computes `result`.                                                                                                                                                          |
| GET    | `/attempts/{id}`        | `attempt.get`    | `attempt.read`  | `{ user, score, answers[], rubric[] }`.                                                                                                                                                       |
| POST   | `/attempts/{id}/grade`  | `attempt.grade`  | `attempt.write` | Manual override or re-grade (essay path).                                                                                                                                                     |

#### Agent surface

| Method | Path                   | Tool              | Scope       | Notes                                                                                                                        |
| ------ | ---------------------- | ----------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------- |
| POST   | `/agents/register`     | `agents.register` | (none)      | One-shot: `{ label?, scopes[] }` → `{ apiKey: 'hk_<env>_<full>', userId, openapiUrl, mcpManifestUrl }`. Rate-limited per IP. |
| GET    | `/agents/openapi.json` | —                 | `quiz.read` | OpenAPI 3.1 doc.                                                                                                             |
| GET    | `/agents/mcp.json`     | —                 | `quiz.read` | MCP manifest (see §7).                                                                                                       |
| GET    | `/agents/activity`     | `agent.activity`  | `quiz.read` | Paginated ActivityLog.                                                                                                       |

#### Feedback & plans

| Method | Path          | Tool            | Scope            | Notes                                                                                |
| ------ | ------------- | --------------- | ---------------- | ------------------------------------------------------------------------------------ |
| POST   | `/messages`   | `feedback.send` | `feedback.write` | `{ userId, channel: 'in_app'\|'email', body, linkQuizId? }`. Email channel reserved. |
| POST   | `/plans`      | `plan.create`   | `plan.write`     | `{ userId, goal, lookbackDays? }` → StudyPlan.                                       |
| GET    | `/plans/{id}` | `plan.get`      | `plan.read`      |                                                                                      |

#### Share

Public sharing surface. Powers the Share modal (Library / Exams / Results) and embeddable iframes. The Card tab requires a server-rendered OG image (1200×630); v1 may ship a Satori / Vercel-OG worker behind `og_image_url`.

| Method | Path                  | Tool           | Scope           | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------ | --------------------- | -------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| POST   | `/shares`             | `share.create` | `quiz.read`     | Body: `{ kind: 'quiz'\|'exam'\|'item', id, visibility?: 'public'\|'cohort', includeExplanation?, includeScore?, includeAttribution? }`. Returns `{ shareId, url, embedUrl, og: { image, title, description } }`. Idempotency: standard `Idempotency-Key` header per §5.2 is the primary mechanism; in its absence, the server additionally dedupes on the natural tuple `(created_by, kind, target_id, visibility, include_*)` and returns the existing live (non-revoked) share. |
| GET    | `/shares/{id}`        | `share.get`    | (none — public) | Public read-only resolution. Returns a stripped target with privacy rules from §4.16 applied. `404 not_found` for revoked links.                                                                                                                                                                                                                                                                                                                                                  |
| GET    | `/{kinds}/{id}/embed` | —              | (none — public) | Iframe-friendly read-only view. `kinds ∈ quizzes\|exams\|items` (plural, matching the resource paths above — derived from the share body's singular `kind`). The trailing `/embed` segment disambiguates from `GET /quizzes/{id}` etc., so the router MUST match the longer path first (declare `/quizzes/{id}/embed` before `/quizzes/{id}`). `?interactive=1` enables anonymous attempts (rate-limited per IP, never written to any user's record).                             |
| DELETE | `/shares/{id}`        | `share.revoke` | `quiz.read`     | Sets `revoked_at`. Caller must be the original `created_by_user_id` or an admin.                                                                                                                                                                                                                                                                                                                                                                                                  |

#### Keys & webhooks (instructor / admin UI; no agent scope)

| Method | Path                   | Notes                                                      |
| ------ | ---------------------- | ---------------------------------------------------------- |
| GET    | `/me/keys`             | List your keys.                                            |
| POST   | `/me/keys`             | Create `{ label, scopes[] }` → `{ apiKey, prefix }`.       |
| POST   | `/me/keys/{id}/rotate` | Returns new full token, old hash invalidated.              |
| DELETE | `/me/keys/{id}`        | Revoke (sets `revoked_at`).                                |
| GET    | `/me/webhooks`         | List.                                                      |
| POST   | `/me/webhooks`         | `{ url, events[] }` → `{ id, secret }`. Secret shown once. |
| DELETE | `/me/webhooks/{id}`    |                                                            |

### 6.2 Scopes

Bearer tokens carry one or more scopes. The set is fixed (DB `CHECK` constraint + Rust `Scope` enum):

| Scope            | Reads                         | Writes                                             |
| ---------------- | ----------------------------- | -------------------------------------------------- |
| `quiz.read`      | quizzes, exams, agent surface | —                                                  |
| `quiz.write`     | (implies quiz.read)           | quizzes, exams                                     |
| `attempt.read`   | sessions, attempts            | —                                                  |
| `attempt.write`  | (implies attempt.read)        | sessions, attempts (including answers/finish)      |
| `stats.read`     | per-quiz & per-exam stats     | —                                                  |
| `feedback.write` | —                             | messages                                           |
| `plan.read`      | study plans                   | —                                                  |
| `plan.write`     | (implies plan.read)           | study plans                                        |
| `admin`          | —                             | key & webhook management for any user (admin only) |

Human sessions (browser cookie auth) carry an effective scope set based on `role`:

- `learner` → `attempt.read`, `attempt.write`, `quiz.read`, `plan.read`.
- `instructor` → all `quiz.*`, `attempt.*`, `stats.read`, `feedback.write`, `plan.*` for owned courses.
- `admin` → all scopes.
- `agent` (programmatic) → whatever the key declares; no implicit scopes.

**Display shorthand:** the UI MAY render the token `*` in scope chips as shorthand for "all currently-active scopes" (see `design/source/src/data.jsx` `key_test_z1`). `*` is never a stored value in `api_keys.scopes` and never accepted on `POST /me/keys`; the persisted column always carries the explicit enum members.

**Sharing** (`POST /shares`, `DELETE /shares/{id}`) uses `quiz.read` — every reader can mint shares, and revoking is gated by ownership rather than scope. The public `GET /shares/{id}` and `GET /{kinds}/{id}/embed` paths require no scope.

### 6.3 Auth model

- Browser users authenticate via SSR session cookie (Plan 2 keeps `axum-login` or similar; the May-19 token-auth path is removed for humans).
- Agents authenticate via `Authorization: Bearer hk_…` only.
- One user can mint many keys; revoking a key invalidates all in-flight requests on the next auth pass.
- The `POST /v1/agents/register` shortcut is an unauthenticated bootstrap path. It creates a `role='agent'` user, mints a single key with the requested scopes (rejected if `admin` requested), and returns it. Rate-limited per source IP to deter farming.

---

## 7. MCP manifest

Exported at `GET /v1/agents/mcp.json` (also downloadable from the Agent screen). One tool entry per row in §6.1 with a `Tool` column. Shape:

```json
{
  "schema_version": "v1",
  "name": "harus",
  "description": "Read and write quizzes, exams, attempts, and study plans on Harus.",
  "auth": { "type": "bearer", "format": "hk_<env>_<id>" },
  "tools": [
    {
      "name": "quiz.import",
      "description": "Create a new quiz from JSON or Markdown source.",
      "input_schema": {
        "type": "object",
        "required": ["source", "format"],
        "properties": {
          "source": { "type": "string" },
          "format": { "type": "string", "enum": ["json", "md"] },
          "courseId": { "type": "string" }
        }
      },
      "method": "POST",
      "path": "/v1/quizzes",
      "scope": "quiz.write"
    },
    {
      "name": "share.create",
      "description": "Mint a public or cohort-scoped read-only share link for a quiz, exam, or single question.",
      "input_schema": {
        "type": "object",
        "required": ["kind", "id"],
        "properties": {
          "kind": { "type": "string", "enum": ["quiz", "exam", "item"] },
          "id": { "type": "string" },
          "visibility": {
            "type": "string",
            "enum": ["public", "cohort"],
            "default": "public"
          },
          "includeExplanation": { "type": "boolean", "default": false },
          "includeScore": { "type": "boolean", "default": false },
          "includeAttribution": { "type": "boolean", "default": true }
        }
      },
      "method": "POST",
      "path": "/v1/shares",
      "scope": "quiz.read"
    },
    {
      "name": "share.get",
      "description": "Resolve a share link to its stripped read-only view. No auth required.",
      "input_schema": {
        "type": "object",
        "required": ["id"],
        "properties": { "id": { "type": "string" } }
      },
      "method": "GET",
      "path": "/v1/shares/{id}",
      "scope": null
    },
    {
      "name": "share.revoke",
      "description": "Revoke a previously minted share link. Caller must be the original creator or an admin.",
      "input_schema": {
        "type": "object",
        "required": ["id"],
        "properties": { "id": { "type": "string" } }
      },
      "method": "DELETE",
      "path": "/v1/shares/{id}",
      "scope": "quiz.read"
    }
  ]
}
```

The Agent screen renders this descriptor verbatim. The `method`, `path`, and `scope` fields are Harus extensions on top of the MCP base schema — drop them on export to a strict MCP consumer.

---

## 8. Webhooks

Outbound HTTPS POST to the URL the user registered. Body is JSON. Signature: `X-Harus-Signature: t=<unix_ts>,v1=<hmac_sha256(secret, ts + "." + body)>`. Retries: 3 attempts with exponential backoff (10s, 60s, 300s) on non-2xx.

| Event               | Payload                                                           |
| ------------------- | ----------------------------------------------------------------- |
| `attempt.submitted` | `{ attemptId, userId, sessionId, quizId?, examId?, submittedAt }` |
| `attempt.graded`    | `{ attemptId, score, total, percent, gradedAt }`                  |
| `quiz.published`    | `{ quizId, publishedBy, publishedAt }`                            |
| `exam.opened`       | `{ examId, openedAt }`                                            |
| `exam.closed`       | `{ examId, closedAt, attemptsCount }`                             |
| `plan.created`      | `{ planId, userId, generatedAt }`                                 |

---

## 9. Screens

The eight screens in `design/README.md` §"Screens / Views" are the authoritative UI surface. This section maps them to the API and notes implementation constraints. **Do not re-document the visual layout here — that's `design/README.md`'s job.** The JSX in `design/source/src/screen-*.jsx` is the pixel-faithful reference.

### 9.1 Signup (`/signup`)

- `POST /v1/auth/signup` (human) or `POST /v1/agents/register` (agent shortcut).
- Role picker writes `role` into the resulting User row.
- SSO is reserved; the screen shows the button but it's not wired.

### 9.2 Library (`/library`)

- Lists current user's accessible quizzes via `GET /v1/quizzes?course=<currentTerm>`.
- Tabs filter client-side over a single fetch.
- The "Up next" hero is the highest-priority assigned quiz (`assigned=true` + earliest deadline + not completed).
- The hero renders `<LearningObjectives items={quiz.objectives} />` between description and the CTA row.
- Each card's accent stripe reads `quiz.color`.
- Share triggers: a `<ShareButton size="md" variant="ghost" payload={ kind:'quiz', id }>` sits next to Preview on the hero; each library card footer carries a small "↑ Share" link with the same payload.

### 9.3 Exams (`/exams`)

- `GET /v1/exams?course=<currentTerm>`.
- Selecting a row triggers `GET /v1/exams/{id}` for the detail panel — composition trace lives there.
- The detail panel renders `<LearningObjectives items={exam.objectives} />` between the stat strip and the Composition section.
- Manual exams render an `composedBy` row; agent exams render the `compositionTrace` JSON block in mono.
- The CTA opens a `session.create` with `{ examId }`. A `<ShareButton payload={ kind:'exam', id }>` sits adjacent to the CTA.

### 9.4 Quiz (`/quiz/{quizId}/setup` → `/quiz/{quizId}`)

- Stage A (setup): purely client-state until Start.
- Start → `POST /v1/sessions` with the setup body. The agent-equivalent panel echoes the same JSON the form would post.
- Stage B (active): on mount, the questions come from `session.questions[]`. Autosave fires `POST /v1/sessions/{id}/answer` per question per 8s window (debounced). Submit → `POST /v1/sessions/{id}/finish` → redirect to Results.
- Timer is server-anchored: client renders `deadline_at - now`, never trusts its own clock for the deadline.
- Question palette uses session-local IDs; server doesn't know "flagged" — that's purely a client annotation in localStorage keyed by `attemptId`.

### 9.5 Results (`/sessions/{sessionId}/result`)

- `GET /v1/attempts?session={id}` to render per-item review.
- Donut + histogram use the same data as `GET /v1/quizzes/{quizId}/stats`.
- The "View 6-week plan" CTA hits `POST /v1/plans` then redirects to the Dashboard with the plan id selected.
- Share triggers: a header-level "Share quiz" button uses `payload={ kind:'quiz', id }`; each per-item review row carries a Share with `payload={ kind:'item', id, explanation }` — the explanation rides along so the **knowledge travels**, not just a marketing tease.

### 9.6 Progress / Dashboard (`/progress`)

- Renders progressively based on user setting `statsDepth ∈ minimal|standard|full`.
- Three queries: `GET /v1/users/me/trend`, `GET /v1/users/me/subjects`, `GET /v1/users/me/cohort-distribution`. These are not in the public agent surface — they're SSR-only with cookie auth.
- The IRT scatter (`full` mode) is computed offline; the dashboard reads a precomputed `item_analysis` row per quiz.
- The mastery map heatmap is `color-mix(in oklch, accent X%, surface-2)` — see §10.

### 9.7 Author studio (`/author/{quizId}`)

- Three-pane layout. Left pane = `GET /v1/quizzes/{id}` (questions list); centre = active question editor; right = distribution + rubric + activity.
- All edits flow through `PATCH /v1/quizzes/{id}` and `PATCH /v1/questions/{id}` (the latter is a question-bank endpoint inherited from Plan 3, still exposed under `/v1/questions/*`).
- The metadata strip includes an objectives editor (multi-line, drag-reorderable, 3-4 bullets typical). Persisted via `PATCH /v1/quizzes/{id}` with `{ objectives: string[] }`. Server warns at >6 entries (§5.2).
- The per-question editor exposes a `points` input (default 1) alongside tag and difficulty. Persisted via `PATCH /v1/questions/{id}` with `{ points }`.
- The "Generate questions" callout (left pane footer) opens a dialog that calls `POST /v1/quizzes/generate` with the current quiz pre-filled as `courseId`. The response's `objectives[]` (if any) preview-merges into the metadata strip with an accept/reject prompt.
- Publish → `PATCH /v1/quizzes/{id}` with `{ status: 'active' }`. Server checks: ≥1 question, all questions `status='live'`.

### 9.8 Agent integration (`/agent`)

- Instructor / admin only.
- Tabs: Keys, MCP tools, Import demo, Recent activity.
- Keys list = `GET /v1/me/keys`. Create / rotate / revoke = corresponding endpoints.
- MCP tools list = parsed `GET /v1/agents/mcp.json`. Selecting a tool renders the descriptor + an inline curl example.
- Import demo = a live `POST /v1/quizzes` against the user's own key. Response panel shows status, latency, body. Successful imports link to the new quiz.
- Recent activity = `GET /v1/agents/activity?limit=50`.

---

## 10. Design system

Authoritative source: `design/tokens.md`. This section gives backend / API engineers the bits they actually touch.

- **Three themes** switched by `data-theme` on `<html>`: `slate` (default, dark), `paper` (light academic), `cobalt` (deep blue + gold).
- **All colour values are CSS variables.** No hex literals in TSX. The Tailwind config maps `--accent` → `colors.accent.DEFAULT`, `--surface-2` → `colors.surface.2`, etc.
- **Type families** are loaded via `next/font` once: `Source Serif 4`, `Inter`, `JetBrains Mono`. No second weight load on screen transitions.
- **Spacing** is a 4/2 scale (see tokens.md §Spacing). No 8pt grid.
- **Iconography:** stroke-width 1.6, 16px default, line-only. The `Icon` component is centralised; never `<img src="…icon.svg" />` for UI chrome.
- **Theme persistence:** `localStorage.harus.theme` mirrors the user's setting; defaults to `slate`; respects `prefers-color-scheme` only if the user has never set a value.

### 10.1 Cross-cutting components

These are not screens — they overlay any screen and ship as global providers.

- **Share modal** (`design/source/src/share.jsx`): a global `<ShareProvider>` wraps the app at the root. `useShare()` exposes `open(payload)`; `<ShareButton payload={…} />` is the inline trigger. The modal has four tabs (Link / Socials / Card / Embed) plus a right rail of "what travels with it" toggles and a privacy segmented control. Payload kinds: `quiz`, `exam`, `item` (item-kind payloads carry the explanation). All mutations route through `POST /v1/shares`.
- **LearningObjectives** (`design/source/src/share.jsx`, top): a reusable component with two modes — default 2-column grid (mono "01"/"02" indices, accent-bar left rails) and `compact` single-column with check icons. Reads `quiz.objectives` or `exam.objectives`.

### 10.2 Runtime preferences

Two runtime user preferences ship in v1:

- `theme: 'slate' | 'paper' | 'cobalt'` — applied via `data-theme` on `<html>`, persisted in `localStorage.harus.theme`.
- `statsDepth: 'minimal' | 'standard' | 'full'` — controls Dashboard rendering depth (§9.6).

The **Tweaks panel** (`design/source/tweaks-panel.jsx`) exposes both, plus an "Agent panel" toggle that hides/shows the Agent sidebar entry. The panel itself is **demo-only**; in production the agent sidebar entry is role-gated (instructor / admin only) and the toggle is removed.

---

## 11. Accessibility

- All interactive elements are `<button>` or `<a>`. No clickable `<div>`s.
- `:focus-visible` rings on every interactive element: `outline: 2px solid var(--accent); outline-offset: 2px`.
- Question palette buttons carry `aria-current="step"` for the current question and `aria-label="Question 4 — answered, flagged"`.
- Timer is `role="timer" aria-live="off"` until the last 5 minutes, then `aria-live="polite"`.
- Code editor: real `<label>`, `spellCheck="off"`.
- Contrast: tokens are tuned for WCAG AA on all three themes — Paper is the tightest; re-check after any token change.
- Reduced-motion: respect `prefers-reduced-motion: reduce` and disable card hover transforms + progress-bar transitions.

---

## 12. Out of scope (v1)

- Real SSO / SAML wiring (the button is decorative).
- Production proctor integrations.
- Payment / billing.
- Email channel for `feedback.send`.
- Mobile layouts (<1024px).
- Real LLM judge for `essay` / free-text questions (the `judge: 'llm'` value is reserved but unused).
- Real code-execution sandbox for `code` questions (v1 ships with an exemplar-match stub; the spec leaves the field in place).
- Multi-tenant org isolation beyond per-user / per-cohort scoping.
- Real-time collaboration in the Author studio.
- Production OG image generator: v1 may ship a stub PNG generator or a minimal Satori / Vercel-OG worker behind `share_links.og_image_url`. A polished social-card pipeline (per-quiz branding, per-cohort templates) is deferred.
- `share.created` / `share.attempted` webhooks: reserved but not emitted in v1.

---

## 13. Migration notes (May-19 → 2026-05-20)

This redesign is not a strict superset of the May-19 spec. Specifically:

| May-19                                                                            | Now                                                             | Migration                                                                                                                                                                                             |
| --------------------------------------------------------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `QuestionKind ∈ {mcq, free_text, cloze}`                                          | `kind ∈ {mc, tf, short, essay, code}`                           | Rename `mcq → mc`, `free_text → short`. `cloze` is dropped from v1 (no design for it); preserve existing rows as `status='archived'`. `tf`, `essay`, `code` are net-new.                              |
| Session is the only assessment entity                                             | Quiz, Exam are first-class                                      | New tables `quizzes`, `exams`, `exam_sections`, `quiz_questions`. Sessions stay; their `quiz_id` / `exam_id` become FKs.                                                                              |
| `Scope ∈ {human, agent:write-questions, agent:read-only}`                         | Eight scopes (§6.2)                                             | The three legacy scopes map: `human` → all role-derived scopes; `agent:write-questions` → `quiz.write`; `agent:read-only` → `quiz.read + attempt.read + stats.read`. Migration backfills accordingly. |
| `affects_rating` per session, default true                                        | Same, but with rule in §5.2                                     | Existing data unaffected.                                                                                                                                                                             |
| `correct_index` on MCQ payload                                                    | Same (kept)                                                     | No change.                                                                                                                                                                                            |
| `needs_work_score` (corrected sign on 2026-05-19)                                 | Same formula, applies to per-tag deltas                         | No change.                                                                                                                                                                                            |
| Three-letter `level_label` resolution                                             | Replaced by `difficulty ∈ intro/inter/adv` on Question and Quiz | Existing tag-level ratings keep their float values; UI maps to bands at render time.                                                                                                                  |
| `Role ∈ {admin, user}`                                                            | `Role ∈ {learner, instructor, admin, agent}`                    | Backfill: `user → learner` if they've created an attempt, `user → instructor` if they've authored a question, else `learner`. `admin` unchanged.                                                      |
| No `Course`, `Cohort`, `ApiKey`, `Webhook`, `ActivityLog`, `StudyPlan`, `Message` | All net-new                                                     | Plan 2 of the new ledger creates these tables.                                                                                                                                                        |
| No `ShareLink`, `anonymous_attempts`                                              | Net-new                                                         | Plan 8 ledger creates these tables; no May-19 ancestor to migrate.                                                                                                                                    |
| Question has no `points`                                                          | `points: int (default 1)`                                       | Backfill all existing questions to `points = 1`. UI displays `points` everywhere; per-attempt `score` is now `points × correctness_fraction`.                                                         |
| Quiz / Exam have no `objectives`                                                  | `objectives: text[]` on both                                    | Backfill to `'{}'::text[]` (empty). Author studio prompts to fill on next edit; agents on `quiz.generate` populate going forward.                                                                     |
| Channel enum mismatch between design (`'in-app'`) and spec                        | Canonical: `'in_app'` (snake_case, consistent with §5.1)        | `design/api.md` will be updated to match; no DB data to migrate (Message is net-new).                                                                                                                 |

The May-19 spec file (`docs/specs/2026-05-19-question-exam-platform-design.md`) stays on disk and is **not** modified. Cross-references in code comments to the May-19 spec become stale once Plan 2 lands; sweep them in the same commit that introduces the new schema.

---

## 14. Implementation order

Phases live in `docs/plans/tasks/phases.jsonl` and `docs/plans/2026-05-20-plan-{N}-<topic>.md`. Headline order (10 phases):

1. **P1 — Foundations & repo cleanup** — _done_ on `feat/foundation`. Toolchain, Make targets, lint hooks, basic Axum + sqlx skeleton.
2. **P2 — Schema rewrite & auth refactor** — new entity tables, scope enum widening, `api_keys` + `/agents/register` shortcut.
3. **P3 — Question bank (5 kinds)** — extends current `questions` to add `tf`, `essay`, `code`; payload validators; Author studio HTTP routes.
4. **P4 — Quiz authoring** — `quizzes` table, `quiz_questions` join, full CRUD, publish gating.
5. **P5 — Engine (Elo, planner, graders)** — port current spike work; add graders for `tf`/`essay`/`code` (essay returns "pending manual"; code stubs an exemplar match).
6. **P6 — Exams & composition** — `exams`, `exam_sections`, `compose` endpoint, composition trace persistence.
7. **P7 — Stats & feedback** — `/quizzes/{id}/stats`, `/exams/{id}/stats`, `/messages`, item analysis precompute.
8. **P8 — Agent surface & sharing** — MCP manifest export, ActivityLog, webhooks, `/plans`, and the Share API (`/v1/shares`, `/v1/{kind}/{id}/embed`, anonymous-attempt path, OG image stub). Share is folded into P8 because `share.create` / `share.get` are MCP tools — a separate P11 is reserved if scope grows (e.g. branded social-card pipeline).
9. **P9 — Frontend learner path** — Library, Quiz setup, active Quiz, Results, Progress.
10. **P10 — Frontend author + agent** — Author studio, Exams screen, Agent integration screen.

Each phase has a matching task ledger under `docs/plans/tasks/plan-N-<topic>.jsonl`. Phase boundaries are commit boundaries; cross-phase work needs a written rationale in the phase ledger.

---

## 15. When in doubt

- Trust this spec over the May-19 spec.
- Trust `design/tokens.md` over any local copy of token values.
- Trust `design/source/src/*.jsx` over paraphrased visual descriptions — including this document's §9.
- For anything not covered: file a question in `docs/plans/tasks/open-questions.jsonl` (create the file on first use) and proceed with the most conservative interpretation, leaving a TODO with the question id.
