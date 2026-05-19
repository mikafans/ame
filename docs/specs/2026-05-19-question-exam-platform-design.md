# Question Collector + Exam Platform — Design Spec

**Date:** 2026-05-19
**Status:** Draft, awaiting user review
**Source notes:** `docs/v0/draft.md`, `docs/v0/decisions.md`, `docs/v0/research-notes.md`
**Location:** `docs/specs/2026-05-19-question-exam-platform-design.md`

## Goal

A personal platform for collecting questions across diverse knowledge domains
(Rust, Zig, Spanish, piano, …) and self-administering quizzes and exams whose
results map to a calibrated *level* per domain.

The motivation: after reading a book, the reader has no honest signal of their
actual proficiency. This platform replaces self-assessment by feel with a
self-assessment by evidence — measured against an item pool that external
LLM-driven agents continuously expand and target at the user's current weak
areas.

## Scope

### MVP (this spec)

1. **Question bank** — CRUD on a typed, versioned question store with flat
   colon-namespaced tags (`rust:async`, `spanish:a2`).
2. **Agent ingestion** — REST endpoint for external agents (claude-code,
   scripts) to POST batches of questions. Backend holds no LLM API keys.
3. **Quiz runner** — ad-hoc quiz: pick tags + difficulty range, answer N
   questions, see immediate feedback and per-tag rating deltas.
4. **Exam runner** — first-class exam objects with saved blueprints (static or
   dynamic), section weights, optional time limit, hidden per-question
   feedback by default, structured result reports.
5. **Level readout** — per-tag Elo rating (self-calibrating) with optional
   curriculum-style labels (e.g. CEFR A2 for Spanish, `intermediate` for Rust)
   layered on top.
6. **Stats endpoints for agents** — weakest-tags, hardest-failed questions,
   recent attempts. These enable a closed feedback loop:
   *agent reads gaps → generates targeted questions → user attempts them →
   ratings update → agent re-reads.*

### Out of MVP (designed-compatible)

Spaced repetition (FSRS), Glicko-2 rating deviation, LLM-as-judge for
free-text, magic-link/OAuth auth, xAPI emission, QTI import/export, hints,
audit log, per-question exam timers, mobile app, true multidimensional Elo
(M-ERS).

