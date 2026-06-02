# Owner isolation via schema redesign + Row-Level Security

Status: PROPOSED — 2026-06-02
Context: there is **no shipped baseline / no real data**. The single
`20260601000000_baseline.sql` migration can be edited in place and applied with
`make db-reset`. So we redesign the schema to make ownership first-class rather
than layering RLS onto an awkward shape.

Goal: owner isolation enforced in Postgres (can't-forget), as defense-in-depth
over the application-layer scoping already in place.

## Core design decision: denormalized `owner_id` everywhere

Add `owner_id uuid NOT NULL REFERENCES tb_users(id)` to **every owner-scoped
table**, where `owner_id` is the **owner-tree root** (the top-level account, not
a sub-account/agent). Rows created by an agent sub-account still store the root
owner id.

Consequence: every RLS policy is the same single, index-friendly clause —

```sql
USING       (owner_id = current_setting('app.owner')::uuid)
WITH CHECK  (owner_id = current_setting('app.owner')::uuid)
```

No `fn_is_mine` recursion, no child-via-parent joins, no special-casing
`tb_cohorts`, no dual-policy on `tb_attempts`. `CREATE INDEX ... (owner_id)` on
each table keeps reads cheap.

How `owner_id` is set: the application sets it explicitly = `auth.owner_id` on
insert (single source of truth in the auth layer). A `BEFORE INSERT` trigger is
the fallback/guard if we want defense against a forgotten column.

## Tables

Add `owner_id` (+ index, + RLS) to:
`tb_questions`, `tb_assessments`, `tb_assessment_sections`,
`tb_assessment_items`, `tb_question_versions`, `tb_question_tags`,
`tb_attempts`, `tb_sessions`, `tb_study_plans`, `tb_user_tag_ratings`,
`tb_api_tokens`, `tb_webhooks`, `tb_webhook_deliveries`, `tb_idempotency_keys`,
`tb_cohorts`, `tb_cohort_memberships`, `tb_agent_profiles`, `tb_messages`,
`tb_activity_log`.

Special:
- `tb_users` — RLS uses existing `owner_user_id`: `USING (id = app.owner OR owner_user_id = app.owner)`; admin bypass for listing.
- `tb_audit_log` — `owner_id` nullable (system events); visible to row owner OR admin bypass.

Global, no RLS: `tb_tags`, `tb_level_mappings`.

`tb_messages` cross-owner DMs: messages are between users in the **same** owner
tree, so a single `owner_id` is correct; if cross-tree messaging is ever needed
it becomes the documented exception.

## Foundational plumbing (unchanged by the redesign — still the dominant cost)

1. **Non-superuser app role.** App connects as superuser `postgres` today, which
   bypasses RLS. Baseline creates `ame_app` (LOGIN, non-superuser) + grants;
   API uses `ame_app`; migrations / `db-admin` / `seed.py` stay `postgres`.
2. **Request-scoped connection + GUC.** Handlers/`bank::*` currently query the
   shared `&PgPool` directly. For RLS each request must run on a connection with
   `app.owner` set (`SET LOCAL` in a txn, or `set_config(...,false)` +
   `after_release` `DISCARD ALL`). **Every handler and repo fn that takes
   `&PgPool` must move to the request connection — this touches ~all queries and
   is the largest part of the work, independent of the schema redesign.**
   Also set `app.is_admin` for admin bypass.

## Staging

- **Stage 0** — baseline: add `ame_app` role + grants; Rust: request-scoped conn
  extractor + GUC plumbing; NO policies yet. `make db-reset`, stack green.
- **Stage 1** — baseline: add `owner_id` columns + indexes; app sets `owner_id`
  on every insert (+ optional guard trigger). Still no policies; verify writes.
- **Stage 2** — baseline: `ENABLE` + `FORCE ROW LEVEL SECURITY` + the uniform
  policy on every owned table; `tb_users`/`tb_audit_log` variants; admin bypass.
- **Stage 3** — tests: extend `api/tests/cross_owner.rs` into a sweep across all
  endpoints; `AME_RUN_DB_TESTS=1`; Haru verifies on the stack.

Because it is a clean baseline, stages 1–2 are edits to one migration file, not
incremental migrations. Stage 0 is the risky/large one (the Rust refactor).

## Risk / rollout

- Migrations + `seed.py` run as `postgres` (RLS-exempt) — seeding unaffected;
  seed must populate `owner_id` since it inserts via the API (which sets it) or
  via direct SQL (must set it explicitly).
- Missing `app.owner` ⇒ protected queries return empty (fail-closed) — safe, and
  loud in dev if Stage 0 regresses.
- `make check` + DB integration tests gate each stage; pre-commit hook enforces.
