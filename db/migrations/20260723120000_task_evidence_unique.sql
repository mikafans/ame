-- A reviewed task/objective pair contributes at most one mastery evidence row.

CREATE UNIQUE INDEX tb_mastery_evidence_task_objective_unique
ON tb_mastery_evidence (task_submission_id, objective_id)
WHERE task_submission_id IS NOT NULL;
