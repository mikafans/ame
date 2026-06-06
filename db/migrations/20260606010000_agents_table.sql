-- ─── tb_agents ───────────────────────────────────────────────────────────────

CREATE TABLE tb_agents (
  id             uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  owner_user_id  uuid        NOT NULL REFERENCES tb_users(id) ON DELETE CASCADE,
  label          text        NOT NULL,
  focus_tags     text[]      NOT NULL DEFAULT '{}',
  current_goal   text,
  next_target    text,
  memory         jsonb       NOT NULL DEFAULT '{}',
  created_at     timestamptz NOT NULL DEFAULT now(),
  updated_at     timestamptz NOT NULL DEFAULT now(),
  deactivated_at timestamptz
);

CREATE TRIGGER tr_agents_updated_at
  BEFORE UPDATE ON tb_agents
  FOR EACH ROW
  EXECUTE FUNCTION fn_update_updated_at();

-- ─── Backfill tb_agents ───────────────────────────────────────────────────────

INSERT INTO tb_agents (
  id,
  owner_user_id,
  label,
  focus_tags,
  current_goal,
  next_target,
  memory,
  created_at,
  updated_at,
  deactivated_at
)
SELECT
  u.id,
  u.owner_user_id,
  p.label,
  p.focus_tags,
  p.current_goal,
  p.next_target,
  p.memory,
  p.created_at,
  p.updated_at,
  u.deactivated_at
FROM tb_users u
JOIN tb_agent_profiles p ON p.agent_user_id = u.id
WHERE u.role = 'agent';
