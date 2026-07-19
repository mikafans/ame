-- AME clean-slate learning baseline.
-- This replaces the historical migration chain on a fresh database only.

DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ame_app') THEN
    CREATE ROLE ame_app WITH LOGIN PASSWORD 'postgres';
  END IF;
END
$$;

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE OR REPLACE FUNCTION uuid_generate_v7()
RETURNS uuid AS $$
DECLARE
  unix_ts_ms bytea;
  uuid_bytes bytea;
BEGIN
  unix_ts_ms := substring(int8send(floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint) FROM 3);
  uuid_bytes := uuid_send(gen_random_uuid());
  uuid_bytes := overlay(uuid_bytes placing unix_ts_ms FROM 1 FOR 6);
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

CREATE TABLE tb_users (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    email text,
    email_canonical text,
    password_hash text,
    display_name text NOT NULL,
    role text NOT NULL DEFAULT 'learner',
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_users_role_check CHECK (role IN ('learner', 'admin')),
    CONSTRAINT tb_users_status_check CHECK (status IN ('active', 'deactivated')),
    CONSTRAINT tb_users_email_canonical_check CHECK (
        email_canonical IS NULL OR email_canonical = lower(email_canonical)
    )
);
CREATE UNIQUE INDEX tb_users_email_canonical_unique ON tb_users (email_canonical) WHERE email_canonical IS NOT NULL;

CREATE TABLE tb_identities (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    identity_type text NOT NULL,
    owner_user_id uuid REFERENCES tb_users (id) ON DELETE CASCADE,
    label text NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_identities_type_check CHECK (identity_type IN ('human', 'agent', 'system')),
    CONSTRAINT tb_identities_status_check CHECK (status IN ('active', 'revoked')),
    CONSTRAINT tb_identities_owner_check CHECK (
        (identity_type = 'system' AND owner_user_id IS NULL)
        OR identity_type IN ('human', 'agent')
    )
);
CREATE INDEX tb_identities_owner ON tb_identities (owner_user_id) WHERE owner_user_id IS NOT NULL;

ALTER TABLE tb_users
ADD CONSTRAINT tb_users_identity_fk
FOREIGN KEY (id) REFERENCES tb_identities (id) DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE tb_agents (
    id uuid PRIMARY KEY REFERENCES tb_identities (id) ON DELETE CASCADE,
    owner_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    CONSTRAINT tb_agents_identity_type_check CHECK (id <> owner_user_id)
);
CREATE INDEX tb_agents_owner_active ON tb_agents (owner_user_id) WHERE revoked_at IS NULL;

CREATE TABLE tb_login_sessions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz
);
CREATE INDEX tb_login_sessions_user ON tb_login_sessions (user_id, created_at DESC);
CREATE INDEX tb_login_sessions_active ON tb_login_sessions (expires_at) WHERE revoked_at IS NULL;

CREATE TABLE tb_api_tokens (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    agent_id uuid NOT NULL REFERENCES tb_agents (id) ON DELETE CASCADE,
    name text NOT NULL,
    token_hash text NOT NULL UNIQUE,
    scopes text [] NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz,
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CONSTRAINT tb_api_tokens_scopes_check CHECK (cardinality(scopes) > 0)
);
CREATE INDEX tb_api_tokens_agent_active ON tb_api_tokens (agent_id) WHERE revoked_at IS NULL;

CREATE TABLE tb_template_versions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    template_id text NOT NULL,
    version integer NOT NULL,
    title text NOT NULL,
    definition jsonb NOT NULL,
    status text NOT NULL DEFAULT 'published',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_template_versions_version_check CHECK (version > 0),
    CONSTRAINT tb_template_versions_status_check CHECK (status IN ('draft', 'published', 'retired')),
    UNIQUE (template_id, version)
);

CREATE TABLE tb_learning_goals (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    template_version_id uuid REFERENCES tb_template_versions (id),
    raw_intent text NOT NULL,
    normalized_statement text NOT NULL,
    status text NOT NULL DEFAULT 'proposed',
    idempotency_key text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_learning_goals_status_check CHECK (status IN ('proposed', 'active', 'paused', 'completed', 'failed'))
);
CREATE UNIQUE INDEX tb_learning_goals_idempotency ON tb_learning_goals (subject_user_id, idempotency_key)
WHERE idempotency_key IS NOT NULL;
CREATE INDEX tb_learning_goals_subject ON tb_learning_goals (subject_user_id, created_at DESC);

