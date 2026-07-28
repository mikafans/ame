CREATE TABLE tb_learner_notes (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL,
    activity_id uuid,
    content_version integer,
    body text NOT NULL,
    retry_key text NOT NULL,
    revision integer NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_learner_notes_anchor_check CHECK (
        (activity_id IS NULL AND content_version IS NULL)
        OR (activity_id IS NOT NULL AND content_version > 0)
    ),
    CONSTRAINT tb_learner_notes_body_check CHECK (
        length(btrim(body)) > 0 AND octet_length(body) <= 16384
    ),
    UNIQUE (subject_user_id, retry_key)
);

CREATE INDEX tb_learner_notes_anchor
ON tb_learner_notes (subject_user_id, journey_id, activity_id, updated_at DESC);
