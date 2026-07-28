CREATE TABLE tb_review_items (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    content_version integer NOT NULL,
    source_evidence_id uuid NOT NULL REFERENCES tb_mastery_evidence (id) ON DELETE CASCADE,
    due_at timestamptz NOT NULL,
    last_reviewed_at timestamptz,
    interval_days integer NOT NULL DEFAULT 0,
    stability real,
    difficulty real,
    review_count integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_review_items_content_version_check CHECK (content_version > 0),
    CONSTRAINT tb_review_items_interval_check CHECK (interval_days >= 0),
    CONSTRAINT tb_review_items_review_count_check CHECK (review_count >= 0),
    CONSTRAINT tb_review_items_memory_check CHECK (
        (stability IS NULL AND difficulty IS NULL)
        OR (stability > 0 AND difficulty BETWEEN 1 AND 10)
    ),
    UNIQUE (subject_user_id, source_evidence_id)
);

CREATE INDEX tb_review_items_subject_due
ON tb_review_items (subject_user_id, due_at);

CREATE FUNCTION seed_review_item_from_evidence()
RETURNS trigger AS $$
BEGIN
    INSERT INTO tb_review_items (
        subject_user_id, journey_id, objective_id, activity_id,
        content_version, source_evidence_id, due_at
    )
    SELECT NEW.subject_user_id, NEW.journey_id, NEW.objective_id, NEW.activity_id,
           activity.content_version, NEW.id, NEW.created_at
      FROM tb_activities activity
     WHERE activity.id = NEW.activity_id
    ON CONFLICT (subject_user_id, source_evidence_id) DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER tr_mastery_evidence_seed_review
AFTER INSERT ON tb_mastery_evidence
FOR EACH ROW EXECUTE FUNCTION seed_review_item_from_evidence();