CREATE TABLE tb_learning_journeys (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    goal_id uuid NOT NULL UNIQUE REFERENCES tb_learning_goals (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    promise text NOT NULL,
    status text NOT NULL DEFAULT 'onboarding',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_learning_journeys_status_check CHECK (
        status IN ('onboarding', 'active', 'paused', 'completed', 'failed')
    )
);
CREATE INDEX tb_learning_journeys_subject_status ON tb_learning_journeys (subject_user_id, status, updated_at DESC);

CREATE TABLE tb_skills (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    name text NOT NULL,
    normalized_name text NOT NULL,
    provenance jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (subject_user_id, normalized_name)
);
CREATE INDEX tb_skills_subject ON tb_skills (subject_user_id, normalized_name);

CREATE TABLE tb_journey_objectives (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    verb text NOT NULL,
    statement text NOT NULL,
    success_criteria text NOT NULL,
    order_index integer NOT NULL,
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (journey_id, order_index),
    CONSTRAINT tb_journey_objectives_status_check CHECK (status IN ('active', 'paused', 'completed')),
    CONSTRAINT tb_journey_objectives_order_check CHECK (order_index >= 0)
);
CREATE INDEX tb_journey_objectives_subject ON tb_journey_objectives (subject_user_id, journey_id, order_index);

CREATE TABLE tb_objective_skills (
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    skill_id uuid NOT NULL REFERENCES tb_skills (id) ON DELETE RESTRICT,
    PRIMARY KEY (objective_id, skill_id)
);

CREATE TABLE tb_agent_runs (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    actor_identity_id uuid NOT NULL REFERENCES tb_identities (id),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid REFERENCES tb_learning_journeys (id) ON DELETE SET NULL,
    template_version_id uuid REFERENCES tb_template_versions (id),
    operation text NOT NULL,
    provider text,
    provider_run_id text,
    status text NOT NULL DEFAULT 'running',
    input jsonb NOT NULL DEFAULT '{}'::jsonb,
    output jsonb,
    error jsonb,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    CONSTRAINT tb_agent_runs_status_check CHECK (status IN ('running', 'succeeded', 'failed', 'resumable'))
);
CREATE INDEX tb_agent_runs_subject ON tb_agent_runs (subject_user_id, started_at DESC);
CREATE INDEX tb_agent_runs_active ON tb_agent_runs (subject_user_id) WHERE status IN ('running', 'resumable');

CREATE TABLE tb_activities (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    source_run_id uuid REFERENCES tb_agent_runs (id) ON DELETE SET NULL,
    kind text NOT NULL,
    title text NOT NULL,
    order_index integer NOT NULL,
    payload_schema_version integer NOT NULL DEFAULT 1,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    status text NOT NULL DEFAULT 'proposed',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (journey_id, order_index),
    CONSTRAINT tb_activities_kind_check CHECK (kind IN (
        'explanation', 'example', 'diagnostic', 'practice', 'feedback',
        'application', 'reflection', 'milestone', 'timed_practice', 'recommendation'
    )),
    CONSTRAINT tb_activities_status_check CHECK (status IN ('proposed', 'ready', 'in_progress', 'completed', 'failed')),
    CONSTRAINT tb_activities_order_check CHECK (order_index >= 0),
    CONSTRAINT tb_activities_payload_version_check CHECK (payload_schema_version > 0)
);
CREATE INDEX tb_activities_journey_order ON tb_activities (journey_id, order_index);
CREATE INDEX tb_activities_subject_status ON tb_activities (subject_user_id, status, updated_at DESC);

CREATE TABLE tb_activity_objectives (
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE CASCADE,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    PRIMARY KEY (activity_id, objective_id)
);

CREATE TABLE tb_questions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    source_run_id uuid REFERENCES tb_agent_runs (id) ON DELETE SET NULL,
    kind text NOT NULL,
    prompt text NOT NULL,
    payload jsonb NOT NULL,
    explanation text,
    status text NOT NULL DEFAULT 'draft',
    version integer NOT NULL DEFAULT 1,
    points integer NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    prompt_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', coalesce(prompt, ''))) STORED,
    CONSTRAINT tb_questions_kind_check CHECK (kind IN ('mc', 'tf', 'short', 'essay', 'code')),
    CONSTRAINT tb_questions_status_check CHECK (status IN ('draft', 'live', 'archived')),
    CONSTRAINT tb_questions_version_check CHECK (version > 0),
    CONSTRAINT tb_questions_points_check CHECK (points >= 0)
);
CREATE INDEX tb_questions_subject_status ON tb_questions (subject_user_id, status, created_at DESC);
CREATE INDEX tb_questions_prompt_tsv ON tb_questions USING gin (prompt_tsv);

CREATE TABLE tb_question_versions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    question_id uuid NOT NULL REFERENCES tb_questions (id) ON DELETE CASCADE,
    version integer NOT NULL,
    prompt text NOT NULL,
    payload jsonb NOT NULL,
    explanation text,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (question_id, version)
);

CREATE TABLE tb_assessments (
    id uuid PRIMARY KEY REFERENCES tb_activities (id) ON DELETE CASCADE,
    mode text NOT NULL,
    time_limit_seconds integer,
    total_points integer NOT NULL DEFAULT 0,
    passing_points integer,
    show_results_during boolean NOT NULL DEFAULT FALSE,
    affects_rating boolean NOT NULL DEFAULT FALSE,
    CONSTRAINT tb_assessments_mode_check CHECK (mode IN ('practice', 'graded')),
    CONSTRAINT tb_assessments_time_check CHECK (time_limit_seconds IS NULL OR time_limit_seconds > 0),
    CONSTRAINT tb_assessments_points_check CHECK (total_points >= 0)
);

