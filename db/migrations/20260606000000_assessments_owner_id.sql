ALTER TABLE tb_assessments ADD COLUMN owner_id uuid REFERENCES tb_users(id);

UPDATE tb_assessments SET owner_id = COALESCE((SELECT owner_user_id FROM tb_users u
WHERE u.id = created_by), created_by);

ALTER TABLE tb_assessments ALTER COLUMN owner_id SET NOT NULL;

CREATE INDEX idx_assessments_owner_id ON tb_assessments (owner_id);
