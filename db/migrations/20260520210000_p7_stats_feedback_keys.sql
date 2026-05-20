-- Plan 7: stats, messages, keys/webhooks, minimal quizzes table

-- Quizzes table (minimal; full CRUD + quiz_questions join comes in Plan 4)
create table quizzes (
  id          uuid primary key default uuid_generate_v7(),
  title       text not null,
  status      text not null default 'draft',
  created_by  uuid not null references users(id),
  created_at  timestamptz not null default now(),
  updated_at  timestamptz not null default now(),
  constraint quizzes_status_check check (status in ('draft', 'active', 'archived'))
);

-- FK that Plan 5 left open (quiz_id was added without a FK)
alter table sessions
  add constraint sessions_quiz_fk foreign key (quiz_id) references quizzes(id);

-- Messages (instructor → learner feedback; email channel reserved for future transport)
create table messages (
  id            uuid primary key default uuid_generate_v7(),
  from_user_id  uuid not null references users(id),
  to_user_id    uuid not null references users(id),
  channel       text not null,
  body          text not null,
  link_quiz_id  uuid references quizzes(id),
  status        text not null default 'queued',
  created_at    timestamptz not null default now(),
  constraint messages_channel_check check (channel in ('in_app', 'email')),
  constraint messages_status_check  check (status  in ('queued', 'delivered', 'read'))
);
create index messages_to_user on messages(to_user_id, created_at desc) where status != 'read';

-- Webhooks (secret shown once at creation, stored hashed)
create table webhooks (
  id           uuid primary key default uuid_generate_v7(),
  user_id      uuid not null references users(id) on delete cascade,
  url          text not null,
  events       text[] not null default '{}',
  secret_hash  text not null,
  created_at   timestamptz not null default now(),
  revoked_at   timestamptz
);
create index webhooks_user_active on webhooks(user_id) where revoked_at is null;

-- Quiz item stats rollup (populated by Plan 8 webhook trigger; live-computed fallback in P7)
create table quiz_item_stats (
  quiz_id            uuid not null references quizzes(id) on delete cascade,
  question_id        uuid not null references questions(id) on delete cascade,
  correct_rate       double precision not null default 0,
  avg_time_ms        double precision,
  discrimination_idx double precision,
  last_computed      timestamptz not null default now(),
  primary key (quiz_id, question_id)
);

-- Expand scopes CHECK to include Plan-7+ vocabulary
alter table api_tokens drop constraint api_tokens_scopes_check;
alter table api_tokens
  add constraint api_tokens_scopes_check
  check (
    scopes <@ array[
      'human', 'agent:write-questions', 'agent:read-only',
      'quiz.read', 'quiz.write',
      'attempt.read', 'attempt.write',
      'stats.read', 'feedback.write',
      'plan.read', 'plan.write',
      'admin'
    ]::text[]
    and array_length(scopes, 1) >= 1
  );
