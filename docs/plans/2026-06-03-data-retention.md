# Data Retention & Pruning — keep Postgres slim (pre-v1.0)

**Status:** planned · **Branch:** `feat/data-retention` · **Lands:** v0.3, enforced by v1.0

## Goal

Bound the growth of append-only / transient tables so PG stays small and fast,
**without** touching the learning data that analytics (v1.0) and users depend on.
Retention windows are configurable; deletion runs unattended on a schedule and is
safe to run repeatedly.

## Table-by-table (verified against `db/migrations/`)

### Prune — transient / replaceable (highest value)

| Table | Grows on | Default retention | Notes |
|-------|----------|-------------------|-------|
| `tb_idempotency_keys` | every idempotent write | **48h** since `created_at` | **No cleanup exists today** — pure leak. Only matters within a client retry window. Indexed on `created_at`. Safest first win. |
| `tb_activity_log` | **every** authenticated agent API call (middleware) | **90d** since `ts` | Fastest grower. ⚠ see *grade-count dependency* below before deleting. |
| `tb_webhook_deliveries` | every delivery attempt | **30d** for terminal rows (`status IN ('delivered','failed')`) | Never delete `status='pending'`. |
| `tb_messages` | every notification | **90d** for `status='read'` | Keep unread/queued forever. |

### Prune — terminal lifecycle rows (lower volume)

| Table | Condition | Default retention |
|-------|-----------|-------------------|
| `tb_api_tokens` | `revoked_at` or `expires_at` in the past | **90d** past that timestamp. CASCADEs to `tb_idempotency_keys` (fine). |
| `tb_assessments` | `deleted_at IS NOT NULL` (admin soft-delete) | **90d** → finally does the real cascade delete (sections, items, sessions). This is the back half of the v0.1 soft-delete feature. |

### Compliance — long window then purge

| Table | Retention |
|-------|-----------|
| `tb_audit_log` | **365d** (regulatory/forensic), then delete. Consider archiving to the backup bucket before purge rather than dropping outright. |

### Never auto-prune (business / analytics data)

`tb_attempts`, `tb_sessions`, `tb_user_tag_ratings`, `tb_questions`(+versions),
`tb_assessment_*` (non-deleted), `tb_study_plans`, `tb_users`. These feed v1.0
analytics and user history. If they ever get heavy, the answer is
**partitioning**, not deletion — out of scope here.

## ⚠ Critical dependency to resolve first: activity-log grade counts

`tb_activity_log` carries a partial index
`tb_activity_log_grade_count ON (agent_id) WHERE tool_name='attempt.grade' AND status=200`
— i.e. **lifetime agent grade totals are computed by counting activity_log rows.**
Pruning old activity would silently shrink those totals.

Resolve one of these before enabling activity-log pruning:
1. **Roll up before delete (preferred):** maintain a counter (e.g. on
   `tb_agent_profiles` or a small `tb_agent_counters`) incremented at prune time
   by the number of grade rows about to be deleted; read = counter + live count.
2. **Exempt grade rows:** only prune `tool_name != 'attempt.grade'` (keeps the
   counted rows; sheds the bulk of noise — most calls aren't grades).
3. **Accept windowed counts:** redefine the metric as "last 90d" and document it.

Pick this with the metrics owner (Phase D / Gemini) since analytics reads the
same data. Until resolved, ship pruning for the other tables and leave
`tb_activity_log` at a generous window or grade-exempt.

## Mechanism

Mirror the existing `deploy/k8s/base/backup-cronjob.yaml` pattern.

- **`fn_prune_stale_data(dry_run boolean)`** SQL function (new migration). Returns
  a row per table with `would_delete`/`deleted` counts. Encapsulates every cutoff
  so the policy lives in one auditable place.
- **Batched deletes** to avoid long locks on big tables: delete in chunks
  (`DELETE ... WHERE ctid IN (SELECT ctid ... LIMIT 10000)`) looping until 0 rows.
- **CronJob** `data-retention-cronjob.yaml` runs daily (off-peak), calls the
  function via `psql`, logs the per-table counts.
- **Retention windows are config**, not hardcoded — env vars now, promoted to the
  v0.2 **settings/feature-flags** store later (natural synergy with feature #2).
- **Audit + observability:** write one `tb_audit_log` row per run
  (`action='data.prune'`, metadata = per-table counts) so operators see it in the
  existing audit UI. Optionally surface "last prune run + rows freed" on the admin
  health page.
- **Dry-run first:** CronJob ships in `dry_run=true` mode initially; flip to live
  after one cycle confirms sane counts.

## Out of scope

- Table partitioning / `pg_partman` (future, for `tb_attempts` at real scale).
- `VACUUM FULL` / physical reclaim — autovacuum + routine deletes are enough at
  v1.0 scale; document a manual `pg_repack` runbook entry instead.
- Purging any learning/analytics data.

## Tests

- `fn_prune_stale_data(true)` (dry-run) reports correct counts and deletes
  **nothing**.
- Seed rows straddling each cutoff; `fn_prune_stale_data(false)` deletes only the
  stale side; idempotent on a second run (0 deleted).
- Soft-deleted assessment past window → fully cascade-removed (sections/items/
  sessions gone); within window → untouched.
- Activity grade-count invariant holds under the chosen mitigation (counter or
  exemption) — counts don't drop after a prune.

## Verification

`make check` (sole command; read the log file directly). Manual: seed an aged
`tb_idempotency_keys` / `tb_activity_log` set, run the function, confirm size drop
via `pg_total_relation_size`.

## Dispatch (task.jsonl line)

```json
{"id":15,"title":"Data retention & pruning (pre-v1.0)","branch":"feat/data-retention","plan":"docs/plans/2026-06-03-data-retention.md","summary":"Add fn_prune_stale_data(dry_run) SQL function with batched deletes for tb_idempotency_keys (48h), tb_webhook_deliveries terminal (30d), tb_messages read (90d), expired/revoked tb_api_tokens (90d), soft-deleted tb_assessments (90d cascade), tb_audit_log (365d). Resolve the activity_log grade-count dependency before pruning tb_activity_log. Add a daily k8s CronJob (mirrors backup-cronjob.yaml) starting in dry-run, env-configurable windows, one audit row per run.","files_created":["db/migrations/<ts>_prune_stale_data.sql","deploy/k8s/base/data-retention-cronjob.yaml"],"files_modified":["deploy/k8s/base/kustomization.yaml","deploy/RUNBOOK.md","api/tests/<retention test>"],"files_deleted":[],"verify":"make check > /tmp/retention-check.log 2>&1 (run as the SOLE command; then read the log file)","commit_message":"feat(ops): scheduled stale-data pruning to bound PG growth"}
```
