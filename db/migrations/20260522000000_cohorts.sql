CREATE TABLE cohorts (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  name        text NOT NULL,
  description text,
  created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE cohort_memberships (
  id         uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  cohort_id  uuid NOT NULL REFERENCES cohorts(id) ON DELETE CASCADE,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  joined_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE(cohort_id, user_id)
);

CREATE INDEX cohort_memberships_cohort ON cohort_memberships(cohort_id);
CREATE INDEX cohort_memberships_user   ON cohort_memberships(user_id);
