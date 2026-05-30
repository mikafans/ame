# AI-2.1 — tb_agent_profiles table

**Phase:** 2 · **Lane:** db · **Agent:** haiku-developer · **Depends on:** AI-1.1
**Spec:** §"Schema deltas → tb_agent_profiles (new)"
**Paths (edit only here):** `db/migrations/`

## Objective
Per-agent config + memory store (focus, goal, next target, freeform jsonb).
Shared truth (level/progress) is **not** here — it stays on the owner's existing
rating/progress tables.

## Do
Migration creating `tb_agent_profiles` exactly per spec:
`agent_user_id` PK + FK→`tb_users` ON DELETE CASCADE, `label`, `focus_tags
text[]`, `current_goal`, `next_target`, `memory jsonb default '{}'`,
`created_at`, `updated_at`.

## Gotchas
- PK is `agent_user_id` (1:1 with the agent user row), not a fresh id.
- Additive; runs after AI-1.1.

## Done when
- Table applies on fresh `make db-reset`; `make check` passes.
