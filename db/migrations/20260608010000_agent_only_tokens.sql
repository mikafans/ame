-- ─── Agent-Only API Tokens ──────────────────────────────────────────────────

ALTER TABLE tb_api_tokens DROP CONSTRAINT chk_api_tokens_polymorphic;

DROP INDEX tb_api_tokens_user_active;

DELETE FROM tb_api_tokens
WHERE agent_id IS NULL;

ALTER TABLE tb_api_tokens ALTER COLUMN agent_id SET NOT NULL;

ALTER TABLE tb_api_tokens DROP COLUMN user_id;

CREATE INDEX tb_api_tokens_agent_active ON tb_api_tokens(agent_id) WHERE revoked_at IS NULL;
