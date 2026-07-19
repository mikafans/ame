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
    CONSTRAINT tb_identities_type_check CHECK (identity_type IN ('human', 'system')),
    CONSTRAINT tb_identities_status_check CHECK (status IN ('active', 'revoked')),
    CONSTRAINT tb_identities_owner_check CHECK (
        (identity_type = 'system' AND owner_user_id IS NULL)
        OR identity_type = 'human'
    )
);
CREATE INDEX tb_identities_owner ON tb_identities (owner_user_id) WHERE owner_user_id IS NOT NULL;

ALTER TABLE tb_users
ADD CONSTRAINT tb_users_identity_fk
FOREIGN KEY (id) REFERENCES tb_identities (id) DEFERRABLE INITIALLY DEFERRED;

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

CREATE TABLE tb_activities (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
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
        'application', 'reflection', 'milestone', 'deep_dive', 'timed_practice', 'recommendation'
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

CREATE TABLE tb_generation_runs (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    operation text NOT NULL,
    provider text,
    template_version_id uuid REFERENCES tb_template_versions (id),
    content_version integer NOT NULL DEFAULT 1,
    retry_key text,
    status text NOT NULL DEFAULT 'requested',
    error jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_generation_runs_status_check CHECK (
        status IN ('requested', 'running', 'review_required', 'published', 'failed')
    ),
    CONSTRAINT tb_generation_runs_content_version_check CHECK (content_version > 0)
);
CREATE UNIQUE INDEX tb_generation_runs_retry ON tb_generation_runs (subject_user_id, operation, retry_key)
WHERE retry_key IS NOT NULL;
CREATE INDEX tb_generation_runs_subject_status ON tb_generation_runs (subject_user_id, status, created_at DESC);

CREATE TABLE tb_questions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    kind text NOT NULL,
    status text NOT NULL DEFAULT 'draft',
    current_version integer NOT NULL DEFAULT 1,
    generation_run_id uuid REFERENCES tb_generation_runs (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_questions_kind_check CHECK (
        kind IN ('multiple_choice', 'true_false', 'short_answer', 'essay', 'code')
    ),
    CONSTRAINT tb_questions_status_check CHECK (status IN ('draft', 'review', 'approved', 'retired')),
    CONSTRAINT tb_questions_version_check CHECK (current_version > 0)
);
CREATE INDEX tb_questions_subject_status ON tb_questions (subject_user_id, status, updated_at DESC);

CREATE TABLE tb_question_versions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    question_id uuid NOT NULL REFERENCES tb_questions (id) ON DELETE CASCADE,
    version integer NOT NULL,
    kind text NOT NULL,
    prompt text NOT NULL,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    explanation text,
    rationale text,
    points integer NOT NULL DEFAULT 1,
    difficulty text,
    source_references jsonb NOT NULL DEFAULT '[]'::jsonb,
    review_status text NOT NULL DEFAULT 'draft',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_question_versions_version_check CHECK (version > 0),
    CONSTRAINT tb_question_versions_kind_check CHECK (
        kind IN ('multiple_choice', 'true_false', 'short_answer', 'essay', 'code')
    ),
    CONSTRAINT tb_question_versions_points_check CHECK (points > 0),
    CONSTRAINT tb_question_versions_review_check CHECK (
        review_status IN ('draft', 'review', 'approved', 'rejected')
    ),
    UNIQUE (question_id, version)
);
CREATE INDEX tb_question_versions_question ON tb_question_versions (question_id, version DESC);

