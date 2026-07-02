-- ─── Add agent_id audit columns ──────────────────────────────────────────────

ALTER TABLE tb_questions ADD COLUMN agent_id uuid REFERENCES tb_agents (id) ON DELETE SET NULL;
ALTER TABLE tb_assessments ADD COLUMN agent_id uuid REFERENCES tb_agents (id) ON DELETE SET NULL;
ALTER TABLE tb_sessions ADD COLUMN agent_id uuid REFERENCES tb_agents (id) ON DELETE SET NULL;

-- ─── Backfill agent_id and repoint attributions ─────────────────────────────

-- 1. tb_questions: Backfill agent_id and repoint created_by + owner_id
UPDATE tb_questions
SET agent_id = created_by
WHERE created_by IN (SELECT id FROM tb_agents);

UPDATE tb_questions
SET
    created_by = (
        SELECT owner_user_id FROM tb_agents
        WHERE id = created_by
    )
WHERE created_by IN (SELECT id FROM tb_agents);

UPDATE tb_questions
SET
    owner_id = (
        SELECT owner_user_id FROM tb_agents
        WHERE id = owner_id
    )
WHERE owner_id IN (SELECT id FROM tb_agents);

-- 2. tb_assessments: Backfill agent_id and repoint created_by + owner_id
UPDATE tb_assessments
SET agent_id = created_by
WHERE created_by IN (SELECT id FROM tb_agents);

UPDATE tb_assessments
SET
    created_by = (
        SELECT owner_user_id FROM tb_agents
        WHERE id = created_by
    )
WHERE created_by IN (SELECT id FROM tb_agents);

UPDATE tb_assessments
SET
    owner_id = (
        SELECT owner_user_id FROM tb_agents
        WHERE id = owner_id
    )
WHERE owner_id IN (SELECT id FROM tb_agents);

-- 3. tb_sessions: Backfill agent_id and repoint user_id
UPDATE tb_sessions
SET agent_id = user_id
WHERE user_id IN (SELECT id FROM tb_agents);

UPDATE tb_sessions
SET
    user_id = (
        SELECT owner_user_id FROM tb_agents
        WHERE id = user_id
    )
WHERE user_id IN (SELECT id FROM tb_agents);

-- ─── Indexes on agent_id ─────────────────────────────────────────────────────

CREATE INDEX idx_questions_agent_id ON tb_questions (agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_assessments_agent_id ON tb_assessments (agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_sessions_agent_id ON tb_sessions (agent_id) WHERE agent_id IS NOT NULL;

-- ─── Repoint tb_activity_log ──────────────────────────────────────────────────

ALTER TABLE tb_activity_log DROP CONSTRAINT tb_activity_log_agent_id_fkey;
ALTER TABLE tb_activity_log ADD CONSTRAINT tb_activity_log_agent_id_fkey FOREIGN KEY (agent_id) REFERENCES tb_agents (
    id
) ON DELETE CASCADE;
