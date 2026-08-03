-- A learner may create a short-lived, revocable bearer capability for a course
-- authoring agent. The secret is never persisted and is intentionally not a
-- browser cookie or a login session.
ALTER TABLE tb_identities
    DROP CONSTRAINT tb_identities_type_check,
    ADD CONSTRAINT tb_identities_type_check
        CHECK (identity_type IN ('human', 'agent', 'system'));

ALTER TABLE tb_identities
    DROP CONSTRAINT tb_identities_owner_check,
    ADD CONSTRAINT tb_identities_owner_check CHECK (
        (identity_type = 'system' AND owner_user_id IS NULL)
        OR identity_type IN ('human', 'agent')
    );

CREATE TABLE tb_agent_delegations (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    actor_identity_id uuid NOT NULL UNIQUE REFERENCES tb_identities (id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    goal text NOT NULL,
    scope text NOT NULL DEFAULT 'course_author',
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    last_used_at timestamptz,
    CONSTRAINT tb_agent_delegations_scope_check CHECK (scope = 'course_author'),
    CONSTRAINT tb_agent_delegations_expiry_check CHECK (expires_at > created_at)
);

CREATE INDEX tb_agent_delegations_subject
    ON tb_agent_delegations (subject_user_id, created_at DESC);
CREATE INDEX tb_agent_delegations_active
    ON tb_agent_delegations (expires_at) WHERE revoked_at IS NULL;
