-- Squashed baseline: consolidates all migrations up to 2026-05-21.
-- Safe to apply on a fresh DB only. Run via: make db-reset

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

-- ─── users ──────────────────────────────────────────────────────────────────

CREATE TABLE users (
  id            uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  email         text        UNIQUE,
  password_hash text,
  display_name  text        NOT NULL,
  role          text        NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT users_role_check CHECK (role IN ('learner', 'instructor', 'admin', 'agent'))
);

-- ─── api_tokens ─────────────────────────────────────────────────────────────

CREATE TABLE api_tokens (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id      uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name         text        NOT NULL,
  token_hash   text        NOT NULL,
  scopes       text[]      NOT NULL,
  last_used_at timestamptz,
  revoked_at   timestamptz,
  created_at   timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT api_tokens_scopes_check
    CHECK (
      scopes <@ ARRAY[
        'quiz.read', 'quiz.write',
        'attempt.read', 'attempt.write',
        'stats.read',
        'feedback.write',
        'plan.read', 'plan.write',
        'admin'
      ]::text[]
      AND array_length(scopes, 1) >= 1
    )
);
CREATE INDEX api_tokens_user_active ON api_tokens(user_id) WHERE revoked_at IS NULL;

-- ─── tags ───────────────────────────────────────────────────────────────────

CREATE TABLE tags (
  id          uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  name        text        NOT NULL UNIQUE,
  description text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tags_name_lowercase CHECK (name = lower(name))
);

-- ─── questions ──────────────────────────────────────────────────────────────

CREATE TABLE questions (
  id             uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
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
  created_by     uuid             NOT NULL REFERENCES users(id),
  created_at     timestamptz      NOT NULL DEFAULT now(),
  updated_at     timestamptz      NOT NULL DEFAULT now(),
  CONSTRAINT questions_kind_check   CHECK (kind   IN ('mc', 'tf', 'short', 'essay', 'code')),
  CONSTRAINT questions_status_check CHECK (status IN ('draft', 'live', 'archived')),
  CONSTRAINT questions_points_nonnegative CHECK (points >= 0)
);
CREATE INDEX questions_live_created ON questions(created_at DESC) WHERE status = 'live';
CREATE INDEX questions_status        ON questions(status);

CREATE TABLE question_versions (
  question_id  uuid        NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  version      integer     NOT NULL,
  prompt       text        NOT NULL,
  code_snippet jsonb,
  payload      jsonb       NOT NULL,
  explanation  text,
  archived_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (question_id, version)
);

CREATE TABLE question_tags (
  question_id uuid NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  tag_id      uuid NOT NULL REFERENCES tags(id)      ON DELETE RESTRICT,
  PRIMARY KEY (question_id, tag_id)
);
CREATE INDEX question_tags_tag ON question_tags(tag_id, question_id);

CREATE TABLE user_tag_ratings (
  user_id        uuid             NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tag_id         uuid             NOT NULL REFERENCES tags(id)  ON DELETE RESTRICT,
  rating         double precision NOT NULL DEFAULT 1200,
  attempts_count integer          NOT NULL DEFAULT 0,
  last_updated   timestamptz      NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, tag_id)
);
CREATE INDEX user_tag_ratings_user_rating ON user_tag_ratings(user_id, rating);

-- ─── quizzes ────────────────────────────────────────────────────────────────

CREATE TABLE quizzes (
  id         uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  title      text        NOT NULL,
  status     text        NOT NULL DEFAULT 'draft',
  objectives text[]      NOT NULL DEFAULT '{}',
  created_by uuid        NOT NULL REFERENCES users(id),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT quizzes_status_check CHECK (status IN ('draft', 'active', 'archived'))
);

CREATE TABLE quiz_questions (
  quiz_id         uuid    NOT NULL REFERENCES quizzes(id)   ON DELETE CASCADE,
  question_id     uuid    NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  order_index     integer NOT NULL,
  points_override integer,
  PRIMARY KEY (quiz_id, question_id),
  UNIQUE (quiz_id, order_index)
);

-- ─── exams ──────────────────────────────────────────────────────────────────

CREATE TABLE exams (
  id                  uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  name                text             NOT NULL,
  description         text,
  blueprint           jsonb            NOT NULL,
  method              text             NOT NULL DEFAULT 'manual',
  duration_min        integer,
  time_limit_seconds  integer,
  total_points        integer          NOT NULL DEFAULT 0,
  passing_points      integer,
  objectives          text[]           NOT NULL DEFAULT '{}',
  composition_trace   jsonb,
  show_results_during boolean          NOT NULL DEFAULT FALSE,
  affects_rating      boolean          NOT NULL DEFAULT TRUE,
  status              text             NOT NULL DEFAULT 'draft',
  created_by          uuid             NOT NULL REFERENCES users(id),
  created_at          timestamptz      NOT NULL DEFAULT now(),
  updated_at          timestamptz      NOT NULL DEFAULT now(),
  CONSTRAINT exams_method_check CHECK (method IN ('manual', 'agent')),
  CONSTRAINT exams_status_check  CHECK (status IN ('draft', 'published', 'archived'))
);

