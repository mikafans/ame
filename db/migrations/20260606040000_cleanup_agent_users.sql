-- Delete agent rows from tb_users as they are now in tb_agents
DELETE FROM tb_users
WHERE role = 'agent';

-- Update tb_users_role_check constraint to only allow human roles
ALTER TABLE tb_users DROP CONSTRAINT tb_users_role_check;
ALTER TABLE tb_users ADD CONSTRAINT tb_users_role_check CHECK (role IN ('user', 'admin'));

-- Drop the legacy tb_agent_profiles table
DROP TABLE IF EXISTS tb_agent_profiles CASCADE;
