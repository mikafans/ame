-- Default question-bank listing orders by created_at DESC. Without an index the
-- planner sorts the whole table (external merge, hundreds of MB to disk) just to
-- return one page. This btree lets the no-search list use an index scan + LIMIT.
-- id is the tiebreaker so pagination is stable when created_at values collide.
CREATE INDEX IF NOT EXISTS idx_questions_created_at
    ON tb_questions (created_at DESC, id DESC);
