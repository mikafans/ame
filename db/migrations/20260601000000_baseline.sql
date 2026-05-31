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

-- ─── tb_users ───────────────────────────────────────────────────────────────

CREATE TABLE tb_users (
  id            uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  email         text        UNIQUE,
  password_hash text,
  display_name  text        NOT NULL,
  role          text        NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_users_role_check CHECK (role IN ('learner', 'instructor', 'admin', 'agent'))
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
  created_at   timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_api_tokens_scopes_check
    CHECK (
      scopes <@ ARRAY[
        'assessment.read', 'assessment.write', 'quiz.read', 'quiz.write', 'public.publish',
        'attempt.read', 'attempt.write',
        'stats.read',
        'feedback.write',
        'plan.read', 'plan.write',
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
  CONSTRAINT tb_questions_kind_check   CHECK (kind   IN ('mc', 'tf', 'short', 'essay', 'code')),
  CONSTRAINT tb_questions_status_check CHECK (status IN ('draft', 'live', 'archived')),
  CONSTRAINT tb_questions_points_nonnegative CHECK (points >= 0)
);
CREATE INDEX tb_questions_live_created ON tb_questions(created_at DESC) WHERE status = 'live';
CREATE INDEX tb_questions_status        ON tb_questions(status);

CREATE TABLE tb_question_versions (
  question_id  uuid        NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  version      integer     NOT NULL,
  prompt       text        NOT NULL,
  code_snippet jsonb,
  payload      jsonb       NOT NULL,
  explanation  text,
  archived_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (question_id, version)
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

-- ─── tb_quizzes ──────────────────────────────────────────────────────────────

CREATE TABLE tb_quizzes (
  id         uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  title      text        NOT NULL,
  status     text        NOT NULL DEFAULT 'draft',
  objectives text[]      NOT NULL DEFAULT '{}',
  course     text,
  created_by uuid        NOT NULL REFERENCES tb_users(id),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tb_quizzes_status_check CHECK (status IN ('draft', 'active', 'archived'))
);

CREATE TABLE tb_quiz_questions (
  quiz_id         uuid    NOT NULL REFERENCES tb_quizzes(id)   ON DELETE CASCADE,
  question_id     uuid    NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  order_index     integer NOT NULL,
  points_override integer,
  PRIMARY KEY (quiz_id, question_id),
  UNIQUE (quiz_id, order_index)
);

-- ─── tb_exams ───────────────────────────────────────────────────────────────

CREATE TABLE tb_exams (
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
  created_by          uuid             NOT NULL REFERENCES tb_users(id),
  created_at          timestamptz      NOT NULL DEFAULT now(),
  updated_at          timestamptz      NOT NULL DEFAULT now(),
  CONSTRAINT tb_exams_method_check CHECK (method IN ('manual', 'agent')),
  CONSTRAINT tb_exams_status_check  CHECK (status IN ('draft', 'published', 'archived'))
);

CREATE TABLE tb_exam_sections (
  id           uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  exam_id      uuid             NOT NULL REFERENCES tb_exams(id) ON DELETE CASCADE,
  title        text             NOT NULL,
  order_index  integer          NOT NULL,
  weight       double precision NOT NULL DEFAULT 1.0,
  question_ids uuid[],
  mix          jsonb,
  items_count  integer          NOT NULL,
  created_at   timestamptz      NOT NULL DEFAULT now(),
  CONSTRAINT tb_exam_sections_kind_check
    CHECK ((question_ids IS NOT NULL AND mix IS NULL)
        OR (question_ids IS NULL     AND mix IS NOT NULL))
);
CREATE INDEX tb_exam_sections_exam ON tb_exam_sections(exam_id, order_index);

-- ─── tb_sessions ────────────────────────────────────────────────────────────

CREATE TABLE tb_sessions (
  id              uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id         uuid        NOT NULL REFERENCES tb_users(id)   ON DELETE CASCADE,
  kind            text        NOT NULL DEFAULT 'quiz',
  exam_id         uuid        REFERENCES tb_exams(id),
  quiz_id         uuid        REFERENCES tb_quizzes(id),
  filter          jsonb,
  question_plan   jsonb       NOT NULL,
  status          text        NOT NULL DEFAULT 'in_progress',
  affects_rating  boolean     NOT NULL DEFAULT TRUE,
  rating_snapshot jsonb       NOT NULL DEFAULT '{}'::jsonb,
  result          jsonb,
  deadline_at     timestamptz,
  started_at      timestamptz NOT NULL DEFAULT now(),
  finished_at     timestamptz,
  CONSTRAINT tb_sessions_kind_link
    CHECK (
      (kind = 'exam'     AND exam_id IS NOT NULL AND quiz_id IS NULL)
      OR (kind = 'quiz'  AND quiz_id IS NOT NULL AND exam_id IS NULL)
      OR (kind = 'practice' AND quiz_id IS NULL  AND exam_id IS NULL)
    ),
  CONSTRAINT tb_sessions_status_check CHECK (status IN ('in_progress', 'finished', 'abandoned'))
);
CREATE INDEX tb_sessions_user_started ON tb_sessions(user_id, started_at DESC);

-- ─── tb_attempts ────────────────────────────────────────────────────────────

CREATE TABLE tb_attempts (
  id                     uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id                uuid             NOT NULL REFERENCES tb_users(id)      ON DELETE CASCADE,
  question_id            uuid             NOT NULL REFERENCES tb_questions(id)  ON DELETE RESTRICT,
  question_version       integer          NOT NULL,
  session_id             uuid             REFERENCES tb_sessions(id)            ON DELETE SET NULL,
  response               jsonb            NOT NULL,
  presentation           jsonb            NOT NULL DEFAULT '{}'::jsonb,
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
  link_quiz_id uuid        REFERENCES tb_quizzes(id),
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

-- ─── tb_quiz_item_stats ─────────────────────────────────────────────────────

CREATE TABLE tb_quiz_item_stats (
  quiz_id            uuid             NOT NULL REFERENCES tb_quizzes(id)   ON DELETE CASCADE,
  question_id        uuid             NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  correct_rate       double precision NOT NULL DEFAULT 0,
  avg_time_ms        double precision,
  discrimination_idx double precision,
  last_computed      timestamptz      NOT NULL DEFAULT now(),
  PRIMARY KEY (quiz_id, question_id)
);

-- ─── tb_study_plans ─────────────────────────────────────────────────────────

CREATE TABLE tb_study_plans (
  id           uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  user_id      uuid        NOT NULL REFERENCES tb_users(id),
  goal         text        NOT NULL,
  lookback_days bigint     NOT NULL,
  generated_at timestamptz NOT NULL DEFAULT now(),
  weeks        jsonb       NOT NULL DEFAULT '[]'
);
CREATE INDEX tb_study_plans_user ON tb_study_plans(user_id, generated_at DESC);

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

-- ─── tb_share_links ─────────────────────────────────────────────────────────

CREATE TABLE tb_share_links (
  id                  uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  kind                text        NOT NULL,
  target_id           uuid        NOT NULL,
  created_by_user_id  uuid        NOT NULL REFERENCES tb_users(id),
  visibility          text        NOT NULL DEFAULT 'public',
  include_explanation boolean     NOT NULL DEFAULT FALSE,
  include_score       boolean     NOT NULL DEFAULT FALSE,
  include_attribution boolean     NOT NULL DEFAULT TRUE,
  og_image_url        text,
  created_at          timestamptz NOT NULL DEFAULT now(),
  revoked_at          timestamptz,
  CONSTRAINT tb_share_links_kind_check       CHECK (kind       IN ('quiz', 'exam', 'item')),
  CONSTRAINT tb_share_links_visibility_check CHECK (visibility IN ('public', 'cohort'))
);
CREATE INDEX tb_share_links_target  ON tb_share_links(target_id, kind);
CREATE INDEX tb_share_links_creator ON tb_share_links(created_by_user_id, created_at DESC);

CREATE TABLE tb_anonymous_attempts (
  id          uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  share_id    uuid        NOT NULL REFERENCES tb_share_links(id) ON DELETE CASCADE,
  question_id uuid        NOT NULL REFERENCES tb_questions(id),
  ts          timestamptz NOT NULL DEFAULT now(),
  ip_hash     text        NOT NULL,
  response    jsonb       NOT NULL,
  is_correct  boolean
);
CREATE INDEX tb_anonymous_attempts_share ON tb_anonymous_attempts(share_id, ts DESC);

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
ALTER TABLE tb_questions
  ADD COLUMN IF NOT EXISTS prompt_tsv tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(prompt, ''))) STORED;

CREATE INDEX IF NOT EXISTS idx_tb_questions_prompt_tsv
  ON tb_questions USING gin (prompt_tsv);
-- Token hashing migrated from Argon2 (variable-length PHC string starting
-- with `$argon2`) to lowercase-hex SHA-256 (64 chars). Existing rows cannot
-- be verified by the new code path, so revoke them. Users will be issued a
-- fresh token on their next login.

UPDATE tb_api_tokens
SET revoked_at = now()
WHERE revoked_at IS NULL
  AND token_hash LIKE '$argon2%';
-- Default question-bank listing orders by created_at DESC. Without an index the
-- planner sorts the whole table (external merge, hundreds of MB to disk) just to
-- return one page. This btree lets the no-search list use an index scan + LIMIT.
-- id is the tiebreaker so pagination is stable when created_at values collide.
CREATE INDEX IF NOT EXISTS idx_questions_created_at
    ON tb_questions (created_at DESC, id DESC);
-- Filtering the bank by kind (mc/tf/short/essay/code) currently scans every row
-- to both count and order the matches. This composite btree serves a kind-scoped
-- listing the same way idx_questions_created_at serves the unfiltered one: the
-- leading kind narrows to the matching rows, and the trailing created_at DESC,
-- id DESC keys return them already ordered for an index scan + LIMIT (and an
-- index-only count of the kind).
CREATE INDEX IF NOT EXISTS idx_questions_kind_created_at
    ON tb_questions (kind, created_at DESC, id DESC);
-- Collapse role domain from {learner, instructor, admin, agent} to {user, admin, agent}
-- Promotes learner and instructor to the generic 'user' role.

ALTER TABLE tb_users DROP CONSTRAINT tb_users_role_check;

UPDATE tb_users
SET role = 'user'
WHERE role IN ('learner', 'instructor');

ALTER TABLE tb_users ADD CONSTRAINT tb_users_role_check CHECK (role IN ('user', 'admin', 'agent'));
-- AI-1.1: Add owner_user_id and agent_shape constraint
-- Agents are sub-accounts of their owner, token-only, and exactly one level deep.

ALTER TABLE tb_users ADD COLUMN owner_user_id uuid
  REFERENCES tb_users(id) ON DELETE CASCADE;

ALTER TABLE tb_users ADD CONSTRAINT tb_users_agent_shape CHECK (
  (role = 'agent' AND owner_user_id IS NOT NULL AND email IS NULL AND password_hash IS NULL)
  OR (role <> 'agent' AND owner_user_id IS NULL)
);
-- AI-2.1: Add tb_agent_profiles table
-- Per-agent config and freeform memory store.

CREATE OR REPLACE FUNCTION fn_update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE tb_agent_profiles (
  agent_user_id UUID PRIMARY KEY REFERENCES tb_users(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  focus_tags TEXT[] NOT NULL DEFAULT '{}',
  current_goal TEXT,
  next_target TEXT,
  memory JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Trigger to update updated_at
CREATE TRIGGER tr_agent_profiles_updated_at
  BEFORE UPDATE ON tb_agent_profiles
  FOR EACH ROW
  EXECUTE FUNCTION fn_update_updated_at();
-- AI-3.1: Add quiz visibility and drop anonymous attempts
-- Quizzes are now private, unlisted, or public. Anonymous attempts are deprecated.

ALTER TABLE tb_quizzes ADD COLUMN visibility text NOT NULL DEFAULT 'private'
  CHECK (visibility IN ('private', 'unlisted', 'public'));

CREATE INDEX idx_quizzes_public ON tb_quizzes (created_at DESC)
  WHERE visibility = 'public';

DROP TABLE IF EXISTS tb_anonymous_attempts CASCADE;
-- AI-4.1: Add plan column to tb_users
-- Default is 'free', constraint to {free, premium}.

ALTER TABLE tb_users ADD COLUMN plan text NOT NULL DEFAULT 'free'
  CHECK (plan IN ('free', 'premium'));
-- AI-4.3: Add 'public.publish' to tb_api_tokens scopes constraint

ALTER TABLE tb_api_tokens DROP CONSTRAINT tb_api_tokens_scopes_check;

ALTER TABLE tb_api_tokens ADD CONSTRAINT tb_api_tokens_scopes_check
  CHECK (
    scopes <@ ARRAY[
      'assessment.read', 'assessment.write', 'quiz.read', 'quiz.write', 'public.publish',
      'attempt.read', 'attempt.write',
      'stats.read',
      'feedback.write',
      'plan.read', 'plan.write',
      'public.publish',
      'admin'
    ]::text[]
    AND array_length(scopes, 1) >= 1
  );
-- AI-6.1: Create tb_audit_log table and index exactly per specification

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
-- Add deactivated_at column to tb_users to persistently disable accounts
ALTER TABLE tb_users ADD COLUMN deactivated_at timestamptz;
-- 20260531122435_assessment_unification.sql

-- 1. Create Assessment tables
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

CREATE TABLE tb_assessment_sections (
  id            uuid             PRIMARY KEY DEFAULT uuid_generate_v7(),
  assessment_id uuid             NOT NULL REFERENCES tb_assessments(id) ON DELETE CASCADE,
  title         text             NOT NULL,
  order_index   integer          NOT NULL,
  weight        double precision NOT NULL DEFAULT 1.0,
  mix           jsonb,
  items_count   integer          NOT NULL,
  UNIQUE (assessment_id, order_index)
);

CREATE TABLE tb_assessment_items (
  section_id      uuid    NOT NULL REFERENCES tb_assessment_sections(id) ON DELETE CASCADE,
  question_id     uuid    NOT NULL REFERENCES tb_questions(id) ON DELETE CASCADE,
  order_index     integer NOT NULL,
  points_override integer,
  PRIMARY KEY (section_id, question_id),
  UNIQUE (section_id, order_index)
);

-- 2. Data Migration

-- A. Migrate Quizzes to practice assessments
INSERT INTO tb_assessments (
  id, title, description, mode, status, objectives, course,
  created_by, created_at, updated_at
)
SELECT
  id, title, NULL, 'practice', status, objectives, course,
  created_by, created_at, updated_at
FROM tb_quizzes;

INSERT INTO tb_assessment_sections (
  assessment_id, title, order_index, weight, mix, items_count
)
SELECT
  id, 'Main Section', 0, 1.0, NULL, (SELECT count(*) FROM tb_quiz_questions WHERE quiz_id = tb_quizzes.id)
FROM tb_quizzes;

INSERT INTO tb_assessment_items (
  section_id, question_id, order_index, points_override
)
SELECT
  s.id, q.question_id, q.order_index, q.points_override
FROM tb_quiz_questions q
JOIN tb_assessment_sections s ON q.quiz_id = s.assessment_id;

-- B. Migrate Exams to graded assessments
INSERT INTO tb_assessments (
  id, title, description, mode, status, objectives, course,
  duration_min, time_limit_seconds, total_points, passing_points,
  show_results_during, affects_rating, method, composition_trace,
  created_by, created_at, updated_at
)
SELECT
  id, name, description, 'graded',
  CASE WHEN status = 'published' THEN 'active' ELSE status END,
  objectives, NULL,
  duration_min, time_limit_seconds, total_points, passing_points,
  show_results_during, affects_rating, method, composition_trace,
  created_by, created_at, updated_at
FROM tb_exams;

-- FIX: Insert sections with explicit ID = es.id so we can join items correctly
INSERT INTO tb_assessment_sections (
  id, assessment_id, title, order_index, weight, mix, items_count
)
SELECT
  id, exam_id, title, order_index, weight, mix, items_count
FROM tb_exam_sections;

INSERT INTO tb_assessment_items (
  section_id, question_id, order_index, points_override
)
SELECT
  es.id, qid, idx, NULL
FROM tb_exam_sections es
CROSS JOIN LATERAL unnest(es.question_ids) WITH ORDINALITY AS t(qid, idx)
WHERE es.question_ids IS NOT NULL;

-- 3. Update tb_sessions
ALTER TABLE tb_sessions ADD COLUMN assessment_id uuid REFERENCES tb_assessments(id);

UPDATE tb_sessions
SET assessment_id = COALESCE(quiz_id, exam_id)
WHERE quiz_id IS NOT NULL OR exam_id IS NOT NULL;
-- Add correct_answer column to tb_attempts to store the truth at the time of grading
ALTER TABLE tb_attempts ADD COLUMN correct_answer JSONB;
-- Migration: drop quiz_id from tb_sessions, add visibility to tb_assessments,
-- fix kind default, and add performance indexes.
--
-- Visibility semantics:
--   tb_assessments.visibility = 'private' (default) → owner-only
--   tb_assessments.visibility = 'public'             → all authenticated users
--   Sessions and attempts are always user-bound (user_id).

-- ── tb_sessions: remove legacy quiz_id ────────────────────────────────────────

-- 1. Drop legacy check constraint that references quiz_id
ALTER TABLE tb_sessions
    DROP CONSTRAINT IF EXISTS tb_sessions_kind_link;

-- 2. Drop FK from quiz_id → tb_quizzes
ALTER TABLE tb_sessions
    DROP CONSTRAINT IF EXISTS tb_sessions_quiz_id_fkey;

-- 3. Drop the quiz_id column
ALTER TABLE tb_sessions
    DROP COLUMN IF EXISTS quiz_id;

-- 4. Fix the kind column default (was 'quiz', now 'assessment')
ALTER TABLE tb_sessions
    ALTER COLUMN kind SET DEFAULT 'assessment';

-- 5. Clean status check constraint (no more quiz_id reference)
ALTER TABLE tb_sessions
    DROP CONSTRAINT IF EXISTS tb_sessions_status_check;

ALTER TABLE tb_sessions
    ADD CONSTRAINT tb_sessions_status_check
        CHECK (status IN ('in_progress', 'finished', 'abandoned'));

-- ── tb_assessments: add visibility column ─────────────────────────────────────
-- private (default): only the creator can see and start it.
-- public: any authenticated user can discover and start it (when status='active').

ALTER TABLE tb_assessments
    ADD COLUMN IF NOT EXISTS visibility text NOT NULL DEFAULT 'private'
        CHECK (visibility IN ('public', 'private'));

-- Existing active rows keep 'private' — authors opt-in to public explicitly.

-- ── Indexes: sessions (user_id + assessment_id are hot paths) ─────────────────

-- Supports: list_assessments completed/last_session_id correlated subqueries
CREATE INDEX IF NOT EXISTS idx_sessions_user_assessment_status
    ON tb_sessions (user_id, assessment_id, status)
    WHERE assessment_id IS NOT NULL;

-- Supports: ORDER BY finished_at DESC in last_session_id subquery
CREATE INDEX IF NOT EXISTS idx_sessions_assessment_finished
    ON tb_sessions (assessment_id, finished_at DESC)
    WHERE status = 'finished' AND assessment_id IS NOT NULL;

-- Supports: GET /v1/sessions (list_my_sessions) ordered by started_at
CREATE INDEX IF NOT EXISTS idx_sessions_user_status_started
    ON tb_sessions (user_id, status, started_at DESC);

-- Supports: exam session lookups
CREATE INDEX IF NOT EXISTS idx_sessions_exam_id
    ON tb_sessions (exam_id)
    WHERE exam_id IS NOT NULL;

-- ── Indexes: assessments ───────────────────────────────────────────────────────

-- Supports: learner library — public active assessments
CREATE INDEX IF NOT EXISTS idx_assessments_visibility_status
    ON tb_assessments (visibility, status);

-- Supports: author studio — owner's own assessments (any visibility/status)
CREATE INDEX IF NOT EXISTS idx_assessments_created_by_status
    ON tb_assessments (created_by, status);

-- ── Indexes: assessment items ──────────────────────────────────────────────────

-- Supports: build_assessment_plan question fetch by section
CREATE INDEX IF NOT EXISTS idx_assessment_items_section
    ON tb_assessment_items (section_id, order_index);

-- Supports: section → assessment join
CREATE INDEX IF NOT EXISTS idx_assessment_sections_assessment
    ON tb_assessment_sections (assessment_id, order_index);
