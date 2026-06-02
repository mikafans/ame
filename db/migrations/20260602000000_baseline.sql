-- Squashed baseline: consolidates all migrations up to 2026-06-01.
-- Safe to apply on a fresh DB only. Run via: make db-reset

DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ame_app') THEN
    CREATE ROLE ame_app WITH LOGIN PASSWORD 'postgres';
  END IF;
END
$$;

-- ─── helpers ────────────────────────────────────────────────────────────────

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

CREATE OR REPLACE FUNCTION fn_update_updated_at()
RETURNS trigger AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ─── tb_users ───────────────────────────────────────────────────────────────

CREATE TABLE tb_users (
  id             uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  owner_user_id  uuid        REFERENCES tb_users(id),
  email          text        UNIQUE,
  password_hash  text,
  display_name   text        NOT NULL,
  role           text        NOT NULL,
  plan           text        NOT NULL DEFAULT 'free',
  deactivated_at timestamptz,
  created_at     timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_users_role_check CHECK (role IN ('user', 'admin', 'agent')),
  CONSTRAINT tb_users_plan_check CHECK (plan IN ('free', 'premium'))
);

-- ─── tb_api_tokens ──────────────────────────────────────────────────────────

CREATE TABLE tb_api_tokens (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id      uuid        NOT NULL REFERENCES tb_users(id) ON DELETE CASCADE,
  name         text        NOT NULL,
  token_hash   text        NOT NULL,
  scopes       text[]      NOT NULL,
  last_used_at timestamptz,
  revoked_at   timestamptz,
  expires_at   timestamptz NOT NULL DEFAULT now() + INTERVAL '30 days',
  created_at   timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_api_tokens_scopes_check
    CHECK (
      scopes <@ ARRAY[
        'assessment.read', 'assessment.write', 'attempt.read', 'attempt.write',
        'stats.read', 'feedback.write', 'plan.read', 'plan.write',
        'admin'
      ]::text[]
      AND array_length(scopes, 1) >= 1
    )
);
CREATE INDEX tb_api_tokens_user_active ON tb_api_tokens(user_id) WHERE revoked_at IS NULL;

-- ─── tb_tags ────────────────────────────────────────────────────────────────

CREATE TABLE tb_tags (
  id          uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  name        text        NOT NULL UNIQUE,
  description text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_tags_name_lowercase CHECK (name = lower(name))
);

-- ─── tb_questions ───────────────────────────────────────────────────────────

CREATE TABLE tb_questions (
  id             uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  owner_id       uuid             NOT NULL REFERENCES tb_users(id),
  kind           text             NOT NULL,
  prompt         text             NOT NULL,
  code_snippet   jsonb,
  payload        jsonb            NOT NULL,
  explanation    text,
  status         text             NOT NULL DEFAULT 'draft',
  source         text,
  rating         double precision NOT NULL DEFAULT 1400,
  attempts_count integer          NOT NULL DEFAULT 0,
  version        integer          NOT NULL DEFAULT 1,
  points         integer          NOT NULL DEFAULT 1,
  created_by     uuid             NOT NULL REFERENCES tb_users(id),
  created_at     timestamptz      NOT NULL DEFAULT now(),
  updated_at     timestamptz      NOT NULL DEFAULT now(),
  prompt_tsv     tsvector         GENERATED ALWAYS AS (to_tsvector('english', coalesce(prompt, ''))) STORED,
  CONSTRAINT tb_questions_kind_check   CHECK (kind   IN ('mc', 'tf', 'short', 'essay', 'code')),
  CONSTRAINT tb_questions_status_check CHECK (status IN ('draft', 'live', 'archived')),
  CONSTRAINT tb_questions_points_nonnegative CHECK (points >= 0)
);
CREATE INDEX tb_questions_live_created ON tb_questions(created_at DESC) WHERE status = 'live';
CREATE INDEX tb_questions_status        ON tb_questions(status);
CREATE INDEX idx_tb_questions_prompt_tsv ON tb_questions USING gin (prompt_tsv);
CREATE INDEX idx_tb_questions_owner_id ON tb_questions(owner_id);

