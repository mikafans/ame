-- M5 backfill: move legacy starter activities into the second curriculum chapter.

-- Older journeys were bootstrapped with one Foundations chapter. Only create a
-- second chapter when the journey has later activities to place in it. The
-- unique order constraint keeps this safe for journeys already backfilled by
-- an onboarding retry or a previous deployment.
INSERT INTO tb_journey_chapters (
    journey_id,
    subject_user_id,
    title,
    summary,
    order_index
)
SELECT
    j.id AS journey_id,
    j.subject_user_id,
    'Practice and application' AS title,
    format(
        'Use the model in examples and a bounded task for %s.',
        g.normalized_statement
    ) AS summary,
    1 AS order_index
FROM tb_learning_journeys AS j
INNER JOIN tb_learning_goals AS g ON j.goal_id = g.id
WHERE
    EXISTS (
        SELECT 1
        FROM tb_journey_chapters AS foundations
        WHERE
            foundations.journey_id = j.id
            AND foundations.order_index = 0
    )
    AND EXISTS (
        SELECT 1
        FROM tb_activities AS later_activity
        WHERE
            later_activity.journey_id = j.id
            AND later_activity.order_index >= 2
    )
ON CONFLICT (journey_id, order_index) DO NOTHING;

-- Restrict reassignment to journeys whose activities are still all in the
-- original chapter. This preserves intentionally authored multi-chapter
-- journeys while making the legacy starter path match the new blueprint.
WITH legacy_journeys AS (
    SELECT
        foundations.journey_id,
        foundations.id AS foundation_id,
        practice.id AS practice_id
    FROM tb_journey_chapters AS foundations
    INNER JOIN tb_journey_chapters AS practice
        ON
            foundations.journey_id = practice.journey_id
            AND practice.order_index = 1
    WHERE
        foundations.order_index = 0
        AND NOT EXISTS (
            SELECT 1
            FROM tb_activities AS authored_activity
            WHERE
                authored_activity.journey_id = foundations.journey_id
                AND authored_activity.chapter_id IS NOT NULL
                AND authored_activity.chapter_id <> foundations.id
        )
)

UPDATE tb_activities AS activity
SET chapter_id = legacy.practice_id
FROM legacy_journeys AS legacy
WHERE
    activity.journey_id = legacy.journey_id
    AND activity.chapter_id = legacy.foundation_id
    AND activity.order_index >= 2;
