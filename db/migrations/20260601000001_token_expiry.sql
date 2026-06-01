ALTER TABLE tb_api_tokens ADD COLUMN expires_at TIMESTAMPTZ;
UPDATE tb_api_tokens SET expires_at = created_at + INTERVAL '30 days';
ALTER TABLE tb_api_tokens ALTER COLUMN expires_at SET NOT NULL;
ALTER TABLE tb_api_tokens ALTER COLUMN expires_at SET DEFAULT NOW() + INTERVAL '30 days';
