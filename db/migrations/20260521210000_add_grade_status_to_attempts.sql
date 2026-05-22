ALTER TABLE attempts ADD COLUMN grade_status text NOT NULL DEFAULT 'graded'
  CHECK (grade_status IN ('graded', 'pending_manual'));
ALTER TABLE attempts ADD COLUMN grader_notes text;

UPDATE attempts SET grade_status = 'pending_manual'
WHERE is_correct = false AND score = 0.0
  AND response ? 'body';