CREATE TABLE exam_sections (
  id           uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  exam_id      uuid             NOT NULL REFERENCES exams(id) ON DELETE CASCADE,
  title        text             NOT NULL,
  order_index  integer          NOT NULL,
  weight       double precision NOT NULL DEFAULT 1.0,
  question_ids uuid[],
  mix          jsonb,
  items_count  integer          NOT NULL,
  created_at   timestamptz      NOT NULL DEFAULT now(),
  CONSTRAINT exam_sections_kind_check
    CHECK ((question_ids IS NOT NULL AND mix IS NULL)
        OR (question_ids IS NULL     AND mix IS NOT NULL))
);
CREATE INDEX exam_sections_exam ON exam_sections(exam_id, order_index);

-- ─── sessions ───────────────────────────────────────────────────────────────

CREATE TABLE sessions (
  id              uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id         uuid        NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
  kind            text        NOT NULL DEFAULT 'quiz',
  exam_id         uuid        REFERENCES exams(id),
  quiz_id         uuid        REFERENCES quizzes(id),
  filter          jsonb,
  question_plan   jsonb       NOT NULL,
  status          text        NOT NULL DEFAULT 'in_progress',
  affects_rating  boolean     NOT NULL DEFAULT TRUE,
  rating_snapshot jsonb       NOT NULL DEFAULT '{}'::jsonb,
  result          jsonb,
  deadline_at     timestamptz,
  started_at      timestamptz NOT NULL DEFAULT now(),
  finished_at     timestamptz,
  CONSTRAINT sessions_kind_link
    CHECK (
      (kind = 'exam'     AND exam_id IS NOT NULL AND quiz_id IS NULL)
      OR (kind = 'quiz'  AND quiz_id IS NOT NULL AND exam_id IS NULL)
      OR (kind = 'practice' AND quiz_id IS NULL  AND exam_id IS NULL)
    ),
  CONSTRAINT sessions_status_check CHECK (status IN ('in_progress', 'finished', 'abandoned'))
);
CREATE INDEX sessions_user_started ON sessions(user_id, started_at DESC);

-- ─── attempts ───────────────────────────────────────────────────────────────

