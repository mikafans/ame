-- Fix 1: expand users.role to include spec-defined values
-- Old values: 'admin' | 'user' (no CHECK constraint, comment-only)
-- New values: 'learner' | 'instructor' | 'admin' | 'agent'
-- Migrate existing 'user' rows → 'learner' (closest semantic match)
UPDATE users SET role = 'learner' WHERE role = 'user';

ALTER TABLE users
    ADD CONSTRAINT users_role_check
    CHECK (role IN ('learner', 'instructor', 'admin', 'agent'));

-- Fix 2: remove legacy scopes from api_tokens CHECK constraint.
-- The P7 migration already expanded the CHECK list but kept the old scopes.
-- Drop the old constraint and replace with the spec-canonical scope set only.
ALTER TABLE api_tokens DROP CONSTRAINT IF EXISTS api_tokens_scopes_check;

ALTER TABLE api_tokens
    ADD CONSTRAINT api_tokens_scopes_check
    CHECK (scopes <@ ARRAY[
        'quiz.read', 'quiz.write',
        'attempt.read', 'attempt.write',
        'stats.read',
        'feedback.write',
        'plan.read', 'plan.write',
        'admin'
    ]::text[]);

-- Migrate any existing tokens that carry legacy scopes to their spec equivalents:
-- 'human'               → 'quiz.write' + 'attempt.write' + 'plan.write' + 'stats.read' + 'feedback.write'
-- 'agent:write-questions' → 'quiz.write'
-- 'agent:read-only'     → 'quiz.read'
UPDATE api_tokens t SET scopes = (
    SELECT array_agg(DISTINCT new_scope)
    FROM unnest(t.scopes) AS s
    CROSS JOIN LATERAL unnest(
        CASE s
            WHEN 'human'                 THEN ARRAY['quiz.read','quiz.write','attempt.read','attempt.write','stats.read','feedback.write','plan.read','plan.write']
            WHEN 'agent:write-questions' THEN ARRAY['quiz.write']
            WHEN 'agent:read-only'       THEN ARRAY['quiz.read']
            ELSE ARRAY[s]
        END
    ) AS new_scope
)
WHERE t.scopes && ARRAY['human','agent:write-questions','agent:read-only']::text[];
