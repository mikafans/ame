# Implementation Plan: Identity Unification & Ergonomic Agent Surface

**Status:** Planned · **Branch:** `feat/identity-unification` · **Author:** Antigravity

## Goal

Resolve the cognitive burden of the parallel human (`tb_users`) and agent (`tb_agents`) schema. We want a unified architecture where human users and AI agents are sub-types of a polymorphic `tb_identities` parent entity. This simplifies stats, attempts, and audit queries, aligns agent tokens directly with standard REST endpoints, and removes the parallel router overhead.

---

## 1. Database Schema Refactor (`tb_identities`)

We will create a database migration `db/migrations/20260614010000_identity_unification.sql`:

### A. Create the Polymorphic Parent Table
```sql
CREATE TABLE tb_identities (
  id         uuid        PRIMARY KEY DEFAULT uuid_generate_v7(),
  type       text        NOT NULL CHECK (type IN ('human', 'agent')),
  created_at timestamptz NOT NULL DEFAULT now()
);
```

### B. Migrate and Repoint the Primary Identity Tables
1. **Backfill Identities**:
   Insert all existing users and agents into `tb_identities`:
   ```sql
   INSERT INTO tb_identities (id, type, created_at)
   SELECT id, 'human', created_at FROM tb_users;

   INSERT INTO tb_identities (id, type, created_at)
   SELECT id, 'agent', created_at FROM tb_agents;
   ```
2. **Alter `tb_users`**:
   * Add a foreign key constraint on `id` referencing `tb_identities(id)`.
   * Drop the `owner_user_id` column (since only agents are owned).
3. **Alter `tb_agents`**:
   * Add a foreign key constraint on `id` referencing `tb_identities(id)`.
   * Repoint `owner_user_id` to reference `tb_users(id)` directly.

### C. Consolidate Attribution Columns (Polymorphic Actors)
Update target tables to use `actor_id` (who did it, referencing `tb_identities`) and `owner_id` (whose record it is, referencing `tb_users`):

1. **`tb_sessions`**:
   * Add `actor_id` (references `tb_identities`) and `owner_id` (references `tb_users`).
   * Backfill:
     * For human-taken sessions: `actor_id = user_id`, `owner_id = user_id`.
     * For agent-taken sessions: `actor_id = agent_id`, `owner_id = user_id` (since migration repointed `user_id` to owner).
   * Drop old columns: `user_id`, `agent_id`.
   * Make `actor_id` and `owner_id` `NOT NULL`.

2. **`tb_attempts`**:
   * Add `actor_id` and `owner_id`.
   * Backfill: `actor_id = user_id`, `owner_id = user_id`.
   * Drop `user_id`. Make both new columns `NOT NULL`.

3. **`tb_questions`**:
   * Add `created_by` (references `tb_identities`) and `owner_id` (references `tb_users`).
   * Backfill:
     * If created by agent (`agent_id IS NOT NULL`): `created_by = agent_id`, `owner_id = owner_id`.
     * If created by human: `created_by = created_by`, `owner_id = owner_id`.
   * Drop old columns: `agent_id`. Make both `created_by` and `owner_id` `NOT NULL`.

4. **`tb_assessments`**:
   * Similar to questions, repoint `created_by` (references `tb_identities`) and `owner_id` (references `tb_users`). Drop `agent_id`.

5. **`tb_activity_log`**:
   * Rename `agent_id` to `actor_id` and repoint foreign key to `tb_identities(id)`. This allows logging both human and agent system actions uniformly.

---

## 2. Backend Model & Context Refactor (`api/src/`)

### A. Authentication & Actor Identification (`api/src/auth/`)
Update the authenticated request context in `extractor.rs`:
* Define a clear `Actor` enum:
  ```rust
  pub enum ActorKind {
      Human,
      Agent { owner_id: Uuid },
  }
  ```
* Modify `AuthenticatedUser` to carry:
  * `id`: The actor's UUID (human or agent).
  * `owner_id`: The human owner's UUID (equals `id` for human, equals `owner_user_id` for agent).
  * `kind`: `ActorKind`.
  * `token_scopes`: Allowed scopes for this caller.

### B. Standardize REST Routes (Removing the Single-Door Barrier)
* Update the routing layer in `api/src/http/mod.rs` to allow agent bearer tokens directly on REST endpoints:
  * Remove the path checks that throw `403` for agent tokens on REST GET/POST routes.
  * Agent tokens are authorized solely based on **scopes** (`assessment.read`, `attempt.write`, etc.) and automatically scoped to `auth.owner_id` internally, preventing cross-tenant access.
* Keep `POST /v1/agents/run` purely as a legacy wrapper / tool-calling bridge if needed, but under the hood, it simply calls the standard REST handlers.

### C. Simplify Backend Database Queries
With the database schema simplified, all queries become clean and uniform:
* **Human Stats**: Query `tb_attempts WHERE owner_id = $1` and `tb_sessions WHERE owner_id = $1`.
* **Agent-Specific Actions**: Query `tb_sessions WHERE actor_id = $1` or filter `WHERE actor_id != owner_id`.
* This completely avoids complicated subqueries and resolves the progress page dashboard data issues elegantly.

---

## 3. Tolerant Option Input Parsing

Ensure question creation (`question.create` / `POST /v1/questions`) is robust:
* Update `Option` deserialization in `graders.rs` to support:
  * Bare strings: `["let", "fn", "mod"]`
  * Object forms: `[{"text": "let"}, {"text": "fn"}]`
* Normalize to the internal `Vec<String>` representation during JSON deserialization to prevent schema validation crashes.

---

## 4. Test Migration Strategy

Following rule 2 (*"Refactors are test-first"*), we will:
1. Update integration tests in `api/tests/` to use the new `actor_id` / `owner_id` DB layout.
2. Ensure mock agents are created with a corresponding row in `tb_identities` and `tb_agents` without needing a fake shadow `tb_users` record.
3. Verify all endpoints behave identically under direct agent REST calls.
