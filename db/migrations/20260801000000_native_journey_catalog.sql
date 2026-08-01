ALTER TABLE tb_learning_goals
ADD COLUMN catalog_entry_id text,
ADD COLUMN catalog_entry_version integer,
ADD CONSTRAINT tb_learning_goals_catalog_entry_check CHECK (
    (catalog_entry_id IS NULL AND catalog_entry_version IS NULL)
    OR (catalog_entry_id IS NOT NULL AND catalog_entry_id <> '' AND catalog_entry_version > 0)
);

ALTER TABLE tb_learning_journeys
ADD COLUMN catalog_entry_id text,
ADD COLUMN catalog_entry_version integer,
ADD CONSTRAINT tb_learning_journeys_catalog_entry_check CHECK (
    (catalog_entry_id IS NULL AND catalog_entry_version IS NULL)
    OR (catalog_entry_id IS NOT NULL AND catalog_entry_id <> '' AND catalog_entry_version > 0)
);
