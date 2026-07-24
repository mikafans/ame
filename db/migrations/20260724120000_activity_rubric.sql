-- Structured scoring rubric for task/application activities (issue #36 P6).
-- Nullable: most activities carry no rubric. Shape is validated at the API
-- boundary against the platform TaskRubric type; stored here as opaque jsonb.
ALTER TABLE tb_activities ADD COLUMN rubric jsonb;