Each is additive: the schema, APIs, and module boundaries leave room.

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│  NAS, reachable over Tailscale                                     │
│                                                                    │
│  Next.js  ──────┐                                                  │
│   /*  public  HTTP/JSON                                            │
│   /admin/*    │                                                    │
│               ▼                                                    │
│             Axum API ◀─────── external agents (claude-code, …)    │
│               │              ▲   read  /me/stats, /weakest-tags    │
│               ▼              │   write /questions                  │
│             Postgres         │   scoped tokens (no /sessions write)│
│             (Docker)         │                                     │
│                                                                    │
│  Repo also ships:                                                  │
│    agents/SKILL.md bundle (agent how-to docs)                      │
│    api/openapi.yaml         (machine-readable contract)            │
└────────────────────────────────────────────────────────────────────┘
```

### Components

- **`api/`** — Rust, single Axum binary. Talks to Postgres via `sqlx`
  (compile-time-checked queries). Owns all business logic: grading, Elo
  updates, exam blueprint resolution, level mapping.
- **`web/`** — Next.js 15 App Router, TypeScript. One deployment, public
  `/*` routes for taking, `/admin/*` routes gated on `users.role = 'admin'`.
- **Postgres** — Docker container for local dev, Kubernetes for production. Schema-versioned via `sqlx migrate`
  with migrations checked into `db/migrations/`.
- **Auth** — opaque API tokens (argon2id-hashed at rest) carried as
  `Authorization: Bearer …`. Each token has one or more *scopes* selected
  from `human`, `agent:write-questions`, `agent:read-only`. Token plaintext
  is shown once at issue, never stored.
- **Agent skill bundle** (`agents/`) — markdown skills following the
  Anthropic skills convention, plus the OpenAPI spec, version-controlled
  alongside the API. Agents read these to learn the system contract.

### Internal module layout (`api/src/`)

```
bank/        questions, versions, tags, draft/promote workflow
engine/      sessions, attempts, grading, Elo updates
assess/      exam blueprints, blueprint resolution, result computation
stats/       per-tag analytics, weakest-tag, recent attempts
auth/        token issuance, scope-gating middleware
http/        Axum handlers, OpenAPI annotations (utoipa)
domain/      shared types (Question, Attempt, Rating, …)
```

Dependency rule: `engine/` and `assess/` may depend on `bank/` and
`domain/`. The reverse is forbidden. `bank/` does not know that attempts
exist.

## Data Model

Postgres 18. All tables use `uuidv7()` for primary keys (time-ordered →
better B-tree locality). All timestamps are `timestamptz`. Elo ratings
are `double precision`.

Four `CHECK` constraints in the entire schema, all guarding values the
code branches on or data that's painful to clean up later. No paranoid
range checks.

### users / auth

```sql
create table users (
  id            uuid primary key default uuidv7(),
  email         text,                                       -- nullable until magic-link auth lands
  display_name  text not null,
  role          text not null,                              -- 'admin' | 'user' (app-enforced)
  created_at    timestamptz not null default now()
);

create table api_tokens (
  id            uuid primary key default uuidv7(),
  user_id       uuid not null references users(id) on delete cascade,
  name          text not null,                              -- "claude-code laptop", "phone"
  token_hash    text not null,                              -- argon2id
  scopes        text[] not null,
  last_used_at  timestamptz,
  revoked_at    timestamptz,
  created_at    timestamptz not null default now(),
  constraint api_tokens_scopes_check
    check (scopes <@ array['human','agent:write-questions','agent:read-only']::text[]
           and array_length(scopes, 1) >= 1)
);
create index api_tokens_user_active
  on api_tokens(user_id) where revoked_at is null;
```

### tags

```sql
create table tags (
  id            uuid primary key default uuidv7(),
  name          text not null unique,                       -- stored lowercased
  description   text,
  created_at    timestamptz not null default now(),
  constraint tags_name_lowercase check (name = lower(name))
);
```

Tag names use a colon-namespaced convention (`rust`, `rust:async`,
`spanish:a2`). Pseudo-hierarchy without a tree. Lazy-created on first POST
that references the name.

### questions, with versioning

```sql
create table questions (
  id              uuid primary key default uuidv7(),
  kind            text not null,
  prompt          text not null,                            -- markdown
  code_snippet    jsonb,                                    -- { language, body, caption? }
  payload         jsonb not null,                           -- shape varies by kind
  explanation     text,
  status          text not null default 'draft',
  source          text,                                     -- "Rust Book ch.10", agent prompt, etc.
  rating          double precision not null default 1400,   -- Elo
  attempts_count  integer not null default 0,
  version         integer not null default 1,
  created_by      uuid not null references users(id),
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now(),
  constraint questions_kind_check
    check (kind in ('mcq', 'free_text', 'cloze')),
  constraint questions_status_check
    check (status in ('draft', 'live', 'archived'))
);
create index questions_live_created on questions(created_at desc) where status = 'live';
create index questions_status        on questions(status);

create table question_versions (
  question_id   uuid not null references questions(id) on delete cascade,
  version       integer not null,
  prompt        text not null,
  code_snippet  jsonb,
  payload       jsonb not null,
  explanation   text,
  archived_at   timestamptz not null default now(),
  primary key (question_id, version)
);

create table question_tags (
  question_id   uuid not null references questions(id) on delete cascade,
  tag_id        uuid not null references tags(id) on delete restrict,
  primary key (question_id, tag_id)
);
create index question_tags_tag on question_tags(tag_id, question_id);
```

**Code questions are not a separate kind.** A "code question" is an MCQ,
free-text, or cloze question with `code_snippet` set. The renderer shows
the snippet above the prompt for any kind.

**Versioning rule:** editing a `live` question copies the previous
`prompt`/`code_snippet`/`payload`/`explanation` into `question_versions` and
bumps `questions.version`. `attempts` records the version it scored against.
Editing a `draft` question does not version (it's not in flight).

### Payload shapes

```jsonc
// mcq
{ "options": ["...", "...", "..."], "correct_index": 1 }

// free_text
{
  "accepted": ["café", "cafe"],
  "normalize": "case_insensitive_strip_accents",   // or "exact"
  "judge": "exact"                                  // future: "llm"
}

// cloze
{
  "template": "El {{1}} fui al {{2}}.",
  "blanks": [["año"], ["mercado", "supermercado"]],
  "normalize": "case_insensitive_strip_accents"
}
```

**Normalize semantics (free_text + cloze).** The same `normalize` function
is applied to *both* sides before comparison: each entry in `accepted` (or
each entry in a cloze `blanks[i]`) and the user's response are normalized,
then compared for equality. `exact` is the identity function. The grader
returns correct if any normalized accepted form matches the normalized
response. Normalization happens at grade time, not at write time — stored
strings keep their original form.

### Ratings

```sql
create table user_tag_ratings (
  user_id         uuid not null references users(id) on delete cascade,
  tag_id          uuid not null references tags(id) on delete restrict,
  rating          double precision not null default 1200,
  attempts_count  integer not null default 0,
  last_updated    timestamptz not null default now(),
  primary key (user_id, tag_id)
);
create index user_tag_ratings_user_rating on user_tag_ratings(user_id, rating);
```

### Sessions (unified for quiz and exam)

```sql
create table exams (
  id            uuid primary key default uuidv7(),
  name          text not null,
  description   text,
  blueprint     jsonb not null,                              -- static or dynamic
  time_limit_seconds  integer,                               -- nullable: no limit
  show_results_during boolean not null default false,
  affects_rating      boolean not null default true,
  status        text not null default 'draft',               -- 'draft' | 'published' | 'archived'
  created_by    uuid not null references users(id),
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);

create table sessions (
  id              uuid primary key default uuidv7(),
  user_id         uuid not null references users(id) on delete cascade,
  kind            text not null default 'quiz',              -- 'quiz' | 'exam'
  exam_id         uuid references exams(id),
  filter          jsonb,                                     -- quiz: filter at start time
  question_plan   jsonb not null,                            -- both: ordered list of planned items
                                                             --   { items: [{ question_id, version,
                                                             --     section?, option_order? }, ...] }
  status          text not null default 'in_progress',       -- 'in_progress' | 'finished' | 'abandoned'
  affects_rating  boolean not null default true,             -- copied from exams.affects_rating, or from /quiz body
  rating_snapshot jsonb not null default '{}'::jsonb,        -- { "<tag_id>": <rating>, ... } captured at session creation
                                                             -- for tags in the question_plan; powers result.rating_changes
  result          jsonb,                                     -- exam: scoring report set on finish
  deadline_at     timestamptz,                               -- start + time_limit_seconds (exams only)
  started_at      timestamptz not null default now(),
  finished_at     timestamptz,
  constraint sessions_exam_link
    check ((kind = 'exam' and exam_id is not null)
        or (kind = 'quiz' and exam_id is null))
);
create index sessions_user_started on sessions(user_id, started_at desc);
```

### Attempts (with presentation state)

```sql
create table attempts (
  id                       uuid primary key default uuidv7(),
  user_id                  uuid not null references users(id) on delete cascade,
  question_id              uuid not null references questions(id) on delete restrict,
  question_version         integer not null,
  session_id               uuid references sessions(id) on delete set null,
  response                 jsonb not null,
  presentation             jsonb not null default '{}'::jsonb,   -- e.g. { "option_order": [2,0,3,1] }
  is_correct               boolean not null,
  score                    double precision not null,            -- 0..1
  time_to_answer_ms        integer,
  rating_before_user_avg   double precision not null,
  rating_before_question   double precision not null,
  user_tag_deltas          jsonb not null,                       -- { "rust:async": +12.4, ... }
  question_delta           double precision not null,
  created_at               timestamptz not null default now()
);
create index attempts_user_created on attempts(user_id, created_at desc);
create index attempts_question     on attempts(question_id, created_at desc);
create index attempts_session      on attempts(session_id) where session_id is not null;
```

`presentation` records anything stochastic shown to the user. For MCQ that
means the order options were displayed in. The client posts a
`selected_position` (index into the *shown* order); the backend resolves
that against `presentation.option_order` to determine the canonical
`correct_index`.

**Trust model:** the server generates the shuffle when a question is
served (either in `POST /quiz` / `POST /exams/:id/start` for the first
question, or in the `next_question` of a `POST /sessions/:id/answer`
response) and persists it in the corresponding entry of
`sessions.question_plan`. The client renders options in whatever order
the server provided and reports the chosen *position*. The server
resolves the position against its own stored `option_order` — never
trusting a client-supplied shuffle. On attempt insert, the stored order
is copied into `attempts.presentation` for audit.

### Level mappings (admin-curated, optional)

```sql
create table level_mappings (
  id            uuid primary key default uuidv7(),
  tag_pattern   text not null,                              -- exact tag or 'spanish:*'
  label         text not null,
  elo_min       double precision not null,
  elo_max       double precision not null,
  sort_order    integer not null default 0,
  created_at    timestamptz not null default now()
);
create index level_mappings_pattern on level_mappings(tag_pattern);
```

Tags with no matching mapping just show the raw Elo number. Mappings are
admin-managed in `/admin/level-mappings`.

### Idempotency keys

```sql
create table idempotency_keys (
  token_id        uuid not null references api_tokens(id) on delete cascade,
  key             text not null,
  request_hash    text not null,                              -- sha256 of method+path+body
  response_status smallint not null,
  response_body   jsonb not null,
  created_at      timestamptz not null default now(),
  primary key (token_id, key)
);
create index idempotency_keys_created on idempotency_keys(created_at);
```

Populated by middleware on POST endpoints that opt in (currently
`POST /questions`, `POST /sessions/:id/answer`). On replay:
- same `(token_id, key)` + same `request_hash` → return stored response;
- same `(token_id, key)` + different `request_hash` → `409 idempotency_conflict`.

Rows older than 24h are deleted by a periodic cleanup job (`cron` or a
tokio task; spec doesn't pin).

### ER summary

```
users ─┬─< api_tokens ──< idempotency_keys
       ├─< attempts >─ questions ─< question_tags >─ tags
       │                  │
       │                  └─< question_versions
       ├─< user_tag_ratings ─────────────────────── tags (PK)
       └─< sessions ─< attempts
              └─ exams (nullable, when kind='exam')

level_mappings   (free-standing)
```

## Elo Update Rule

Single canonical implementation in `engine/elo.rs`. Called once per attempt
inside the same DB transaction that inserts the row.

```
On answer to question Q with tags [T1, ..., Tn], score s ∈ [0,1]:

  user_avg     = mean(user_tag_ratings[Tᵢ].rating, default=1200)
  expected     = 1 / (1 + 10^((Q.rating - user_avg) / 400))

  // calibration phase: protect user rating while question rating settles
  if Q.attempts_count < 20:
    K_user     = 0
    K_question = 48
  else:
    K_user     = 32 if min(user_tag_ratings[Tᵢ].attempts_count) < 30 else 16
    K_question = 16

  delta_user_total = K_user     * (s - expected)
  delta_question   = K_question * (expected - s)

  for Tᵢ in tags:
    user_tag_ratings[Tᵢ].rating += delta_user_total / n     // M-ERS-lite splitting
    user_tag_ratings[Tᵢ].attempts_count += 1
  Q.rating += delta_question
  Q.attempts_count += 1

  // attempts row records snapshots and per-tag deltas
```

Properties verified by property tests:
- ratings stay bounded (no NaN, no infinity even with degenerate inputs);
- a correct answer never decreases the user's rating;
- a wrong answer never increases the user's rating;
- update is symmetric: `Δuser_total = -Δquestion` (energy-conserving).

Quiz sessions where `affects_rating = false` (or exams flagged the same)
skip the Elo update entirely — `attempts` still records what would have
been awarded under `user_tag_deltas` / `question_delta` so the UI can
show a "ghost result", but `user_tag_ratings` and `questions.rating` are
not touched.

## Exam Blueprints

Two `blueprint.type` shapes:

```jsonc
// Static: pinned questions. Same every attempt. Like a midterm.
{
  "type": "static",
  "sections": [
    { "name": "Ownership", "question_ids": ["…", "…"], "weight": 1.0 }
  ]
}

// Dynamic: sampling criteria. Fresh items each attempt; shape is comparable.
{
  "type": "dynamic",
  "sections": [
    {
      "name": "Async",
      "tags": ["rust:async"],
      "tags_mode": "any",                    // "any" | "all"
      "count": 5,
      "difficulty_min": 1300,
      "difficulty_max": 1700,
      "weight": 1.5
    }
  ],
  "exclude_recent_hours": 168       // per-user: skip questions this user
                                    // attempted in the last N hours
}
```

### Session planning (`question_plan`)

Both quizzes and exams build a `question_plan` upfront at session creation
and persist it on `sessions.question_plan`. The plan is an ordered list of
items, each containing `question_id`, `version` (the live version at plan
time — attempts pin to this), and any precomputed presentation state
(MCQ `option_order` is server-generated here, never client-generated). For
exams each item also carries its `section` name.

- **`POST /quiz`** → sample without replacement from `questions where status='live'`
  matching the filter (tags + `tags_mode`, difficulty range,
  `exclude_recent_hours` — per-user, i.e. skip questions *this user* has
  attempted in the last N hours), shuffle to `count` items. If the filter
  produces fewer than `count` matches, return what's available (no error)
  with a warning in the response:
  `{ requested: 10, planned: 7, reason: "pool_short" }`.
- **`POST /exams/:id/start`**:
  - **Static blueprint** → copy `question_ids` into plan items in order.
    If a pinned question is currently `archived`, fail with
    `error.code = "exam_pool_insufficient"` and refuse to start.
  - **Dynamic blueprint** → sample without replacement per section,
    honoring tags/difficulty/`exclude_recent_hours`. If any section can't
    be filled, return `422 exam_pool_insufficient` with
    `{ section, required, available }` — the agent's signal to generate
    more questions for the gap.

The next question served is always `question_plan.items[answered_count]`.
This makes resume after a closed tab trivial: server re-renders item N+1
from the existing plan; nothing is regenerated.

### Result shape (written into `sessions.result` at finish)

```jsonc
{
  "score_overall": 0.78,
  "sections": [
    { "name": "Ownership", "score": 0.85, "correct": 4, "total": 5, "weight": 1.0 },
    { "name": "Async",     "score": 0.60, "correct": 2, "total": 3, "weight": 1.5 }
  ],
  "duration_seconds": 1240,
  "rating_changes": [{ "tag": "rust:ownership", "before": 1450, "after": 1481 }],
  // `before` = rating at session creation (snapshotted into sessions row);
  // `after`  = rating at finish. Only tags actually touched by attempts in
  // this session appear. Omitted entirely when affects_rating = false.
  "level_label": "intermediate"
}
```

`score_overall` = weighted sum of section scores ÷ total weight.

`level_label` is resolved against `level_mappings` as follows:
1. For each tag referenced in the exam blueprint, sum the weight of every
   section it appears in.
2. Pick the tag with the highest cumulative weight (ties broken by tag
   name, ascending — deterministic).
3. Look up that tag in `level_mappings` (exact match first, then `prefix:*`
   patterns); return the matching label.
4. If no mapping matches, omit `level_label` from the result.

Static blueprints have no `tags` field on sections; in that case derive
the tag set from the pinned questions' `question_tags`.

### Edge cases

- **Static exam references an archived question** → exam auto-marked
  `status='draft'` until re-curated. Existing attempts unaffected.
- **Time limit exceeded mid-exam** → backend rejects further answers with
  `error.code = "exam_expired"`. A `POST /sessions/:id/finish` (or the
  scheduled timeout job) finalizes the session with whatever answers exist.
- **Closed-tab resumption** → session stays `in_progress` until the user
  hits `/sessions/:id` again *and* the deadline (if any) hasn't passed.
  Stale quiz sessions auto-abandon after 24h with no activity.

## API Surface

JSON, `snake_case`, Bearer-token auth, scope-gated. Errors:
`{ error: { code, message, details? } }`. Cursor pagination on lists.
Optional `Idempotency-Key` header on POSTs (TTL 24h, stored in
`idempotency_keys` with `(token_id, key) → response_hash`). OpenAPI spec
served at `/openapi.json` and checked into the repo as `api/openapi.yaml`,
generated by `utoipa`.

### Scope matrix

| Endpoints | `human` | `agent:write-questions` | `agent:read-only` |
|---|:---:|:---:|:---:|
| `/me`, `/me/stats`, `/me/weakest-tags`, `/me/recent-attempts`, `/me/exam-attempts`, `/tags/*/stats` | ✓ | ✓ | ✓ |
| `GET /questions`, `GET /tags`, `GET /exams`, `GET /levels` | ✓ | ✓ | ✓ |
| `POST /questions`, `PATCH /questions/:id`, `POST /questions/:id/promote`, `POST /questions/:id/archive` | ✓ | ✓ | ✗ |
| `POST /quiz`, `POST /exams/:id/start`, `POST /sessions/:id/answer`, `POST /sessions/:id/finish` | ✓ | ✗ | ✗ |
| `/admin/**` | admin role | ✗ | ✗ |

### Endpoint list

```
# identity (any token)
GET    /me
GET    /me/tokens                       # own tokens
POST   /me/tokens
DELETE /me/tokens/:id

# tags
GET    /tags?q=&limit=
POST   /tags                             # idempotent; lazy-create on POST /questions also works

# questions
GET    /questions?tag=&status=&difficulty_min=&difficulty_max=&q=&cursor=&limit=
GET    /questions/:id
POST   /questions                        # batch, max 50/call
PATCH  /questions/:id                    # edits to 'live' auto-version
POST   /questions/:id/promote            # draft → live
POST   /questions/:id/archive

# quiz (creates a session with kind='quiz')
POST   /quiz                             # body: tags, difficulty range, count, etc.

# exams
GET    /exams                            # 'published' only for non-admin
GET    /exams/:id
POST   /admin/exams
PATCH  /admin/exams/:id
POST   /admin/exams/:id/publish
POST   /admin/exams/:id/archive
POST   /exams/:id/start                  # creates session with kind='exam'

# sessions (unified runner)
GET    /sessions/:id
POST   /sessions/:id/answer
POST   /sessions/:id/finish

# stats (agent-readable)
GET    /me/stats
GET    /me/weakest-tags?limit=
GET    /me/recent-attempts?limit=
GET    /me/exam-attempts?exam_id=&cursor=&limit=    # exam_id optional: omit to list across all exams
GET    /tags/:name/stats
GET    /levels

# admin
GET/POST/PATCH/DELETE /admin/level-mappings
GET    /admin/users
```

### Canonical request/response examples

`POST /questions` — the agent ingestion contract:

```jsonc
// Request (batch, partial success allowed)
{
  "questions": [
    {
      "kind": "mcq",
      "prompt": "What does `Arc::clone` do?",
      "code_snippet": { "language": "rust", "body": "let a = Arc::clone(&x);" },
      "payload": { "options": ["…", "…", "…", "…"], "correct_index": 2 },
      "explanation": "Increments the strong count …",
      "tags": ["rust", "rust:concurrency"],
      "difficulty_initial": 1500,
      "source": "agent: weakest-tag loop 2026-05-19",
      "status": "draft"
    }
  ]
}

// Response
{
  "created": 1,
  "questions": [{ "id": "01HX…", "status": "draft" }],
  "errors": []
}
```

`POST /quiz`:

```jsonc
{
  "tags": ["rust"],
  "tags_mode": "any",
  "difficulty_min": 1200,
  "difficulty_max": 1800,
  "count": 10,
  "exclude_recent_hours": 24,
  "affects_rating": true            // optional, default true; false = practice mode
}

// → { "session_id": "01HX…", "remaining": 10, "question": { /* correct_index stripped */ } }
```

`POST /sessions/:id/answer`:

```jsonc
// Request — MCQ response uses position-in-shown-order
{
  "question_id": "01HX…",
  "response": { "selected_position": 0 },
  "time_to_answer_ms": 4500
}

