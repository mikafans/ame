-- M3 application tasks: persist learner work independently from assessment attempts.

CREATE TABLE tb_task_submissions (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    task_id uuid NOT NULL REFERENCES tb_activities (id) ON DELETE RESTRICT,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    content_version integer NOT NULL,
    response jsonb NOT NULL,
    evaluation_method text NOT NULL,
    status text NOT NULL DEFAULT 'in_progress',
    review_status text NOT NULL DEFAULT 'not_required',
    score numeric(8, 3),
    feedback jsonb,
    submitted_at timestamptz,
    reviewed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_task_submissions_content_version_check CHECK (content_version > 0),
    CONSTRAINT tb_task_submissions_evaluation_method_check CHECK (
        evaluation_method IN ('self_review', 'automatic', 'agent', 'manual')
    ),
    CONSTRAINT tb_task_submissions_status_check CHECK (
        status IN ('in_progress', 'submitted', 'in_review', 'reviewed', 'rejected', 'abandoned')
    ),
    CONSTRAINT tb_task_submissions_review_status_check CHECK (
        review_status IN ('not_required', 'pending', 'complete')
    ),
    CONSTRAINT tb_task_submissions_score_check CHECK (score IS NULL OR (score >= 0 AND score <= 1))
);

CREATE INDEX tb_task_submissions_subject_task
ON tb_task_submissions (subject_user_id, task_id, created_at DESC);

CREATE INDEX tb_task_submissions_journey
ON tb_task_submissions (subject_user_id, journey_id, created_at DESC);
