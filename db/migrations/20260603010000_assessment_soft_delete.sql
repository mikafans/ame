ALTER TABLE tb_assessments ADD COLUMN deleted_at timestamptz;
CREATE INDEX idx_assessments_not_deleted ON tb_assessments (created_by) WHERE deleted_at IS NULL;
