# AI-1.1 — owner_user_id + agent_shape constraint

**Phase:** 1 · **Lane:** db · **Agent:** haiku-developer · **Depends on:** AI-0.1
**Spec:** §"Schema deltas → tb_users" (owner_user_id + tb_users_agent_shape)
**Paths (edit only here):** `db/migrations/`

## Objective
Make an agent a sub-account of its owner, enforced at the DB level: agents are
token-only (no email/password) and exactly one level deep (cannot own agents).

## Do
A new migration that adds:
```sql
ALTER TABLE tb_users ADD COLUMN owner_user_id uuid
  REFERENCES tb_users(id) ON DELETE CASCADE;
ALTER TABLE tb_users ADD CONSTRAINT tb_users_agent_shape CHECK (
  (role = 'agent' AND owner_user_id IS NOT NULL AND email IS NULL AND password_hash IS NULL)
  OR (role <> 'agent' AND owner_user_id IS NULL)
);
```

## Read first
- The latest `tb_users` migration to confirm the real column names
  (`password_hash` vs other) before writing the CHECK — match reality.

## Gotchas
- Do **not** add `plan` here (that's AI-4.1) or `tb_agent_profiles` (AI-2.1).
- Additive only; runs after AI-0.1's role values exist (the CHECK references
  `role='agent'`).

## Done when
- Column + constraint apply on a fresh `make db-reset`; `make check` passes.
