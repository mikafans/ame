-- Course revisions own their authored graph. The nullable legacy path keeps
-- existing learner and fixture journeys readable while new agent-authored
-- courses can be versioned without modifying published material.

ALTER TABLE tb_journey_objectives
ADD COLUMN course_revision_id uuid REFERENCES tb_course_revisions (id) ON DELETE RESTRICT;

ALTER TABLE tb_journey_chapters
ADD COLUMN course_revision_id uuid REFERENCES tb_course_revisions (id) ON DELETE RESTRICT;

ALTER TABLE tb_activities
ADD COLUMN course_revision_id uuid REFERENCES tb_course_revisions (id) ON DELETE RESTRICT;

ALTER TABLE tb_course_revisions
DROP CONSTRAINT tb_course_revisions_publish_check,
ADD CONSTRAINT tb_course_revisions_publish_check CHECK (
    (status = 'published' AND published_at IS NOT NULL)
    OR status <> 'published'
);

ALTER TABLE tb_journey_objectives
DROP CONSTRAINT tb_journey_objectives_journey_id_order_index_key;

ALTER TABLE tb_journey_chapters
DROP CONSTRAINT tb_journey_chapters_journey_id_order_index_key;

DROP INDEX tb_activities_chapter_order;
DROP INDEX tb_activities_journey_order_ungrouped;

CREATE UNIQUE INDEX tb_journey_objectives_course_revision_order
ON tb_journey_objectives (course_revision_id, order_index)
WHERE course_revision_id IS NOT NULL;

CREATE UNIQUE INDEX tb_journey_objectives_legacy_journey_order
ON tb_journey_objectives (journey_id, order_index)
WHERE course_revision_id IS NULL;

CREATE UNIQUE INDEX tb_journey_chapters_course_revision_order
ON tb_journey_chapters (course_revision_id, order_index)
WHERE course_revision_id IS NOT NULL;

CREATE UNIQUE INDEX tb_journey_chapters_legacy_journey_order
ON tb_journey_chapters (journey_id, order_index)
WHERE course_revision_id IS NULL;

CREATE UNIQUE INDEX tb_activities_course_revision_chapter_order
ON tb_activities (course_revision_id, chapter_id, order_index)
WHERE course_revision_id IS NOT NULL AND chapter_id IS NOT NULL;

CREATE UNIQUE INDEX tb_activities_course_revision_ungrouped_order
ON tb_activities (course_revision_id, order_index)
WHERE course_revision_id IS NOT NULL AND chapter_id IS NULL;

CREATE UNIQUE INDEX tb_activities_legacy_chapter_order
ON tb_activities (chapter_id, order_index)
WHERE course_revision_id IS NULL AND chapter_id IS NOT NULL;

CREATE UNIQUE INDEX tb_activities_legacy_journey_order_ungrouped
ON tb_activities (journey_id, order_index)
WHERE course_revision_id IS NULL AND chapter_id IS NULL;

CREATE INDEX tb_journey_objectives_course_revision
ON tb_journey_objectives (course_revision_id, order_index);

CREATE INDEX tb_journey_chapters_course_revision
ON tb_journey_chapters (course_revision_id, order_index);

CREATE INDEX tb_activities_course_revision
ON tb_activities (course_revision_id, chapter_id, order_index);