ALTER TABLE tb_questions ENABLE ROW LEVEL SECURITY;
ALTER TABLE tb_questions FORCE ROW LEVEL SECURITY;

CREATE POLICY tb_questions_owner_policy ON tb_questions
  FOR ALL
  USING (
    owner_id = nullif(current_setting('app.owner', true), '')::uuid 
    OR current_setting('app.is_admin', true) = 'true'
  )
  WITH CHECK (
    owner_id = nullif(current_setting('app.owner', true), '')::uuid 
    OR current_setting('app.is_admin', true) = 'true'
  );

CREATE TABLE tb_question_versions (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  question_id  uuid        NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  version      integer     NOT NULL,
  prompt       text        NOT NULL,
  code_snippet jsonb,
  payload      jsonb       NOT NULL,
  explanation  text,
  archived_at  timestamptz NOT NULL DEFAULT now(),
  created_at   timestamptz NOT NULL DEFAULT now(),
  UNIQUE (question_id, version)
);

CREATE TABLE tb_question_tags (
  question_id uuid NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  tag_id      uuid NOT NULL REFERENCES tb_tags(id)      ON DELETE RESTRICT,
  PRIMARY KEY (question_id, tag_id)
);
CREATE INDEX tb_question_tags_tag ON tb_question_tags(tag_id, question_id);

CREATE TABLE tb_user_tag_ratings (
  user_id        uuid             NOT NULL REFERENCES tb_users(id) ON DELETE CASCADE,
  tag_id         uuid             NOT NULL REFERENCES tb_tags(id)  ON DELETE RESTRICT,
  rating         double precision NOT NULL DEFAULT 1200,
  attempts_count integer          NOT NULL DEFAULT 0,
  last_updated   timestamptz      NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, tag_id)
);
CREATE INDEX tb_user_tag_ratings_user_rating ON tb_user_tag_ratings(user_id, rating);

-- ─── tb_assessments ──────────────────────────────────────────────────────────

CREATE TABLE tb_assessments (
  id                  uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  title               text             NOT NULL,
  description         text,
  mode                text             NOT NULL CHECK (mode IN ('practice','graded')),
  status              text             NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','active','archived')),
  objectives          text[]           NOT NULL DEFAULT '{}',
  course              text,
  duration_min        integer,
  time_limit_seconds  integer,
  total_points        integer          NOT NULL DEFAULT 0,
  passing_points      integer,
  show_results_during boolean          NOT NULL DEFAULT FALSE,
  affects_rating      boolean          NOT NULL DEFAULT TRUE,
  method              text             NOT NULL DEFAULT 'manual' CHECK (method IN ('manual','agent')),
  composition_trace   jsonb,
  created_by          uuid             NOT NULL REFERENCES tb_users(id),
  created_at          timestamptz      NOT NULL DEFAULT now(),
  updated_at          timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX idx_assessments_created_by_status ON tb_assessments (created_by, status);

CREATE TABLE tb_assessment_sections (
  id            uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  assessment_id uuid             NOT NULL REFERENCES tb_assessments(id) ON DELETE CASCADE,
  title         text             NOT NULL,
  order_index   integer          NOT NULL,
  weight        double precision NOT NULL DEFAULT 1.0,
  mix           jsonb,
  items_count   integer          NOT NULL,
  created_at    timestamptz      NOT NULL DEFAULT now(),
  UNIQUE (assessment_id, order_index)
);
CREATE INDEX idx_assessment_sections_assessment ON tb_assessment_sections (assessment_id, order_index);

CREATE TABLE tb_assessment_items (
  section_id      uuid    NOT NULL REFERENCES tb_assessment_sections(id) ON DELETE CASCADE,
  question_id     uuid    NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  order_index     integer NOT NULL,
  points_override integer,
  PRIMARY KEY (section_id, question_id),
  UNIQUE (section_id, order_index)
);
CREATE INDEX idx_assessment_items_section ON tb_assessment_items (section_id, order_index);

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
