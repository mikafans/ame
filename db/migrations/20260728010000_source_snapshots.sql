CREATE TABLE tb_sources (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    source_kind text NOT NULL,
    locator text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_sources_kind_check CHECK (source_kind IN ('url', 'document', 'local_file'))
);

CREATE INDEX tb_sources_subject_created
ON tb_sources (subject_user_id, created_at DESC);

CREATE TABLE tb_source_import_runs (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    source_id uuid NOT NULL REFERENCES tb_sources (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    retry_key text NOT NULL,
    status text NOT NULL,
    error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    CONSTRAINT tb_source_import_runs_status_check CHECK (
        status IN ('requested', 'completed', 'failed')
    ),
    UNIQUE (subject_user_id, retry_key)
);

CREATE TABLE tb_source_snapshots (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    source_id uuid NOT NULL REFERENCES tb_sources (id) ON DELETE CASCADE,
    import_run_id uuid NOT NULL UNIQUE REFERENCES tb_source_import_runs (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    media_type text NOT NULL,
    content_sha256 text NOT NULL,
    byte_length integer NOT NULL,
    content bytea NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_source_snapshots_length_check CHECK (byte_length > 0 AND byte_length <= 2097152),
    CONSTRAINT tb_source_snapshots_digest_check CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    UNIQUE (subject_user_id, source_id, content_sha256)
);

CREATE INDEX tb_source_snapshots_subject_created
ON tb_source_snapshots (subject_user_id, created_at DESC);
