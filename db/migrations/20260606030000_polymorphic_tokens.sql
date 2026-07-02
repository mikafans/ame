-- ─── Polymorphic API Tokens ──────────────────────────────────────────────────

ALTER TABLE tb_api_tokens ADD COLUMN agent_id uuid REFERENCES tb_agents (id) ON DELETE CASCADE;

ALTER TABLE tb_api_tokens ALTER COLUMN user_id DROP NOT NULL;

-- Backfill agent sub-account tokens to use the new polymorphic agent_id column
UPDATE tb_api_tokens
SET agent_id = user_id, user_id = NULL
WHERE user_id IN (SELECT id FROM tb_agents);

ALTER TABLE tb_api_tokens ADD CONSTRAINT chk_api_tokens_polymorphic CHECK (
    (user_id IS NOT NULL) <> (agent_id IS NOT NULL)
);