// Response
{
  "is_correct": true,
  "score": 1.0,
  "correct_answer": { "correct_index": 2 },        // canonical index in the stored options
  "explanation": "…",
  "user_tag_deltas": { "rust": 8.4, "rust:concurrency": 8.4 },
  "question_delta": -8.4,
  "level_changes": [{ "tag": "rust", "old_label": "novice", "new_label": "intermediate" }],
  "next_question": { /* same shape as POST /quiz response */ } | null,
  "session_status": "in_progress" | "finished"
}
```

For exam sessions where `show_results_during = false`, `is_correct`,
`score`, `correct_answer`, `explanation`, and `*_delta` fields are
**omitted** from the per-answer response. The full breakdown is only
revealed at `POST /sessions/:id/finish`.

`GET /me/weakest-tags?limit=5` — the agent loop's entry point:

```jsonc
{
  "tags": [
    {
      "tag": "rust:async",
      "rating": 1320,
      "attempts": 24,
      "accuracy_recent": 0.42,            // last 10 attempts
      "last_attempt_at": "2026-05-18T12:00:00Z",
      "current_label": "novice",
      "needs_work_score": 0.78            // see below
    }
  ]
}
```

`needs_work_score` heuristic:
`0.5 * (1 - rating_z) + 0.3 * (1 - accuracy_recent) + 0.2 * recency_factor`
where `rating_z` is the user's rating on this tag rescaled to [0,1] across
all their tags (so `1 - rating_z` is large for weak tags),
`accuracy_recent` is on the last 10 attempts (`1 - accuracy` is large when
the user is missing), and `recency_factor` is large for tags the user
hasn't touched in a while. All three terms point the same way: bigger
score = needs more work. Tunable per `?weights=` later.

### Error codes

| HTTP | code | when |
|---|---|---|
| 401 | `unauthorized` | missing or invalid token |
| 403 | `scope_required` | token lacks required scope; details include needed scope |
| 404 | `not_found` | resource doesn't exist |
| 409 | `session_finished` | answering a finished/abandoned session |
| 409 | `idempotency_conflict` | same `Idempotency-Key` reused with different body |
| 410 | `exam_expired` | answering after `deadline_at` |
| 422 | `validation_failed` | body fails schema validation; `details.fields` lists errors |
| 422 | `invalid_question_payload` | payload doesn't match the declared `kind` |
| 422 | `exam_pool_insufficient` | dynamic blueprint can't be filled; `details` lists section / required / available |
| 503 | `scoring_unavailable` | grade required an unavailable dependency (e.g., LLM judge offline) |
| 500 | `internal` | unhandled; includes opaque `request_id` |

No silent fallbacks. If a free-text question is configured with
`judge: "llm"` and the LLM call fails, the answer returns 503 — never
"guess wrong."

## Frontend

Next.js 15+, App Router, TypeScript. Single deployment.

### Page map

| Path | Purpose |
|---|---|
| `/` | Dashboard: per-tag Elo chart, weakest-tags, "start quiz" / "start exam", recent activity |
| `/quiz/new` | Quiz config form (tags + difficulty + count) |
| `/quiz/[sessionId]` | Take a quiz: per-question feedback, deltas shown after each answer |
| `/exams` | Exam catalog: cards with last score + attempts count |
| `/exams/[examId]` | Exam detail: blueprint preview, history chart, "start" |
| `/exams/[examId]/session/[sessionId]` | Take an exam: progress + optional countdown, no per-answer feedback by default |
| `/exams/[examId]/result/[sessionId]` | Section scores, overall, level label, rating deltas, retake link |
| `/tags` | Tag landscape: list with rating + label + attempts |
| `/tags/[name]` | Tag deep dive: Elo trajectory chart, recent attempts, hardest-failed |
| `/me/tokens` | Manage own tokens (issue, revoke; copy plaintext once) |
| `/admin` | Admin overview: pending drafts count, recent agent activity, token activity |
| `/admin/questions` | Bank browser with filters and bulk operations |
| `/admin/questions/[id]` | Review/edit: live preview, version history, promote |
| `/admin/questions/new` | Manual create form (rare; agents do most authoring) |
| `/admin/tags` | Tag list, rename/merge |
| `/admin/exams` | Exam list + CRUD |
| `/admin/exams/[id]` | Blueprint editor: static/dynamic toggle, sections, weights, time limit, "validate pool" |
| `/admin/level-mappings` | Table editor: tag_pattern + label + elo range |
| `/admin/users` | User list (single user for now) |

### Frontend stack

| Concern | Choice |
|---|---|
| Components | shadcn/ui + Tailwind |
| API client | `openapi-typescript`-generated + thin fetch wrapper |
| Forms + validation | `react-hook-form` + `zod` (zod schemas mirror serde shapes; parity test enforces) |
| Markdown | `react-markdown` + `remark-gfm` + `rehype-sanitize` |
| Code highlighting | `shiki` (server-rendered) |
| Charts | `recharts` |
| Quiz/exam taking state | `zustand` |
| Auth on browser | HTTP-only `SameSite=Lax` cookie carrying the API token; Next.js route handlers (`app/api/*`) read the cookie and forward as `Authorization: Bearer …` to the Axum API. Browser JS never sees the token. |

### Taking screens — kind dispatch

```tsx
function QuestionView({ q, onAnswer }) {
  return (
    <>
      {q.code_snippet && <CodeBlock {...q.code_snippet} />}
      <Prompt markdown={q.prompt} />
      {q.kind === 'mcq'       && <MCQRenderer      q={q} onAnswer={onAnswer} />}
      {q.kind === 'free_text' && <FreeTextRenderer q={q} onAnswer={onAnswer} />}
      {q.kind === 'cloze'     && <ClozeRenderer    q={q} onAnswer={onAnswer} />}
    </>
  );
}
```

MCQ option order is **server-generated** when the session is planned (or
when `next_question` is built) and persisted in `sessions.question_plan`.
The renderer displays options in the order received from the API and posts
back a `selected_position` (index in the *shown* order). The client never
shuffles, never trusts client-supplied order, and never derives the order
from a seed. The server-stored `option_order` is copied into
`attempts.presentation` on each insert.

## Error handling — backend

```rust
// api/src/domain/error.rs (sketch)
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("not found: {resource}")]
    NotFound { resource: &'static str },                       // 404 not_found
    #[error("validation failed")]
    Validation(Vec<FieldError>),                               // 422 validation_failed
    #[error("invalid payload for kind {kind}: {reason}")]
    InvalidPayload { kind: String, reason: String },           // 422 invalid_question_payload
    #[error("exam pool insufficient")]
    ExamPoolInsufficient { section: String, required: usize, available: usize },  // 422
    #[error("session already finished")]
    SessionFinished,                                           // 409 session_finished
    #[error("exam expired")]
    ExamExpired,                                               // 410 exam_expired
    #[error("unauthorized")]
    Unauthorized,                                              // 401 unauthorized
    #[error("scope required: {0:?}")]
    ScopeRequired(&'static str),                               // 403 scope_required
    #[error("idempotency key conflict")]
    IdempotencyConflict,                                       // 409
    #[error("scoring unavailable")]
    ScoringUnavailable,                                        // 503
    #[error(transparent)]
    Internal(#[from] anyhow::Error),                           // 500 internal
}
```

Every error path is logged via `tracing` with structured fields. Internal
errors get a `request_id` returned to the client; full detail stays
server-side.

## Testing

```
Backend (Rust)
  unit tests        per-kind graders (mcq/free_text/cloze), Elo math,
                    blueprint resolution, version snapshot on edit,
                    calibration K-factor switch
  property tests    proptest: ratings bounded, sign-correct, energy-conserving
  integration       sqlx::test against ephemeral Postgres; end-to-end flows
                    (POST /questions → POST /quiz → POST /sessions/:id/answer);
                    exam-pool insufficiency returns the right code;
                    idempotency replay returns identical response;
                    scope-gating: each scope can/can't reach each endpoint
  contract          generated OpenAPI is diffed against api/openapi.yaml in CI

Frontend (TypeScript)
  component         vitest + testing-library on MCQ/FreeText/Cloze renderers,
                    shuffle determinism (same seed → same order)
  zod parity        roundtrip test for each kind's payload through zod
  e2e               Playwright golden paths:
                    – login → take a 3-question quiz → see deltas
                    – admin promotes a draft → it appears in the bank → user takes it
                    – dynamic exam start with insufficient pool surfaces correct UI

Make targets
  make fmt       cargo fmt + prettier + sqlfluff
  make lint      cargo clippy -D warnings + eslint --max-warnings 0
  make test      backend unit/integration + frontend vitest
  make e2e       Playwright (requires docker-compose up)
  make check     fmt + lint + test       (pre-commit hook)
  make validate  check + e2e             (pre-PR hook)
```

## Workspace layout

```
ame/                            (repo root)
  api/                          Axum backend
    src/{bank,engine,assess,stats,auth,http,domain}/
    openapi.yaml                generated, checked in
  web/                          Next.js app
    app/{(public),admin}/...
    lib/api/                    generated client + thin wrapper
  db/
    migrations/0001_init.sql    sqlx migrations
    docker-compose.yml          Postgres for dev + e2e
  agents/                       agent skill bundle
    generate-questions/SKILL.md
    analyze-performance/SKILL.md
    adaptive-generation/SKILL.md
  docs/
    specs/                      design docs (this file lives here)
    v0/                         working notes from brainstorming
    architecture.md, setup.md, plan.md, todo.md
  flake.nix                     nix devShell: rust toolchain, node, pg client, sqlfluff, prettier, …
  Makefile                      single task runner; see below
  CLAUDE.md, AGENTS.md          agent-facing rules and project briefing
  .mcp.json                     project-specific MCP (postgres URL via env)
```

The existing Dioxus workspace (`ui/`, `web/`, `desktop/`, `mobile/`, `api/`)
is replaced wholesale. Cargo.toml stops being a workspace root — `api/`
becomes a standalone crate.

## Defaults (numeric values used unless overridden)

| Setting | Default | Where to change |
|---|---|---|
| New user_tag_rating | 1200 | column default |
| New question rating (no `difficulty_initial`) | 1400 | column default |
| Calibration phase length | 20 attempts | `engine/elo.rs` constant |
| K-factor (calibration) | user 0, question 48 | `engine/elo.rs` |
| K-factor (early, < 30 user attempts on tag) | user 32, question 16 | `engine/elo.rs` |
| K-factor (mature) | user 16, question 16 | `engine/elo.rs` |
| Quiz default question count | 10 | `bank/defaults` |
| Quiz `exclude_recent_hours` default | 24 | request default |
| Idempotency-Key TTL | 24h | scheduled cleanup job |
| Stale quiz session auto-abandon | 24h with no activity | scheduled cleanup job |
| Batch size cap on `POST /questions` | 50 per call | handler-side check |
| Max `count` per quiz/exam section | 100 | handler-side check |

## Open Questions (not blocking implementation; flagged for revisit)

- **Quiz tag selection — `any` vs `all`.** Default `any` is least surprising,
  but `all` might be more useful for narrow practice. Keep configurable per
  request; default `any`.
- **Sampling distribution for dynamic exams.** Uniform random within the
  filter, or weighted toward questions the user hasn't seen / hasn't seen
  in long enough? MVP: uniform random; revisit once usage data exists.
- **Free-text grading "normalize" options beyond
  `case_insensitive_strip_accents` and `exact`.** Add Unicode NFKC
  normalization variant? Defer until a Spanish bank exposes a gap.
- **Tag rename/merge in admin** — design says yes, schema-wise it's
  straightforward (UPDATE question_tags), but UX merits a follow-up doc.

## References

See `docs/v0/research-notes.md` for the full list of prior-art sources
consulted during the brainstorming session.
