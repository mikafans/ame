ALTER TABLE tb_deep_dives
ADD COLUMN category TEXT,
ADD COLUMN user_note TEXT,
ADD COLUMN note_updated_at TIMESTAMPTZ;

CREATE TABLE tb_deep_dive_revisions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    deep_dive_id UUID NOT NULL REFERENCES tb_deep_dives (id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    body_markdown TEXT,
    category TEXT,
    user_note TEXT,
    created_by UUID REFERENCES tb_identities (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX tb_deep_dive_revisions_unique ON tb_deep_dive_revisions (deep_dive_id, revision);
