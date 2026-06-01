# AI-4.1 — tb_users.plan column

**Phase:** 4 · **Lane:** db · **Agent:** haiku-developer · **Depends on:** AI-1.1
**Spec:** §"Schema deltas → tb_users" (plan column) + §"Tiers"
**Paths (edit only here):** `db/migrations/`

## Objective
Add the tier column on human accounts. Agents inherit the owner's plan via
`owner_user_id` (no column on agents needed).

## Do
```sql
ALTER TABLE tb_users ADD COLUMN plan text NOT NULL DEFAULT 'free'
  CHECK (plan IN ('free','premium'));
```

## Gotchas
- `plan` is only meaningful on humans; agents resolve it through their owner at
  read time (AI-4.2). Don't try to enforce that in SQL.
- Additive; runs after AI-1.1.

## Done when
- Column applies on fresh `make db-reset`; `make check` passes.
