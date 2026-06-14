-- Add source to tb_question_versions to mirror external reference link
ALTER TABLE tb_question_versions ADD COLUMN source text;
