CREATE TABLE tb_citations (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    snapshot_id uuid NOT NULL REFERENCES tb_source_snapshots (id) ON DELETE RESTRICT,
    start_byte integer NOT NULL,
    end_byte integer NOT NULL,
    quote text NOT NULL,
    extraction_method text NOT NULL,
    grounding_status text NOT NULL,
    grounding_note text NOT NULL,
    license_status text NOT NULL,
    license_name text,
    license_url text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_citations_range_check CHECK (start_byte >= 0 AND end_byte > start_byte),
    CONSTRAINT tb_citations_extraction_check CHECK (
        extraction_method IN ('exact_quote', 'manual_selection')
    ),
    CONSTRAINT tb_citations_grounding_check CHECK (
        grounding_status IN ('supported', 'contradicted', 'unverified')
    ),
    CONSTRAINT tb_citations_license_check CHECK (
        license_status IN ('allowed', 'restricted', 'unknown')
    ),
    UNIQUE (subject_user_id, snapshot_id, start_byte, end_byte)
);

CREATE INDEX tb_citations_subject_created
ON tb_citations (subject_user_id, created_at DESC);
