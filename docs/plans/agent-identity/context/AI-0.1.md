# AI-0.1 — Role-collapse migration

**Phase:** 0 · **Lane:** db · **Agent:** haiku-developer · **Depends on:** —
**Spec:** §"Schema deltas → tb_users" (the role-migration block)
**Paths (edit only here):** `db/migrations/`

## Objective
Collapse the `role` domain from `{learner, instructor, admin, agent}` to
`{user, admin, agent}` at the DB level. This is the first task of the whole
refactor; AI-0.2 (the Rust `Role` enum) lands right after and must match.

## Read first
- The latest migration defining `tb_users.role` and its CHECK constraint
  (search `db/migrations/` for `role`). Match its constraint name exactly.
- Spec schema-deltas: only the role part. Do **not** add `owner_user_id` or
  `plan` here — those are AI-1.1 and AI-4.1.

## Do
A new timestamped migration that:
1. `UPDATE tb_users SET role='user' WHERE role IN ('learner','instructor');`
2. drops the old role CHECK constraint, adds
   `CHECK (role IN ('user','admin','agent'))`.

## Gotchas
- Migration filename = `YYYYMMDDHHMMSS_<slug>.sql`, after the latest existing one.
- If sqlfluff reformats it post-apply and checksums mismatch, the fix is
  `make db-reset` then `make dev` — never hand-edit the applied row.
- Keep it forward-only; this is dev-stage, no down-migration needed.

## Done when
- `make check` passes (migration applies on a fresh `make db-reset`).
- A user previously `instructor`/`learner` is now `user`.
