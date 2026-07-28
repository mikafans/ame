CREATE TABLE tb_review_events (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
    subject_user_id uuid NOT NULL REFERENCES tb_users (id) ON DELETE CASCADE,
    journey_id uuid NOT NULL REFERENCES tb_learning_journeys (id) ON DELETE CASCADE,
    review_item_id uuid NOT NULL REFERENCES tb_review_items (id) ON DELETE CASCADE,
    rating text NOT NULL,
    reviewed_at timestamptz NOT NULL,
    due_before timestamptz NOT NULL,
    due_after timestamptz NOT NULL,
    interval_days integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tb_review_events_rating_check CHECK (
        rating IN ('again', 'hard', 'good', 'easy')
    ),
    CONSTRAINT tb_review_events_interval_check CHECK (interval_days >= 0),
    UNIQUE (subject_user_id, review_item_id, reviewed_at)
);

CREATE INDEX tb_review_events_owner_journey_time
ON tb_review_events (subject_user_id, journey_id, reviewed_at DESC);

CREATE INDEX tb_learning_sessions_analytics
ON tb_learning_sessions (subject_user_id, journey_id, status, finished_at);

CREATE INDEX tb_attempts_analytics
ON tb_attempts (subject_user_id, status, review_status, graded_at);
