-- AI-4.3: Add 'public.publish' to tb_api_tokens scopes constraint

ALTER TABLE tb_api_tokens DROP CONSTRAINT tb_api_tokens_scopes_check;

ALTER TABLE tb_api_tokens ADD CONSTRAINT tb_api_tokens_scopes_check
  CHECK (
    scopes <@ ARRAY[
      'quiz.read', 'quiz.write',
      'attempt.read', 'attempt.write',
      'stats.read',
      'feedback.write',
      'plan.read', 'plan.write',
      'public.publish',
      'admin'
    ]::text[]
    AND array_length(scopes, 1) >= 1
  );