CREATE TABLE attempts (
  id                     uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id                uuid             NOT NULL REFERENCES users(id)      ON DELETE CASCADE,
  question_id            uuid             NOT NULL REFERENCES questions(id)  ON DELETE RESTRICT,
  question_version       integer          NOT NULL,
  session_id             uuid             REFERENCES sessions(id)            ON DELETE SET NULL,
  response               jsonb            NOT NULL,
  presentation           jsonb            NOT NULL DEFAULT '{}'::jsonb,
  is_correct             boolean          NOT NULL,
  score                  double precision NOT NULL,
  time_to_answer_ms      integer,
  rating_before_user_avg double precision NOT NULL,
  rating_before_question double precision NOT NULL,
  user_tag_deltas        jsonb            NOT NULL,
  question_delta         double precision NOT NULL,
  created_at             timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX attempts_user_created ON attempts(user_id, created_at DESC);
CREATE INDEX attempts_question     ON attempts(question_id, created_at DESC);
CREATE INDEX attempts_session      ON attempts(session_id) WHERE session_id IS NOT NULL;
CREATE UNIQUE INDEX attempts_session_question_unique
  ON attempts(session_id, question_id) WHERE session_id IS NOT NULL;

-- ─── level_mappings ─────────────────────────────────────────────────────────

CREATE TABLE level_mappings (
  id          uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  tag_pattern text             NOT NULL,
  label       text             NOT NULL,
  elo_min     double precision NOT NULL,
  elo_max     double precision NOT NULL,
  sort_order  integer          NOT NULL DEFAULT 0,
  created_at  timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX level_mappings_pattern ON level_mappings(tag_pattern);

-- ─── idempotency_keys ───────────────────────────────────────────────────────

CREATE TABLE idempotency_keys (
  token_id        uuid     NOT NULL REFERENCES api_tokens(id) ON DELETE CASCADE,
  key             text     NOT NULL,
  request_hash    text     NOT NULL,
  response_status smallint NOT NULL,
  response_body   jsonb    NOT NULL,
  created_at      timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (token_id, key)
);
CREATE INDEX idempotency_keys_created ON idempotency_keys(created_at);

-- ─── messages ───────────────────────────────────────────────────────────────

CREATE TABLE messages (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  from_user_id uuid        NOT NULL REFERENCES users(id),
  to_user_id   uuid        NOT NULL REFERENCES users(id),
  channel      text        NOT NULL,
  body         text        NOT NULL,
  link_quiz_id uuid        REFERENCES quizzes(id),
  status       text        NOT NULL DEFAULT 'queued',
  created_at   timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT messages_channel_check CHECK (channel IN ('in_app', 'email')),
  CONSTRAINT messages_status_check  CHECK (status  IN ('queued', 'delivered', 'read'))
);
CREATE INDEX messages_to_user ON messages(to_user_id, created_at DESC) WHERE status != 'read';

-- ─── webhooks ───────────────────────────────────────────────────────────────

CREATE TABLE webhooks (
  id          uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id     uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  url         text        NOT NULL,
  events      text[]      NOT NULL DEFAULT '{}',
  secret_hash text        NOT NULL,
  signing_key text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  revoked_at  timestamptz
);
CREATE INDEX webhooks_user_active ON webhooks(user_id) WHERE revoked_at IS NULL;

CREATE TABLE webhook_deliveries (
  id              uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  webhook_id      uuid        NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
  event           text        NOT NULL,
  payload         jsonb       NOT NULL,
  attempt_num     integer     NOT NULL DEFAULT 0,
  status          text        NOT NULL DEFAULT 'pending',
  last_error      text,
  next_attempt_at timestamptz,
  created_at      timestamptz NOT NULL DEFAULT now(),
  completed_at    timestamptz,
  CONSTRAINT webhook_deliveries_status_check CHECK (status IN ('pending', 'delivered', 'failed'))
);
CREATE INDEX webhook_deliveries_pending ON webhook_deliveries(next_attempt_at) WHERE status = 'pending';

-- ─── quiz_item_stats ────────────────────────────────────────────────────────

CREATE TABLE quiz_item_stats (
  quiz_id            uuid             NOT NULL REFERENCES quizzes(id)   ON DELETE CASCADE,
  question_id        uuid             NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  correct_rate       double precision NOT NULL DEFAULT 0,
  avg_time_ms        double precision,
  discrimination_idx double precision,
  last_computed      timestamptz      NOT NULL DEFAULT now(),
  PRIMARY KEY (quiz_id, question_id)
);

-- ─── study_plans ────────────────────────────────────────────────────────────

CREATE TABLE study_plans (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id      uuid        NOT NULL REFERENCES users(id),
  goal         text        NOT NULL,
  lookback_days bigint     NOT NULL,
  generated_at timestamptz NOT NULL DEFAULT now(),
  weeks        jsonb       NOT NULL DEFAULT '[]'
);
CREATE INDEX study_plans_user ON study_plans(user_id, generated_at DESC);

-- ─── activity_log ───────────────────────────────────────────────────────────

CREATE TABLE activity_log (
  id        uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  ts        timestamptz NOT NULL DEFAULT now(),
  agent_id  uuid        NOT NULL REFERENCES users(id),
  tool_name text        NOT NULL,
  method    text        NOT NULL,
  path      text        NOT NULL,
  status    integer     NOT NULL,
  note      text,
  target_id uuid
);
CREATE INDEX activity_log_agent ON activity_log(agent_id, ts DESC);

-- ─── share_links ────────────────────────────────────────────────────────────

CREATE TABLE share_links (
  id                  uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  kind                text        NOT NULL,
  target_id           uuid        NOT NULL,
  created_by_user_id  uuid        NOT NULL REFERENCES users(id),
  visibility          text        NOT NULL DEFAULT 'public',
  include_explanation boolean     NOT NULL DEFAULT FALSE,
  include_score       boolean     NOT NULL DEFAULT FALSE,
  include_attribution boolean     NOT NULL DEFAULT TRUE,
  og_image_url        text,
  created_at          timestamptz NOT NULL DEFAULT now(),
  revoked_at          timestamptz,
  CONSTRAINT share_links_kind_check       CHECK (kind       IN ('quiz', 'exam', 'item')),
  CONSTRAINT share_links_visibility_check CHECK (visibility IN ('public', 'cohort'))
);
CREATE INDEX share_links_target  ON share_links(target_id, kind);
CREATE INDEX share_links_creator ON share_links(created_by_user_id, created_at DESC);

CREATE TABLE anonymous_attempts (
  id          uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  share_id    uuid        NOT NULL REFERENCES share_links(id) ON DELETE CASCADE,
  question_id uuid        NOT NULL REFERENCES questions(id),
  ts          timestamptz NOT NULL DEFAULT now(),
  ip_hash     text        NOT NULL,
  response    jsonb       NOT NULL,
  is_correct  boolean
);
CREATE INDEX anonymous_attempts_share ON anonymous_attempts(share_id, ts DESC);
