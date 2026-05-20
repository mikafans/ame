-- Plan 8: agent surface — study_plans, activity_log, share_links, anonymous_attempts,
--         webhook_deliveries, signing_key on webhooks, agent role support

-- Add signing_key to webhooks so the dispatcher can compute HMAC signatures.
-- The existing secret_hash (argon2) is kept for future verification; signing_key
-- is the raw secret stored for outbound HMAC only.
alter table webhooks add column signing_key text;

-- Study plans (persisted output of the planner)
create table study_plans (
  id            uuid primary key default uuid_generate_v7(),
  user_id       uuid not null references users(id),
  goal          text not null,
  lookback_days bigint not null,
  generated_at  timestamptz not null default now(),
  weeks         jsonb not null default '[]'
);
create index study_plans_user on study_plans(user_id, generated_at desc);

-- Activity log: append-only audit record of agent-driven API calls
create table activity_log (
  id          uuid primary key default uuid_generate_v7(),
  ts          timestamptz not null default now(),
  agent_id    uuid not null references users(id),
  tool_name   text not null,
  method      text not null,
  path        text not null,
  status      int not null,
  note        text,
  target_id   uuid
);
create index activity_log_agent on activity_log(agent_id, ts desc);

-- Share links
create table share_links (
  id                   uuid primary key default uuid_generate_v7(),
  kind                 text not null,
  target_id            uuid not null,
  created_by_user_id   uuid not null references users(id),
  visibility           text not null default 'public',
  include_explanation  boolean not null default false,
  include_score        boolean not null default false,
  include_attribution  boolean not null default true,
  og_image_url         text,
  created_at           timestamptz not null default now(),
  revoked_at           timestamptz,
  constraint share_links_kind_check check (kind in ('quiz', 'exam', 'item')),
  constraint share_links_visibility_check check (visibility in ('public', 'cohort'))
);
create index share_links_target on share_links(target_id, kind);
create index share_links_creator on share_links(created_by_user_id, created_at desc);

-- Anonymous attempts (written by ?interactive=1 embed routes; never attributed to a user)
create table anonymous_attempts (
  id           uuid primary key default uuid_generate_v7(),
  share_id     uuid not null references share_links(id) on delete cascade,
  question_id  uuid not null references questions(id),
  ts           timestamptz not null default now(),
  ip_hash      text not null,
  response     jsonb not null,
  is_correct   boolean
);
create index anonymous_attempts_share on anonymous_attempts(share_id, ts desc);

-- Webhook deliveries: persisted retry state for outbound webhook dispatch
create table webhook_deliveries (
  id              uuid primary key default uuid_generate_v7(),
  webhook_id      uuid not null references webhooks(id) on delete cascade,
  event           text not null,
  payload         jsonb not null,
  attempt_num     int not null default 0,
  status          text not null default 'pending',
  last_error      text,
  next_attempt_at timestamptz,
  created_at      timestamptz not null default now(),
  completed_at    timestamptz,
  constraint webhook_deliveries_status_check check (status in ('pending', 'delivered', 'failed'))
);
create index webhook_deliveries_pending on webhook_deliveries(next_attempt_at)
  where status = 'pending';
