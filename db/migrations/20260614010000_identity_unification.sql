-- ─── 1. Create tb_identities ──────────────────────────────────────────────────
CREATE TABLE tb_identities (
  id         uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  type       text        NOT NULL CHECK (type IN ('human', 'agent')),
  created_at timestamptz NOT NULL DEFAULT now()
);

-- ─── 2. Populate tb_identities with existing users and agents ────────────────
INSERT INTO tb_identities (id, type, created_at)
SELECT
id,
'human',
created_at
FROM tb_users;

INSERT INTO tb_identities (id, type, created_at)
SELECT
id,
'agent',
created_at
FROM tb_agents;

-- ─── 3. Add foreign key constraints on tb_users and tb_agents ────────────────
ALTER TABLE tb_users ADD CONSTRAINT tb_users_id_identities_fk FOREIGN KEY (id) REFERENCES tb_identities(id) ON DELETE CASCADE;
ALTER TABLE tb_users DROP COLUMN IF EXISTS owner_user_id CASCADE;

ALTER TABLE tb_agents ADD CONSTRAINT tb_agents_id_identities_fk FOREIGN KEY (id) REFERENCES tb_identities(id) ON DELETE CASCADE;

-- ─── 4. Refactor tb_sessions ─────────────────────────────────────────────────
ALTER TABLE tb_sessions ADD COLUMN actor_id uuid;
ALTER TABLE tb_sessions ADD COLUMN owner_id uuid;

ALTER TABLE tb_sessions ADD CONSTRAINT tb_sessions_actor_id_fk FOREIGN KEY (actor_id) REFERENCES tb_identities(id) ON DELETE CASCADE;
ALTER TABLE tb_sessions ADD CONSTRAINT tb_sessions_owner_id_fk FOREIGN KEY (owner_id) REFERENCES tb_users(id) ON DELETE CASCADE;

-- Backfill tb_sessions:
-- Human sessions: actor_id = user_id, owner_id = user_id
UPDATE tb_sessions SET actor_id = user_id, owner_id = user_id
WHERE agent_id IS NULL;
-- Agent sessions: actor_id = agent_id, owner_id = user_id (migration repointed user_id to owner)
UPDATE tb_sessions SET actor_id = agent_id, owner_id = user_id
WHERE agent_id IS NOT NULL;

ALTER TABLE tb_sessions ALTER COLUMN actor_id SET NOT NULL;
ALTER TABLE tb_sessions ALTER COLUMN owner_id SET NOT NULL;

ALTER TABLE tb_sessions DROP COLUMN user_id CASCADE;
ALTER TABLE tb_sessions DROP COLUMN agent_id CASCADE;

-- ─── 5. Refactor tb_attempts ─────────────────────────────────────────────────
ALTER TABLE tb_attempts ADD COLUMN actor_id uuid;
ALTER TABLE tb_attempts ADD COLUMN owner_id uuid;

ALTER TABLE tb_attempts ADD CONSTRAINT tb_attempts_actor_id_fk FOREIGN KEY (actor_id) REFERENCES tb_identities(id) ON DELETE CASCADE;
ALTER TABLE tb_attempts ADD CONSTRAINT tb_attempts_owner_id_fk FOREIGN KEY (owner_id) REFERENCES tb_users(id) ON DELETE CASCADE;

-- Backfill attempts (human-only actors)
UPDATE tb_attempts SET actor_id = user_id, owner_id = user_id;

ALTER TABLE tb_attempts ALTER COLUMN actor_id SET NOT NULL;
ALTER TABLE tb_attempts ALTER COLUMN owner_id SET NOT NULL;

ALTER TABLE tb_attempts DROP COLUMN user_id CASCADE;

-- ─── 6. Refactor tb_questions ────────────────────────────────────────────────
ALTER TABLE tb_questions ADD COLUMN actor_id uuid;

ALTER TABLE tb_questions ADD CONSTRAINT tb_questions_actor_id_fk FOREIGN KEY (actor_id) REFERENCES tb_identities(id);

-- Backfill: if agent_id set, created by agent, else human
UPDATE tb_questions SET actor_id = agent_id
WHERE agent_id IS NOT NULL;
UPDATE tb_questions SET actor_id = created_by
WHERE agent_id IS NULL;

ALTER TABLE tb_questions ALTER COLUMN actor_id SET NOT NULL;

ALTER TABLE tb_questions DROP COLUMN created_by CASCADE;
ALTER TABLE tb_questions DROP COLUMN agent_id CASCADE;

ALTER TABLE tb_questions RENAME COLUMN actor_id TO created_by;

-- ─── 7. Refactor tb_assessments ──────────────────────────────────────────────
ALTER TABLE tb_assessments ADD COLUMN actor_id uuid;

ALTER TABLE tb_assessments ADD CONSTRAINT tb_assessments_actor_id_fk FOREIGN KEY (actor_id) REFERENCES tb_identities(id);

-- Backfill: if agent_id set, created by agent, else human
UPDATE tb_assessments SET actor_id = agent_id
WHERE agent_id IS NOT NULL;
UPDATE tb_assessments SET actor_id = created_by
WHERE agent_id IS NULL;

ALTER TABLE tb_assessments ALTER COLUMN actor_id SET NOT NULL;

ALTER TABLE tb_assessments DROP COLUMN created_by CASCADE;
ALTER TABLE tb_assessments DROP COLUMN agent_id CASCADE;

ALTER TABLE tb_assessments RENAME COLUMN actor_id TO created_by;

-- ─── 8. Refactor tb_activity_log ─────────────────────────────────────────────
ALTER TABLE tb_activity_log RENAME COLUMN agent_id TO actor_id;

ALTER TABLE tb_activity_log DROP CONSTRAINT IF EXISTS tb_activity_log_agent_id_fkey;
ALTER TABLE tb_activity_log ADD CONSTRAINT tb_activity_log_actor_id_fk FOREIGN KEY (actor_id) REFERENCES tb_identities(id) ON DELETE CASCADE;

-- ─── 9. Add automatic triggers to populate tb_identities on INSERT ───────────
CREATE OR REPLACE FUNCTION trigger_insert_identity_users()
RETURNS trigger AS $$
BEGIN
    INSERT INTO tb_identities (id, type)
    VALUES (NEW.id, 'human')
    ON CONFLICT (id) DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS tr_insert_identity_users ON tb_users;
CREATE TRIGGER tr_insert_identity_users
BEFORE INSERT ON tb_users
FOR EACH ROW
EXECUTE FUNCTION trigger_insert_identity_users();

CREATE OR REPLACE FUNCTION trigger_insert_identity_agents()
RETURNS trigger AS $$
BEGIN
    INSERT INTO tb_identities (id, type)
    VALUES (NEW.id, 'agent')
    ON CONFLICT (id) DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS tr_insert_identity_agents ON tb_agents;
CREATE TRIGGER tr_insert_identity_agents
BEFORE INSERT ON tb_agents
FOR EACH ROW
EXECUTE FUNCTION trigger_insert_identity_agents();
