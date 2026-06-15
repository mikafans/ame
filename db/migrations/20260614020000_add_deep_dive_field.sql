-- ─── Add deep_dive field to questions and versions ──────────────────────────

ALTER TABLE tb_questions
ADD COLUMN deep_dive text;

ALTER TABLE tb_question_versions
ADD COLUMN deep_dive text;
