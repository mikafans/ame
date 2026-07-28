CREATE TABLE tb_learning_variants (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_activity_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE CASCADE,
    objective_id uuid NOT NULL REFERENCES tb_journey_objectives (id) ON DELETE CASCADE,
    generation_run_id uuid NOT NULL UNIQUE REFERENCES tb_generation_runs (id) ON DELETE CASCADE,
    variant_kind text NOT NULL,
    recommendation_reason text NOT NULL,
    requested_difficulty text,
    content jsonb,
    source_references jsonb NOT NULL DEFAULT '[]'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_learning_variants_kind_check CHECK (
        variant_kind IN ('explanation', 'example', 'difficulty')
    )
);

CREATE INDEX tb_learning_variants_activity
ON tb_learning_variants (subject_user_id, source_activity_id, created_at DESC);
