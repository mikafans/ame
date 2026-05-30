-- AI-2.1: Add tb_agent_profiles table
-- Per-agent config and freeform memory store.

CREATE OR REPLACE FUNCTION fn_update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE tb_agent_profiles (
  agent_user_id UUID PRIMARY KEY REFERENCES tb_users(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  focus_tags TEXT[] NOT NULL DEFAULT '{}',
  current_goal TEXT,
  next_target TEXT,
  memory JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Trigger to update updated_at
CREATE TRIGGER tr_agent_profiles_updated_at
  BEFORE UPDATE ON tb_agent_profiles
  FOR EACH ROW
  EXECUTE FUNCTION fn_update_updated_at();
