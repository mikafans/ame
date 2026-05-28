ALTER TABLE tb_questions
  ADD COLUMN IF NOT EXISTS prompt_tsv tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(prompt, ''))) STORED;

CREATE INDEX IF NOT EXISTS idx_tb_questions_prompt_tsv
  ON tb_questions USING gin (prompt_tsv);
