-- Add deactivated_at column to tb_users to persistently disable accounts
ALTER TABLE tb_users ADD COLUMN deactivated_at timestamptz;
