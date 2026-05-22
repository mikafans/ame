-- Add course field to quizzes for library categorization
ALTER TABLE quizzes ADD COLUMN IF NOT EXISTS course text;
