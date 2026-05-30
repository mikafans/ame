-- Filtering the bank by kind (mc/tf/short/essay/code) currently scans every row
-- to both count and order the matches. This composite btree serves a kind-scoped
-- listing the same way idx_questions_created_at serves the unfiltered one: the
-- leading kind narrows to the matching rows, and the trailing created_at DESC,
-- id DESC keys return them already ordered for an index scan + LIMIT (and an
-- index-only count of the kind).
CREATE INDEX IF NOT EXISTS idx_questions_kind_created_at
    ON tb_questions (kind, created_at DESC, id DESC);
