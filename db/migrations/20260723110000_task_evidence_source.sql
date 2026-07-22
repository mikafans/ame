-- M3: mastery evidence may originate from a reviewed task submission.

ALTER TABLE tb_mastery_evidence
ADD COLUMN task_submission_id uuid REFERENCES tb_task_submissions (id) ON DELETE RESTRICT;

ALTER TABLE tb_mastery_evidence
ADD CONSTRAINT tb_mastery_evidence_single_source_check
CHECK ((attempt_id IS NULL) <> (task_submission_id IS NULL));

CREATE INDEX tb_mastery_evidence_task_submission
ON tb_mastery_evidence (task_submission_id)
WHERE task_submission_id IS NOT NULL;
