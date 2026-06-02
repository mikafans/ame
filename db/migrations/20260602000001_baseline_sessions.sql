-- ─── tb_sessions ────────────────────────────────────────────────────────────

CREATE TABLE tb_sessions (
  id              uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id         uuid        NOT NULL REFERENCES tb_users(id)   ON DELETE CASCADE,
  assessment_id   uuid        REFERENCES tb_assessments(id) ON DELETE CASCADE,
  kind            text        NOT NULL DEFAULT 'assessment',
  filter          jsonb,
  question_plan   jsonb       NOT NULL,
  status          text        NOT NULL DEFAULT 'in_progress',
  affects_rating  boolean     NOT NULL DEFAULT TRUE,
  rating_snapshot jsonb       NOT NULL DEFAULT '{}'::jsonb,
  result          jsonb,
  deadline_at     timestamptz,
  started_at      timestamptz NOT NULL DEFAULT now(),
  finished_at     timestamptz,
  CONSTRAINT tb_sessions_status_check CHECK (status IN ('in_progress', 'finished', 'abandoned'))
);
CREATE INDEX tb_sessions_user_started ON tb_sessions(user_id, started_at DESC);
CREATE INDEX idx_sessions_user_assessment_status ON tb_sessions (user_id, assessment_id, status);
CREATE INDEX idx_sessions_assessment_finished ON tb_sessions (assessment_id, finished_at DESC) WHERE status = 'finished';
CREATE INDEX idx_sessions_user_status_started ON tb_sessions (user_id, status, started_at DESC);

-- ─── tb_attempts ────────────────────────────────────────────────────────────

