# AI-6.1 — tb_audit_log table

**Phase:** 6 · **Lane:** db · **Agent:** haiku-developer · **Depends on:** AI-1.1
**Spec:** §"Schema deltas → tb_audit_log (new)"
**Paths (edit only here):** `db/migrations/`

## Objective
Append-only audit trail for sensitive actions (public-publish, export, moderate,
plan changes).

## Do
Migration creating `tb_audit_log` exactly per spec: `id` (uuid v7 default),
`actor_user_id` FK→tb_users ON DELETE SET NULL, `action`, `target_type`,
`target_id`, `metadata jsonb default '{}'`, `created_at`, plus
`idx_audit_log_created (created_at DESC)`.

## Gotchas
- `actor_user_id` is `ON DELETE SET NULL` (keep the log if the user is deleted),
  not CASCADE.
- Confirm the v7 default function name used elsewhere in the schema
  (`uuid_generate_v7` vs app-side `Uuid::now_v7()`); match what already works.

## Done when
- Table + index apply on fresh `make db-reset`; `make check` passes.
