-- AI-1.1: Add owner_user_id and agent_shape constraint
-- Agents are sub-accounts of their owner, token-only, and exactly one level deep.

ALTER TABLE tb_users ADD COLUMN owner_user_id uuid
  REFERENCES tb_users(id) ON DELETE CASCADE;

ALTER TABLE tb_users ADD CONSTRAINT tb_users_agent_shape CHECK (
  (role = 'agent' AND owner_user_id IS NOT NULL AND email IS NULL AND password_hash IS NULL)
  OR (role <> 'agent' AND owner_user_id IS NULL)
);
