-- ─── Deep Dive requests ─────────────────────────────────────────────────────

CREATE TABLE tb_deep_dives (
  id                uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  owner_id          uuid        NOT NULL REFERENCES tb_users(id)      ON DELETE CASCADE,
  question_id       uuid        NOT NULL REFERENCES tb_questions(id)  ON DELETE CASCADE,
  source_session_id uuid        REFERENCES tb_sessions(id)            ON DELETE SET NULL,
  source_attempt_id uuid        REFERENCES tb_attempts(id)            ON DELETE SET NULL,
  status            text        NOT NULL DEFAULT 'requested',
  reason            text,
  body_markdown     text,
  created_by        uuid        REFERENCES tb_identities(id)          ON DELETE SET NULL,
  created_at        timestamptz NOT NULL DEFAULT now(),
  updated_at        timestamptz NOT NULL DEFAULT now(),
  published_at      timestamptz,
  archived_at       timestamptz,
  CONSTRAINT tb_deep_dives_status_check CHECK (
    status IN ('requested', 'drafting', 'published', 'needs_revision', 'archived')
  )
);

CREATE INDEX tb_deep_dives_owner_status_updated
  ON tb_deep_dives(owner_id, status, updated_at DESC);
CREATE INDEX tb_deep_dives_question_updated
  ON tb_deep_dives(question_id, updated_at DESC);
CREATE INDEX tb_deep_dives_source_session
  ON tb_deep_dives(source_session_id) WHERE source_session_id IS NOT NULL;
CREATE INDEX tb_deep_dives_source_attempt
  ON tb_deep_dives(source_attempt_id) WHERE source_attempt_id IS NOT NULL;

CREATE UNIQUE INDEX tb_deep_dives_active_source_unique
  ON tb_deep_dives(
    owner_id,
    question_id,
    coalesce(source_session_id, '00000000-0000-0000-0000-000000000000'::uuid),
    coalesce(source_attempt_id, '00000000-0000-0000-0000-000000000000'::uuid)
  )
  WHERE archived_at IS NULL;

CREATE TRIGGER tr_deep_dives_updated_at
  BEFORE UPDATE ON tb_deep_dives
  FOR EACH ROW
  EXECUTE FUNCTION fn_update_updated_at();