CREATE TABLE tb_assessments (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    activity_id uuid NOT NULL UNIQUE REFERENCES tb_activities (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    version integer NOT NULL DEFAULT 1,
    mode text NOT NULL,
    status text NOT NULL DEFAULT 'draft',
    time_limit_seconds integer,
    passing_score numeric(6, 3),
    result_visibility text NOT NULL DEFAULT 'immediate',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_assessments_version_check CHECK (version > 0),
    CONSTRAINT tb_assessments_mode_check CHECK (mode IN ('practice', 'graded')),
    CONSTRAINT tb_assessments_status_check CHECK (status IN ('draft', 'published', 'retired')),
    CONSTRAINT tb_assessments_time_limit_check CHECK (time_limit_seconds IS NULL OR time_limit_seconds > 0),
    CONSTRAINT tb_assessments_passing_score_check CHECK (passing_score IS NULL OR (passing_score >= 0 AND passing_score <= 1)),
    CONSTRAINT tb_assessments_visibility_check CHECK (result_visibility IN ('immediate', 'after_review'))
);
CREATE INDEX tb_assessments_subject_status ON tb_assessments (subject_user_id, status, updated_at DESC);

CREATE TABLE tb_assessment_sections (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    assessment_id uuid NOT NULL REFERENCES tb_assessments (id) ON DELETE CASCADE,
    title text NOT NULL,
    order_index integer NOT NULL,
    UNIQUE (assessment_id, order_index),
    CONSTRAINT tb_assessment_sections_order_check CHECK (order_index >= 0)
);

CREATE TABLE tb_assessment_items (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    assessment_id uuid NOT NULL REFERENCES tb_assessments (id) ON DELETE CASCADE,
    section_id uuid REFERENCES tb_assessment_sections (id) ON DELETE SET NULL,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE RESTRICT,
    question_version_id uuid NOT NULL REFERENCES tb_question_versions (id) ON DELETE RESTRICT,
    order_index integer NOT NULL,
    points_override integer,
    UNIQUE (assessment_id, order_index),
    CONSTRAINT tb_assessment_items_order_check CHECK (order_index >= 0),
    CONSTRAINT tb_assessment_items_points_check CHECK (points_override IS NULL OR points_override > 0)
);
CREATE INDEX tb_assessment_items_question ON tb_assessment_items (question_version_id);

CREATE TABLE tb_deep_dives (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    activity_id uuid NOT NULL UNIQUE REFERENCES tb_activities (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    objective_id uuid REFERENCES tb_journey_objectives (id) ON DELETE SET NULL,
    triggering_evidence_id uuid,
    source_actor_id uuid NOT NULL REFERENCES tb_identities (id),
    content_version integer NOT NULL DEFAULT 1,
    body jsonb NOT NULL,
    source_references jsonb NOT NULL DEFAULT '[]'::jsonb,
    review_status text NOT NULL DEFAULT 'draft',
    application_task jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_deep_dives_version_check CHECK (content_version > 0),
    CONSTRAINT tb_deep_dives_review_check CHECK (
        review_status IN ('draft', 'review', 'approved', 'rejected')
    )
);
CREATE INDEX tb_deep_dives_subject_objective ON tb_deep_dives (subject_user_id, objective_id, updated_at DESC);

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
    learning_session_id uuid NOT NULL REFERENCES tb_learning_sessions (id) ON DELETE CASCADE,
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    assessment_id uuid REFERENCES tb_assessments (id) ON DELETE RESTRICT,
    assessment_version integer,
    status text NOT NULL DEFAULT 'in_progress',
    score numeric(8, 3),
    max_score numeric(8, 3),
    review_status text NOT NULL DEFAULT 'not_required',
    submitted_at timestamptz,
    graded_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_attempts_status_check CHECK (status IN ('in_progress', 'submitted', 'graded', 'abandoned')),
    CONSTRAINT tb_attempts_review_check CHECK (review_status IN ('not_required', 'pending', 'complete')),
    CONSTRAINT tb_attempts_score_check CHECK (score IS NULL OR score >= 0),
    CONSTRAINT tb_attempts_max_score_check CHECK (max_score IS NULL OR max_score >= 0)
);
CREATE INDEX tb_attempts_subject_created ON tb_attempts (subject_user_id, created_at DESC);
CREATE INDEX tb_attempts_session ON tb_attempts (learning_session_id, created_at DESC);

CREATE TABLE tb_attempt_answers (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    attempt_id uuid NOT NULL REFERENCES tb_attempts (id) ON DELETE CASCADE,
    assessment_item_id uuid NOT NULL REFERENCES tb_assessment_items (id) ON DELETE RESTRICT,
    question_version_id uuid NOT NULL REFERENCES tb_question_versions (id) ON DELETE RESTRICT,
    response jsonb NOT NULL,
    correctness numeric(6, 3),
    awarded_points numeric(8, 3),
    evaluation_status text NOT NULL DEFAULT 'pending',
    feedback jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_attempt_answers_correctness_check CHECK (correctness IS NULL OR (correctness >= 0 AND correctness <= 1)),
    CONSTRAINT tb_attempt_answers_evaluation_check CHECK (
        evaluation_status IN ('pending', 'correct', 'incorrect', 'partial', 'manual_review')
    ),
    UNIQUE (attempt_id, assessment_item_id)
);
CREATE INDEX tb_attempt_answers_question ON tb_attempt_answers (question_version_id);

CREATE TABLE tb_mastery_evidence (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    skill_id uuid REFERENCES tb_skills (id) ON DELETE SET NULL,
    attempt_id uuid REFERENCES tb_attempts (id) ON DELETE SET NULL,
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    evidence_type text NOT NULL,
    value numeric(8, 3) NOT NULL,
    derivation_version integer NOT NULL DEFAULT 1,
    details jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_mastery_evidence_value_check CHECK (value >= 0 AND value <= 1),
    CONSTRAINT tb_mastery_evidence_version_check CHECK (derivation_version > 0)
);
CREATE INDEX tb_mastery_evidence_subject_objective ON tb_mastery_evidence (subject_user_id, objective_id, created_at DESC);

ALTER TABLE tb_deep_dives
ADD CONSTRAINT tb_deep_dives_evidence_fk
FOREIGN KEY (triggering_evidence_id) REFERENCES tb_mastery_evidence (id) ON DELETE SET NULL;

CREATE TABLE tb_mastery_snapshots (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    mastery numeric(8, 3) NOT NULL,
    confidence numeric(8, 3) NOT NULL,
    evidence_count integer NOT NULL DEFAULT 0,
    derivation_version integer NOT NULL DEFAULT 1,
    calculated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_mastery_snapshots_mastery_check CHECK (mastery >= 0 AND mastery <= 1),
    CONSTRAINT tb_mastery_snapshots_confidence_check CHECK (confidence >= 0 AND confidence <= 1),
    CONSTRAINT tb_mastery_snapshots_count_check CHECK (evidence_count >= 0),
    UNIQUE (subject_user_id, journey_id, objective_id)
);

CREATE TABLE tb_recommendations (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    objective_id uuid REFERENCES tb_journey_objectives (id) ON DELETE SET NULL,
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    source_generation_run_id uuid REFERENCES tb_generation_runs (id) ON DELETE SET NULL,
    reason text NOT NULL,
    evidence_ids jsonb NOT NULL DEFAULT '[]'::jsonb,
    status text NOT NULL DEFAULT 'proposed',
    recommendation_version integer NOT NULL DEFAULT 1,
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_recommendations_status_check CHECK (status IN ('proposed', 'accepted', 'dismissed', 'expired')),
    CONSTRAINT tb_recommendations_version_check CHECK (recommendation_version > 0)
);
CREATE INDEX tb_recommendations_subject_status ON tb_recommendations (subject_user_id, status, created_at DESC);

CREATE TABLE tb_streak_events (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid REFERENCES tb_learning_journeys (id) ON DELETE SET NULL,
    activity_id uuid REFERENCES tb_activities (id) ON DELETE SET NULL,
    qualifying_event_key text NOT NULL,
    learner_timezone text NOT NULL,
    qualifying_day date NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (subject_user_id, qualifying_event_key)
);
CREATE INDEX tb_streak_events_subject_day ON tb_streak_events (subject_user_id, qualifying_day DESC);

CREATE TABLE tb_idempotency_keys (
    token_id uuid NOT NULL REFERENCES tb_login_sessions (id) ON DELETE CASCADE,
    key text NOT NULL,
    request_hash text NOT NULL,
    response_status smallint,
    response_body jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (token_id, key)
);
CREATE INDEX tb_idempotency_keys_created ON tb_idempotency_keys (created_at);

CREATE TABLE tb_audit_log (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    actor_user_id uuid REFERENCES tb_users (id) ON DELETE SET NULL,
    action text NOT NULL,
    target_type text,
    target_id uuid,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX tb_audit_log_created ON tb_audit_log (created_at DESC);
CREATE INDEX tb_audit_log_action ON tb_audit_log (action, created_at DESC);

CREATE TABLE tb_settings (
    key text PRIMARY KEY,
    value jsonb NOT NULL,
    updated_by uuid REFERENCES tb_users (id) ON DELETE SET NULL,
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
CREATE TRIGGER tr_generation_runs_updated_at BEFORE UPDATE ON tb_generation_runs
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_questions_updated_at BEFORE UPDATE ON tb_questions
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_assessments_updated_at BEFORE UPDATE ON tb_assessments
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_deep_dives_updated_at BEFORE UPDATE ON tb_deep_dives
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_attempts_updated_at BEFORE UPDATE ON tb_attempts
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_attempt_answers_updated_at BEFORE UPDATE ON tb_attempt_answers
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER tr_recommendations_updated_at BEFORE UPDATE ON tb_recommendations
FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
GRANT USAGE ON SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ame_app;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON TABLES TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON SEQUENCES TO ame_app;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL PRIVILEGES ON FUNCTIONS TO ame_app;