CREATE TABLE tb_attempts (
  id                     uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id                uuid             NOT NULL REFERENCES tb_users(id)      ON DELETE CASCADE,
  question_id            uuid             NOT NULL REFERENCES tb_questions(id)  ON DELETE RESTRICT,
  question_version       integer          NOT NULL,
  session_id             uuid             REFERENCES tb_sessions(id)            ON DELETE SET NULL,
  response               jsonb            NOT NULL,
  presentation           jsonb            NOT NULL DEFAULT '{}'::jsonb,
  correct_answer         jsonb,
  is_correct             boolean          NOT NULL,
  score                  double precision NOT NULL,
  time_to_answer_ms      integer,
  rating_before_user_avg double precision NOT NULL,
  rating_before_question double precision NOT NULL,
  user_tag_deltas        jsonb            NOT NULL,
  question_delta         double precision NOT NULL,
  grade_status           text             NOT NULL DEFAULT 'graded' CHECK (grade_status IN ('graded', 'pending_manual')),
  grader_notes           text,
  created_at             timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX tb_attempts_user_created ON tb_attempts(user_id, created_at DESC);
CREATE INDEX tb_attempts_question     ON tb_attempts(question_id, created_at DESC);
CREATE INDEX tb_attempts_session      ON tb_attempts(session_id) WHERE session_id IS NOT NULL;
CREATE UNIQUE INDEX tb_attempts_session_question_unique
  ON tb_attempts(session_id, question_id) WHERE session_id IS NOT NULL;

-- ─── tb_level_mappings ──────────────────────────────────────────────────────

CREATE TABLE tb_level_mappings (
  id          uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  tag_pattern text             NOT NULL,
  label       text             NOT NULL,
  elo_min     double precision NOT NULL,
  elo_max     double precision NOT NULL,
  sort_order  integer          NOT NULL DEFAULT 0,
  created_at  timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX tb_level_mappings_pattern ON tb_level_mappings(tag_pattern);

-- ─── tb_idempotency_keys ────────────────────────────────────────────────────

CREATE TABLE tb_idempotency_keys (
  token_id        uuid     NOT NULL REFERENCES tb_api_tokens(id) ON DELETE CASCADE,
  key             text     NOT NULL,
  request_hash    text     NOT NULL,
  response_status smallint NOT NULL,
  response_body   jsonb    NOT NULL,
  created_at      timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (token_id, key)
);
CREATE INDEX tb_idempotency_keys_created ON tb_idempotency_keys(created_at);

-- ─── tb_messages ────────────────────────────────────────────────────────────

CREATE TABLE tb_messages (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  from_user_id uuid        NOT NULL REFERENCES tb_users(id),
  to_user_id   uuid        NOT NULL REFERENCES tb_users(id),
  channel      text        NOT NULL,
  body         text        NOT NULL,
  status       text        NOT NULL DEFAULT 'queued',
  created_at   timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_messages_channel_check CHECK (channel IN ('in_app', 'email')),
  CONSTRAINT tb_messages_status_check  CHECK (status  IN ('queued', 'delivered', 'read'))
);
CREATE INDEX tb_messages_to_user ON tb_messages(to_user_id, created_at DESC) WHERE status != 'read';

-- ─── tb_webhooks ───────────────────────────────────────────────────────────

CREATE TABLE tb_webhooks (
  id          uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id     uuid        NOT NULL REFERENCES tb_users(id) ON DELETE CASCADE,
  url         text        NOT NULL,
  events      text[]      NOT NULL DEFAULT '{}',
  secret_hash text        NOT NULL,
  signing_key text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  revoked_at  timestamptz
);
CREATE INDEX tb_webhooks_user_active ON tb_webhooks(user_id) WHERE revoked_at IS NULL;

-- ─── tb_webhook_deliveries ───────────────────────────────────────────────────

CREATE TABLE tb_webhook_deliveries (
  id              uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  webhook_id      uuid        NOT NULL REFERENCES tb_webhooks(id) ON DELETE CASCADE,
  event           text        NOT NULL,
  payload         jsonb       NOT NULL,
  attempt_num     integer     NOT NULL DEFAULT 0,
  status          text        NOT NULL DEFAULT 'pending',
  last_error      text,
  next_attempt_at timestamptz,
  created_at      timestamptz NOT NULL DEFAULT now(),
  completed_at    timestamptz,
  CONSTRAINT tb_webhook_deliveries_status_check CHECK (status IN ('pending', 'delivered', 'failed'))
);
CREATE INDEX tb_webhook_deliveries_pending ON tb_webhook_deliveries(next_attempt_at) WHERE status = 'pending';

-- ─── tb_study_plans ─────────────────────────────────────────────────────────

CREATE TABLE tb_study_plans (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id      uuid        NOT NULL REFERENCES tb_users(id),
  goal         text        NOT NULL,
  lookback_days bigint     NOT NULL,
  generated_at timestamptz NOT NULL DEFAULT now(),
  weeks        jsonb       NOT NULL DEFAULT '[]'
);

-- ─── tb_activity_log ────────────────────────────────────────────────────────

CREATE TABLE tb_activity_log (
  id        uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  ts        timestamptz NOT NULL DEFAULT now(),
  agent_id  uuid        NOT NULL REFERENCES tb_users(id),
  tool_name text        NOT NULL,
  method    text        NOT NULL,
  path      text        NOT NULL,
  status    integer     NOT NULL,
  note      text,
  target_id uuid
);
CREATE INDEX tb_activity_log_agent ON tb_activity_log(agent_id, ts DESC);

-- ─── tb_cohorts ─────────────────────────────────────────────────────────────

CREATE TABLE tb_cohorts (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  name        text NOT NULL,
  description text,
  created_at  timestamptz NOT NULL DEFAULT now()
);

-- ─── tb_cohort_memberships ───────────────────────────────────────────────────

CREATE TABLE tb_cohort_memberships (
  id         uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  cohort_id  uuid NOT NULL REFERENCES tb_cohorts(id) ON DELETE CASCADE,
  user_id    uuid NOT NULL REFERENCES tb_users(id) ON DELETE CASCADE,
  joined_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE(cohort_id, user_id)
);

CREATE INDEX tb_cohort_memberships_cohort ON tb_cohort_memberships(cohort_id);
CREATE INDEX tb_cohort_memberships_user   ON tb_cohort_memberships(user_id);

-- ─── tb_agent_profiles ──────────────────────────────────────────────────────

CREATE TABLE tb_agent_profiles (
  agent_user_id uuid PRIMARY KEY REFERENCES tb_users(id) ON DELETE CASCADE,
  label text NOT NULL,
  focus_tags text[] NOT NULL DEFAULT '{}',
  current_goal text,
  next_target text,
  memory jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tr_agent_profiles_updated_at
  BEFORE UPDATE ON tb_agent_profiles
  FOR EACH ROW
  EXECUTE FUNCTION fn_update_updated_at();

-- ─── tb_audit_log ───────────────────────────────────────────────────────────

CREATE TABLE tb_audit_log (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  actor_user_id  uuid REFERENCES tb_users(id) ON DELETE SET NULL,
  action         text NOT NULL,
  target_type    text,
  target_id      uuid,
  metadata       jsonb NOT NULL DEFAULT '{}',
  created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_created ON tb_audit_log (created_at DESC);

-- Grant schema-level permissions to ame_app
GRANT USAGE ON SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO ame_app;

-- Ensure future tables/sequences/functions created by postgres are automatically accessible by ame_app
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON TABLES TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON SEQUENCES TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON FUNCTIONS TO ame_app;
