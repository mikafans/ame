CREATE OR REPLACE FUNCTION uuid_generate_v7()
RETURNS uuid AS $$
DECLARE
  unix_ts_ms bytea;
  uuid_bytes bytea;
BEGIN
  unix_ts_ms := substring(int8send(floor(extract(epoch from clock_timestamp()) * 1000)::bigint) from 3);
  uuid_bytes := uuid_send(gen_random_uuid());
  uuid_bytes := overlay(uuid_bytes placing unix_ts_ms from 1 for 6);
  uuid_bytes := set_byte(uuid_bytes, 6, (get_byte(uuid_bytes, 6) & 15) | 112);
  uuid_bytes := set_byte(uuid_bytes, 8, (get_byte(uuid_bytes, 8) & 63) | 128);
  RETURN encode(uuid_bytes, 'hex')::uuid;
END;
$$ LANGUAGE plpgsql VOLATILE;

create table users (
  id            uuid primary key default uuid_generate_v7(),
  email         text,                                       -- nullable until magic-link auth lands
  display_name  text not null,
  role          text not null,                              -- 'admin' | 'user' (app-enforced)
  created_at    timestamptz not null default now()
);

create table api_tokens (
  id            uuid primary key default uuid_generate_v7(),
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

create table tags (
  id            uuid primary key default uuid_generate_v7(),
  name          text not null unique,                       -- stored lowercased
  description   text,
  created_at    timestamptz not null default now(),
  constraint tags_name_lowercase check (name = lower(name))
);

create table questions (
  id              uuid primary key default uuid_generate_v7(),
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
  id            uuid primary key default uuid_generate_v7(),
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
  id              uuid primary key default uuid_generate_v7(),
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

create table attempts (
  id                       uuid primary key default uuid_generate_v7(),
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

create table level_mappings (
  id            uuid primary key default uuid_generate_v7(),
  tag_pattern   text not null,                              -- exact tag or 'spanish:*'
  label         text not null,
  elo_min       double precision not null,
  elo_max       double precision not null,
  sort_order    integer not null default 0,
  created_at    timestamptz not null default now()
);
create index level_mappings_pattern on level_mappings(tag_pattern);

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