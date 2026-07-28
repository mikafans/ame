CREATE TABLE tb_portability_imports (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL,
    schema_version text NOT NULL,
    checksum text NOT NULL,
    manifest jsonb NOT NULL,
    imported_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_portability_imports_checksum_check CHECK (checksum ~ '^[0-9a-f]{64}$'),
    UNIQUE (subject_user_id, checksum)
);

CREATE INDEX tb_portability_imports_journey
ON tb_portability_imports (subject_user_id, journey_id, imported_at DESC);
