ALTER TABLE tb_task_submissions
ADD COLUMN artifacts jsonb NOT NULL DEFAULT '[]'::jsonb,
ADD COLUMN revision integer NOT NULL DEFAULT 1,
ADD COLUMN parent_submission_id uuid REFERENCES tb_task_submissions (id) ON DELETE RESTRICT,
ADD COLUMN reviewer_user_id uuid REFERENCES tb_users (id) ON DELETE SET NULL,
ADD COLUMN rubric_scores jsonb,
ADD COLUMN review_provenance jsonb;

ALTER TABLE tb_task_submissions
ADD CONSTRAINT tb_task_submissions_revision_check CHECK (revision > 0);

CREATE UNIQUE INDEX tb_task_submissions_parent_unique
ON tb_task_submissions (parent_submission_id)
WHERE parent_submission_id IS NOT NULL;
