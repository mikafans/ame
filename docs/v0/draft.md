# Design Draft — Working Notes

Raw collated draft from the 2026-05-19 brainstorming session.
The clean spec lives at `docs/specs/2026-05-19-question-exam-platform-design.md`.
This file is the un-edited working material in case anything got dropped.

---

## 1. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  NAS (Tailscale)                                                │
│                                                                 │
│  Next.js ──┐                                                    │
│  (web,/admin)                                                   │
│            ▼                                                    │
│         Axum API ◀────── external agents (claude-code, scripts) │
│            │             ▲  read /me/stats, /weakest-tags       │
│            ▼             │  write /questions                    │
│         Postgres         │  scoped tokens (no /attempts write)  │
│                          │                                      │
│                       agents/SKILL.md bundle + openapi.yaml     │
│                       (in repo, drives agent behavior)          │
└─────────────────────────────────────────────────────────────────┘
```

**Components:**

- `api/` (Rust, Axum) — REST API, single binary, sqlx to Postgres.
- `web/` (Next.js + TypeScript) — public `/*` for taking; `/admin/*` role-gated.
- `db/` — Postgres in Docker on NAS, sqlx-migrated.
- `agents/` — skill bundle (markdown skills + OpenAPI reference). Drives external agent behavior.
- Auth — Bearer tokens with scopes: `human`, `agent:write-questions`, `agent:read-only`. Agent tokens cannot POST attempts (no Elo inflation).

**Agent loop:** `GET /me/weakest-tags → GET /tags/:name/stats → generate questions targeting the gap → POST /questions as drafts → user reviews → user takes quiz/exam → Elo updates → agent fetches again`.

---

## 2. Data Model (v2.1 + research-driven additions)

### Core tables

```sql
create extension if not exists pgcrypto;  -- pg18 has uuidv7() built-in

create table users (
  id            uuid primary key default uuidv7(),
  email         text,
  display_name  text not null,
  role          text not null,                        -- 'admin' | 'user' (app-enforced)
  created_at    timestamptz not null default now()
);

create table api_tokens (
  id            uuid primary key default uuidv7(),
  user_id       uuid not null references users(id) on delete cascade,
  name          text not null,
  token_hash    text not null,                        -- argon2id
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

create table tags (
  id            uuid primary key default uuidv7(),
  name          text not null unique,                 -- stored lowercased
  description   text,
  created_at    timestamptz not null default now(),
  constraint tags_name_lowercase check (name = lower(name))
);
```

### Question bank (with versioning)

```sql
create table questions (
  id              uuid primary key default uuidv7(),
  kind            text not null,
  prompt          text not null,
  code_snippet    jsonb,                              -- { language, body, caption? }
  payload         jsonb not null,                     -- shape varies by kind
  explanation     text,
  status          text not null default 'draft',
  source          text,
  rating          double precision not null default 1400,
  attempts_count  integer not null default 0,
  version         integer not null default 1,
  created_by      uuid not null references users(id),
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now(),
  constraint questions_kind_check check (kind in ('mcq','free_text','cloze')),
  constraint questions_status_check check (status in ('draft','live','archived'))
);
create index questions_live_created on questions(created_at desc) where status = 'live';
create index questions_status on questions(status);

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

### Ratings, sessions, attempts

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

create table exams (
  id            uuid primary key default uuidv7(),
  name          text not null,
  description   text,
  blueprint     jsonb not null,
  time_limit_seconds  integer,
  show_results_during boolean not null default false,
  affects_rating      boolean not null default true,
  status        text not null default 'draft',
  created_by    uuid not null references users(id),
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);

create table sessions (                              -- was quiz_sessions
  id              uuid primary key default uuidv7(),
  user_id         uuid not null references users(id) on delete cascade,
  kind            text not null default 'quiz',      -- 'quiz' | 'exam'
  exam_id         uuid references exams(id),
  filter          jsonb,                             -- quiz: tag selectors, difficulty range
  question_plan   jsonb,                             -- exam: ordered ids + section assignment
  status          text not null default 'in_progress',
  result          jsonb,                             -- exam: set on finish
  deadline_at     timestamptz,
  started_at      timestamptz not null default now(),
  finished_at     timestamptz,
  constraint sessions_exam_link
    check ((kind = 'exam' and exam_id is not null) or (kind = 'quiz' and exam_id is null))
);
create index sessions_user_started on sessions(user_id, started_at desc);

create table attempts (
  id                       uuid primary key default uuidv7(),
  user_id                  uuid not null references users(id) on delete cascade,
  question_id              uuid not null references questions(id) on delete restrict,
  question_version         integer not null,
  session_id               uuid references sessions(id) on delete set null,
  response                 jsonb not null,
  presentation             jsonb not null default '{}'::jsonb,  -- e.g. {"option_order":[2,0,3,1]}
  is_correct               boolean not null,
  score                    double precision not null,           -- 0..1
  time_to_answer_ms        integer,
  rating_before_user_avg   double precision not null,
  rating_before_question   double precision not null,
  user_tag_deltas          jsonb not null,                      -- { "tag": delta, ... }
  question_delta           double precision not null,
  created_at               timestamptz not null default now()
);
create index attempts_user_created on attempts(user_id, created_at desc);
create index attempts_question     on attempts(question_id, created_at desc);
create index attempts_session      on attempts(session_id) where session_id is not null;

create table level_mappings (
  id            uuid primary key default uuidv7(),
  tag_pattern   text not null,                       -- exact tag or 'spanish:*'
  label         text not null,
  elo_min       double precision not null,
  elo_max       double precision not null,
  sort_order    integer not null default 0,
  created_at    timestamptz not null default now()
);
create index level_mappings_pattern on level_mappings(tag_pattern);
```

### Elo update (v3 with calibration)

```
On answer to question Q with tags [T1..Tn], score in [0,1]:
  user_avg     = mean(user_tag_ratings[Tᵢ], default=1200)
  expected     = 1 / (1 + 10^((Q.rating - user_avg) / 400))

  if Q.attempts_count < 20:                          # calibration phase
    K_user     = 0                                    # user rating frozen
    K_question = 48                                   # question moves fast
  else:
    K_user     = 32 if min(user_tag_attempts) < 30 else 16
    K_question = 16

  delta_user_total = K_user     * (score - expected)
  delta_question   = K_question * (expected - score)

  for Tᵢ in tags:
    user_tag_ratings[Tᵢ].rating += delta_user_total / n
  Q.rating += delta_question
  Q.attempts_count += 1
```

### Exam blueprint shapes

```jsonc
// Static
{
  "type": "static",
  "sections": [
    { "name": "Ownership", "question_ids": ["..."], "weight": 1.0 }
  ]
}

// Dynamic
{
  "type": "dynamic",
  "sections": [
    {
      "name": "Async",
      "tags": ["rust:async"],
      "tags_mode": "any",
      "count": 5,
      "difficulty_min": 1300,
      "difficulty_max": 1700,
      "weight": 1.5
    }
  ],
  "exclude_recent_hours": 168
}
```

### Result shape

```jsonc
{
  "score_overall": 0.78,
  "sections": [
    { "name": "Ownership", "score": 0.85, "correct": 4, "total": 5, "weight": 1.0 }
  ],
  "duration_seconds": 1240,
  "rating_changes": [{ "tag": "rust:ownership", "before": 1450, "after": 1481 }],
  "level_label": "intermediate"
}
```

---

## 3. API Surface

All endpoints: JSON, `snake_case`, Bearer-token auth, scope-gated. Errors:
`{ error: { code, message, details? } }`. Cursor pagination on lists. Optional
`Idempotency-Key` header on POSTs (TTL 24h). OpenAPI spec at `/openapi.json`,
checked into repo as `api/openapi.yaml`.

### Scope matrix

| Endpoints | `human` | `agent:write-questions` | `agent:read-only` |
|---|---|---|---|
| `/me`, `/me/stats`, `/me/weakest-tags`, `/me/recent-attempts`, `/tags/*/stats` | ✓ | ✓ | ✓ |
| `GET /questions`, `GET /tags`, `GET /exams`, `GET /levels` | ✓ | ✓ | ✓ |
| `POST /questions`, `PATCH /questions/:id`, `POST /questions/:id/promote` | ✓ | ✓ | ✗ |
| `POST /quiz`, `POST /exams/:id/start`, `POST /sessions/:id/answer`, `/sessions/:id/finish` | ✓ | ✗ | ✗ |
| `/admin/**` | admin role | ✗ | ✗ |

### Endpoints

```
# identity
GET    /me
POST   /me/tokens                       # own tokens
GET    /me/tokens
DELETE /me/tokens/:id

# tags
GET    /tags?q=&limit=
POST   /tags

# questions
GET    /questions?tag=&status=&difficulty_min=&difficulty_max=&q=&cursor=&limit=
GET    /questions/:id
POST   /questions                       # batch, max 50 per call
PATCH  /questions/:id                   # editing live questions auto-versions
POST   /questions/:id/promote
POST   /questions/:id/archive

# quiz (convenience wrapper that creates a session with kind='quiz')
POST   /quiz                            # body: tags, difficulty range, count

# exams
GET    /exams
GET    /exams/:id
POST   /admin/exams
PATCH  /admin/exams/:id
POST   /admin/exams/:id/publish
POST   /admin/exams/:id/archive
POST   /exams/:id/start                 # creates session, kind='exam'

# sessions (unified)
GET    /sessions/:id
POST   /sessions/:id/answer
POST   /sessions/:id/finish

# stats
GET    /me/stats
GET    /me/weakest-tags?limit=
GET    /me/recent-attempts?limit=
GET    /tags/:name/stats
GET    /me/exam-attempts?exam_id=
GET    /levels

# admin
GET/POST/PATCH/DELETE /admin/level-mappings
GET    /admin/users
```

### Key request/response shapes

`POST /questions` (agent ingestion contract):

```jsonc
{
  "questions": [
    {
      "kind": "mcq",
      "prompt": "...",
      "code_snippet": { "language": "rust", "body": "..." },
      "payload": { "options": ["..."], "correct_index": 2 },
      "explanation": "...",
      "tags": ["rust", "rust:concurrency"],
      "difficulty_initial": 1500,
      "source": "agent: weakest-tag loop 2026-05-19",
      "status": "draft"
    }
  ]
}

// → { "created": 1, "questions": [...], "errors": [] }
```

`POST /quiz`:

```jsonc
{
  "tags": ["rust"],
  "tags_mode": "any",
  "difficulty_min": 1200,
  "difficulty_max": 1800,
  "count": 10,
  "exclude_recent_hours": 24
}

// → { "session_id": "...", "remaining": 10, "question": { ...payload sans correct_index... } }
```

`POST /sessions/:id/answer`:

```jsonc
// request
{ "question_id": "...", "response": { "selected_position": 0 }, "time_to_answer_ms": 4500 }
// presentation echoed back to backend? No — server stored it. Client sends position into shown order.

// response
{
  "is_correct": true,
  "score": 1.0,
  "correct_answer": { "selected_index": 2 },
  "explanation": "...",
  "user_tag_deltas": { "rust": 8.4 },
  "question_delta": -8.4,
  "level_changes": [{ "tag": "rust", "old_label": "novice", "new_label": "intermediate" }],
  "next_question": { ... } | null,
  "session_status": "in_progress" | "finished"
}
```

`GET /me/weakest-tags?limit=5` (agent loop entry point):

```jsonc
{
  "tags": [{
    "tag": "rust:async",
    "rating": 1320,
    "attempts": 24,
    "accuracy_recent": 0.42,
    "last_attempt_at": "2026-05-18T12:00:00Z",
    "current_label": "novice",
    "needs_work_score": 0.78
  }]
}
```

---

## 4. Frontend (Next.js)

App Router, TypeScript. One deployment, public `/*` + admin `/admin/*` gated by role.

**Page map** — see spec for full list.

**Stack:**
- Components: `shadcn/ui` + Tailwind
- API client: `openapi-typescript` from `api/openapi.yaml`
- Forms: `react-hook-form` + `zod` (zod schemas mirror serde, parity test)
- Markdown: `react-markdown` + `remark-gfm` + `rehype-sanitize`
- Code: `shiki`
- Charts: `recharts`
- Session state: `zustand`
- Auth: HTTP-only cookie carrying token, `SameSite=Lax`

**MCQ shuffle**: client shuffles using seed `(session_id, question_id)` → stable on refresh; `option_order` stored in `attempts.presentation` at submit.

---

## 5. Error handling

Backend typed `ApiError` enum mapped to HTTP + structured body:

- `not_found` (404), `validation_failed` (422), `invalid_question_payload` (422),
  `exam_pool_insufficient` (422 with `{ section, required, available }`),
  `session_finished` (409), `exam_expired` (410), `unauthorized` (401),
  `scope_required` (403), `idempotency_conflict` (409),
  `scoring_unavailable` (503), `internal` (500 with opaque `request_id`).

Every error logged via `tracing`. Frontend: known codes get specific handling
(toast or inline); unknown codes → generic toast with request_id.

No silent fallbacks: LLM-judge failure returns 503 rather than guessing.

---

## 6. Testing

Backend: unit (grading, Elo math, blueprint scoring, version snapshot,
calibration switch), proptest on Elo invariants, integration tests against
ephemeral Postgres via `sqlx::test`, OpenAPI spec diff on every CI run.

Frontend: vitest + testing-library for renderers + shuffle determinism,
zod-vs-serde parity test, Playwright e2e for golden paths.

Make targets: `fmt`, `lint`, `test`, `e2e`, `check` (pre-commit), `validate` (pre-PR).

---

## 7. Non-goals (out of MVP)

FSRS scheduler, Glicko-2, LLM-judge for free-text, magic-link/OAuth, xAPI
emission, QTI import/export, per-question timers, hints field, audit log
enforcement, mobile app. Each is schema-compatible for later.

---

## 8. Workspace layout

```
ame/
  api/                      Rust workspace member (single crate)
    src/{bank,engine,assess,stats,auth,http,domain}/
    openapi.yaml            generated, checked in
  web/                      Next.js app
    app/{(public),admin}/
    lib/api/                generated client + thin wrapper
  db/
    migrations/0001_init.sql
    docker-compose.yml      postgres for dev + e2e
  agents/                   skill bundle
    generate-questions/SKILL.md
    analyze-performance/SKILL.md
    adaptive-generation/SKILL.md
  docs/
    specs/                      design docs live here
    v0/                         working notes
    architecture.md, setup.md, plan.md, todo.md
  flake.nix
  Makefile
  CLAUDE.md, AGENTS.md
  .mcp.json                 project-specific MCP only (postgres URL from env)
```

Old Dioxus dirs (`ui/`, `desktop/`, `mobile/`) — deleted in the same commit
that introduces the new structure.
