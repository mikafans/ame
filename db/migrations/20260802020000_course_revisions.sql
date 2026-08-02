-- 0.5 course publication boundary. A journey remains the learner-owned aggregate;
-- this table records the author-facing course brief and immutable publication
-- decisions for that aggregate.

CREATE TABLE tb_course_revisions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    version integer NOT NULL,
    status text NOT NULL DEFAULT 'draft',
    brief jsonb NOT NULL,
    source_references jsonb NOT NULL DEFAULT '[]'::jsonb,
    validation jsonb NOT NULL DEFAULT '[]'::jsonb,
    review jsonb,
    published_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (journey_id, version),
    CONSTRAINT tb_course_revisions_version_check CHECK (version > 0),
    CONSTRAINT tb_course_revisions_status_check CHECK (
        status IN ('draft', 'review', 'published', 'retired')
    ),
    CONSTRAINT tb_course_revisions_publish_check CHECK (
        (status = 'published') = (published_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX tb_course_revisions_one_active
ON tb_course_revisions (journey_id)
WHERE status IN ('draft', 'review');

CREATE INDEX tb_course_revisions_subject_journey
ON tb_course_revisions (subject_user_id, journey_id, version DESC);
