-- M1 curriculum structure: ordered chapters and explicit activity content state.

CREATE TABLE tb_journey_chapters (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    title text NOT NULL,
    summary text NOT NULL,
    order_index integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (journey_id, order_index),
    CONSTRAINT tb_journey_chapters_order_check CHECK (order_index >= 0)
);
CREATE INDEX tb_journey_chapters_subject_order
ON tb_journey_chapters (subject_user_id, journey_id, order_index);

ALTER TABLE tb_activities
ADD COLUMN chapter_id uuid REFERENCES tb_journey_chapters (id) ON DELETE SET NULL,
ADD COLUMN content_version integer NOT NULL DEFAULT 1,
ADD COLUMN publication_status text NOT NULL DEFAULT 'published';

ALTER TABLE tb_activities
DROP CONSTRAINT tb_activities_journey_id_order_index_key,
ADD CONSTRAINT tb_activities_content_version_check CHECK (content_version > 0),
ADD CONSTRAINT tb_activities_publication_status_check CHECK (
    publication_status IN ('draft', 'review', 'published', 'retired')
);

CREATE UNIQUE INDEX tb_activities_chapter_order
ON tb_activities (chapter_id, order_index)
WHERE chapter_id IS NOT NULL;
CREATE INDEX tb_activities_journey_chapter_order
ON tb_activities (journey_id, chapter_id, order_index);
