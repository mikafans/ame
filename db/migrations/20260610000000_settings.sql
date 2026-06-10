-- Operator-tunable platform settings store (key/jsonb). A present row OVERRIDES
-- the ame.toml config default; an absent row means "use config". Admin-only table
-- (no RLS needed). Backs v0.2 #2 Settings & Feature Flags.
CREATE TABLE tb_settings (
    key text PRIMARY KEY,
    value jsonb NOT NULL,
    updated_by uuid REFERENCES tb_users (id) ON DELETE SET NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO tb_settings (key, value) VALUES ('maintenance_mode', 'false'::jsonb);
