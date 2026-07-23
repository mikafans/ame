-- Widen the question kind CHECK constraints to admit the numeric answer type
-- (numeric grading with tolerance/range). Additive: existing kinds are kept.
ALTER TABLE tb_questions DROP CONSTRAINT IF EXISTS tb_questions_kind_check;
ALTER TABLE tb_questions ADD CONSTRAINT tb_questions_kind_check CHECK (
    kind IN ('multiple_choice', 'true_false', 'short_answer', 'numeric', 'essay', 'code')
);

ALTER TABLE tb_question_versions DROP CONSTRAINT IF EXISTS tb_question_versions_kind_check;
ALTER TABLE tb_question_versions ADD CONSTRAINT tb_question_versions_kind_check CHECK (
    kind IN ('multiple_choice', 'true_false', 'short_answer', 'numeric', 'essay', 'code')
);
