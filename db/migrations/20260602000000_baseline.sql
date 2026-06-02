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
  expires_at   timestamptz NOT NULL DEFAULT now() + interval '30 days',
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
    owner_id = nullif(current_setting('app.owner', TRUE), '')::uuid 
    OR current_setting('app.is_admin', TRUE) = 'true'
  )
  WITH CHECK (
    owner_id = nullif(current_setting('app.owner', TRUE), '')::uuid 
    OR current_setting('app.is_admin', TRUE) = 'true'
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
