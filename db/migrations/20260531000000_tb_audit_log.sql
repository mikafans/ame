-- AI-6.1: Create tb_audit_log table and index exactly per specification

CREATE TABLE tb_audit_log (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  actor_user_id  uuid REFERENCES tb_users(id) ON DELETE SET NULL,
  action         text NOT NULL,
  target_type    text,
  target_id      uuid,
  metadata       jsonb NOT NULL DEFAULT '{}',
  created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_created ON tb_audit_log (created_at DESC);
