-- AI-4.1: Add plan column to tb_users
-- Default is 'free', constraint to {free, premium}.

ALTER TABLE tb_users ADD COLUMN plan text NOT NULL DEFAULT 'free'
  CHECK (plan IN ('free', 'premium'));
