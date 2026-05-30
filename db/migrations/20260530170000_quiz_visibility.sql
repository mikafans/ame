-- AI-3.1: Add quiz visibility and drop anonymous attempts
-- Quizzes are now private, unlisted, or public. Anonymous attempts are deprecated.

ALTER TABLE tb_quizzes ADD COLUMN visibility text NOT NULL DEFAULT 'private'
  CHECK (visibility IN ('private', 'unlisted', 'public'));

CREATE INDEX idx_quizzes_public ON tb_quizzes (created_at DESC)
  WHERE visibility = 'public';

DROP TABLE IF EXISTS tb_anonymous_attempts CASCADE;