CREATE TABLE tb_assessment_items (
    assessment_id uuid NOT NULL REFERENCES tb_assessments (id) ON DELETE CASCADE,
    question_id uuid NOT NULL REFERENCES tb_questions (id) ON DELETE RESTRICT,
    order_index integer NOT NULL,
    points_override integer,
    PRIMARY KEY (assessment_id, question_id),
    UNIQUE (assessment_id, order_index),
    CONSTRAINT tb_assessment_items_order_check CHECK (order_index >= 0)
);

CREATE TABLE tb_learning_sessions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    actor_identity_id uuid NOT NULL REFERENCES tb_identities (id),
    status text NOT NULL DEFAULT 'in_progress',
    question_plan jsonb NOT NULL DEFAULT '[]'::jsonb,
    result jsonb,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    CONSTRAINT tb_learning_sessions_status_check CHECK (status IN ('in_progress', 'finished', 'abandoned'))
);
CREATE INDEX tb_learning_sessions_subject_started ON tb_learning_sessions (subject_user_id, started_at DESC);
CREATE INDEX tb_learning_sessions_journey_status ON tb_learning_sessions (journey_id, status, started_at DESC);

CREATE TABLE tb_attempts (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    session_id uuid NOT NULL REFERENCES tb_learning_sessions (id) ON DELETE CASCADE,
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    actor_identity_id uuid NOT NULL REFERENCES tb_identities (id),
    question_id uuid REFERENCES tb_questions (id) ON DELETE RESTRICT,
    question_version integer,
    response jsonb NOT NULL,
    presentation jsonb NOT NULL DEFAULT '{}'::jsonb,
    is_correct boolean,
    score double precision,
    feedback jsonb,
    grade_status text NOT NULL DEFAULT 'graded',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_attempts_grade_status_check CHECK (grade_status IN ('graded', 'pending_manual', 'failed')),
    CONSTRAINT tb_attempts_question_version_check CHECK (question_id IS NULL OR question_version IS NOT NULL)
);
CREATE INDEX tb_attempts_subject_created ON tb_attempts (subject_user_id, created_at DESC);
CREATE INDEX tb_attempts_session ON tb_attempts (session_id, created_at);
CREATE INDEX tb_attempts_activity ON tb_attempts (activity_id, created_at DESC);

CREATE TABLE tb_mastery_evidence (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    skill_id uuid REFERENCES tb_skills (id) ON DELETE RESTRICT,
    source_attempt_id uuid NOT NULL REFERENCES tb_attempts (id) ON DELETE RESTRICT,
    derivation_run_id uuid REFERENCES tb_agent_runs (id) ON DELETE SET NULL,
    evidence_type text NOT NULL,
    value double precision NOT NULL,
    explanation text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX tb_mastery_evidence_objective ON tb_mastery_evidence (subject_user_id, objective_id, created_at DESC);

CREATE TABLE tb_recommendations (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    activity_id uuid REFERENCES tb_activities (id) ON DELETE SET NULL,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    source_run_id uuid REFERENCES tb_agent_runs (id) ON DELETE SET NULL,
    reason text NOT NULL,
    evidence jsonb NOT NULL DEFAULT '[]'::jsonb,
    status text NOT NULL DEFAULT 'proposed',
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_recommendations_status_check CHECK (status IN ('proposed', 'accepted', 'dismissed', 'expired'))
);
CREATE INDEX tb_recommendations_subject_active ON tb_recommendations (subject_user_id, created_at DESC)
WHERE status = 'proposed';

CREATE TABLE tb_idempotency_keys (
    actor_identity_id uuid NOT NULL REFERENCES tb_identities (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    operation text NOT NULL,
    key text NOT NULL,
    request_hash text NOT NULL,
    response_status smallint,
    response_body jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (actor_identity_id, subject_user_id, operation, key)
);
CREATE INDEX tb_idempotency_keys_created ON tb_idempotency_keys (created_at);

CREATE TABLE tb_audit_events (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    actor_identity_id uuid REFERENCES tb_identities (id) ON DELETE SET NULL,
    subject_user_id uuid REFERENCES tb_users (id) ON DELETE SET NULL,
    operation text NOT NULL,
    target_type text,
    target_id uuid,
    result text NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_audit_events_result_check CHECK (result IN ('success', 'failure', 'denied'))
);
CREATE INDEX tb_audit_events_subject_created ON tb_audit_events (subject_user_id, created_at DESC);
CREATE INDEX tb_audit_events_created ON tb_audit_events (created_at DESC);

CREATE TABLE tb_settings (
    key text PRIMARY KEY,
    value jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tr_users_updated_at BEFORE UPDATE ON tb_users
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_identities_updated_at BEFORE UPDATE ON tb_identities
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_goals_updated_at BEFORE UPDATE ON tb_learning_goals
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_journeys_updated_at BEFORE UPDATE ON tb_learning_journeys
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_activities_updated_at BEFORE UPDATE ON tb_activities
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_questions_updated_at BEFORE UPDATE ON tb_questions
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();

GRANT USAGE ON SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON TABLES TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON SEQUENCES TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON FUNCTIONS TO ame_app;
