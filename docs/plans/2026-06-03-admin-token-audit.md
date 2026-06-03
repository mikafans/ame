# Admin Console — Feature 1.1: Global API Key & Agent Token Audit

**Status:** planned · **Branch:** `feat/admin-token-audit` · **Depends on:** `feat/admin-api` (admin shell, layout, audit helper)

## Goal

Give admins one place to see **every** API token on the platform — human and
agent — with enough context to spot abuse and to **revoke any token globally**,
without going through the owning user's dashboard.

This is the 1.1 (MVP) slice: list + filter + global revoke. Per-token traffic
graphs and rate analytics are explicitly deferred to Phase D (observability).

## Background (verified against code)

- Tokens live in `tb_api_tokens` (`db/migrations/20260602000000_baseline.sql:60`):
  `id, user_id → tb_users ON DELETE CASCADE, name, token_hash, scopes text[],
  last_used_at, revoked_at, expires_at, created_at`. Partial index
  `tb_api_tokens_user_active` on `(user_id) WHERE revoked_at IS NULL`.
- **Agents are users**: `tb_users.role IN ('user','admin','agent')`, agent rows
  carry `owner_user_id` (creator) and a `tb_agent_profiles` row (label, focus_tags,
  goal). Agents call the API with ordinary `tb_api_tokens`. So "agent token audit"
  is just the `role='agent'` slice of the same table — no separate store.
- `last_used_at` is maintained by the auth extractor on each authenticated call.
- No RLS on `tb_api_tokens` (RLS is `tb_questions`-only), so admin-pool queries
  over `state.pool` are correctly unscoped — mirror `list_users` /
  `list_assessments_admin` in `api/src/http/admin.rs`.
- User-facing precedent for revoke: `me.rs:295 revoke_key` →
  `UPDATE tb_api_tokens SET revoked_at = now() WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL`.

**No migration required.** The table already has every column we need.

## Non-goals (defer to Phase D / later)

- Per-token request counts, traffic/QPS charts, last-IP — needs the metrics
  pipeline. 1.1 shows only `last_used_at` + `created_at` + `expires_at`.
- Editing token scopes from the admin console (revoke-and-reissue is the model).
- Bulk revoke / "revoke all for user X" — single-token revoke only in 1.1.
- Showing or recovering the token secret. `token_hash` is **never** serialized.

## Backend

### 1. `GET /v1/admin/tokens` — list all tokens (paginated, filtered)

Mirror `list_assessments_admin` (QueryBuilder, count + page query, `state.pool`).

Query params (all optional):
- `limit` (default 25, cap 100), `offset` — same clamping as `list_users`.
- `q` — ILIKE over `t.name`, owner `u.email`, owner `u.display_name`.
- `status` — `active | revoked | expired`. Computed, not a column:
  - `revoked`  → `t.revoked_at IS NOT NULL`
  - `expired`  → `t.revoked_at IS NULL AND t.expires_at <= now()`
  - `active`   → `t.revoked_at IS NULL AND t.expires_at > now()`
- `role` — filter owner role (`user | admin | agent`); enables the "agent tokens" view.
- `owner_id` (uuid) — all tokens for one user/agent.

Response item (`AdminTokenEntry`, camelCase via existing serde config):
`id, name, ownerId (user_id), ownerEmail, ownerDisplayName, ownerRole, scopes,
status (active|revoked|expired, server-computed), lastUsedAt, expiresAt,
revokedAt, createdAt`. **`token_hash` is not selected.**

`SELECT t.id, t.name, t.user_id, u.email, u.display_name, u.role, t.scopes,
t.last_used_at, t.expires_at, t.revoked_at, t.created_at
FROM tb_api_tokens t JOIN tb_users u ON u.id = t.user_id`
+ WHERE clauses + `ORDER BY t.created_at DESC LIMIT/OFFSET`. Count query shares
the same WHERE construction (keep list/count in lockstep, like the assessments
endpoint).

### 2. `DELETE /v1/admin/tokens/{id}` — global revoke

```
UPDATE tb_api_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL
```
- `rows_affected == 0`: do a follow-up `SELECT 1` existence check.
  - exists (already revoked) → `204` (idempotent, no-op)
  - not found → `404 { resource: "token" }`
- On success, audit: `audit(pool, Some(admin.id), "token.revoke_admin",
  Some("token"), Some(id), json!({ "owner_id": ..., "name": ... }))`
  (fetch owner_id/name in the same handler for the audit payload).
- Admin-guarded via the existing admin extractor (`admin.0.user`).

> No "restore" for tokens — revocation is intentionally terminal (security
> posture). A user simply issues a new key. This differs from assessment
> soft-delete on purpose; call it out in the dialog copy.

### 3. Wiring

- Register both routes in the admin router block in `admin.rs`
  (`.route("/v1/admin/tokens", get(list_tokens_admin))`,
  `.route("/v1/admin/tokens/{id}", delete(revoke_token_admin))`).
- Add utoipa annotations + register in `api/src/http/openapi.rs`; run
  `make openapi` and commit the regenerated `web/src/api/generated/schema.d.ts`.

## Frontend

New page `web/app/(admin)/admin/tokens/page.tsx`, cloned from the assessments
admin page (it already has the search + filter + pagination + confirm-dialog
shape we want):
- Columns: Name · Owner (email + role chip; `AGENT` chip distinct color) ·
  Scopes (chips, truncated) · Status (active=success / revoked=default /
  expired=warning) · Last used · Created.
- Filters: search box; status toggle (All/Active/Revoked/Expired); role select
  (All/User/Agent/Admin). Reset page→0 on filter change (same handlers).
- Action: **Revoke** IconButton (error) on non-revoked rows only; revoked rows
  show no action. Confirm dialog with a checkbox, copy stating revocation is
  **permanent and immediate** ("the holder will be signed out on next request;
  issue a new key to restore access"). MUI Dialog per project rule — no
  `window.confirm`.
- Add a sidebar/nav entry "API Tokens" in the admin layout next to
  Users / Assessments / Audit / Health.

## Tests (`api/tests/admin.rs`)

- `test_admin_tokens_list_and_filters`: seed tokens for a user and an agent
  (active, revoked, expired); assert `GET /v1/admin/tokens` returns them with
  correct computed `status`, owner email/role join populated, and **no
  `token_hash`/secret field** in the JSON. Assert `role=agent`, `status=revoked`,
  and `owner_id` filters each narrow correctly; assert `total` tracks filters.
- `test_admin_token_revoke`: revoke an active token → `204`, row has
  `revoked_at`, an audit row `token.revoke_admin` exists; revoke again → `204`
  (idempotent); revoke a random uuid → `404`.
- `test_admin_tokens_requires_admin`: non-admin token → `403` on both endpoints.

## Verification

- `make check` (sole command; read the log file directly — rtk garbles tails).
- Manual on running stack: create an agent + key as a normal user, confirm it
  appears under `/admin/tokens` with role=agent, revoke it, confirm the agent's
  next API call 401s.

## Dispatch (task.jsonl line)

```json
{"id":14,"title":"Admin global API token audit + revoke (1.1)","branch":"feat/admin-token-audit","plan":"docs/plans/2026-06-03-admin-token-audit.md","summary":"Add GET /v1/admin/tokens (paginated, filter by q/status/role/owner_id, never expose token_hash) and DELETE /v1/admin/tokens/{id} (global revoke, idempotent, audited as token.revoke_admin). Add admin /admin/tokens UI cloned from the assessments admin page with status+role filters and a revoke confirm dialog. Register routes + utoipa, regenerate schema.d.ts via make openapi.","files_created":["web/app/(admin)/admin/tokens/page.tsx"],"files_modified":["api/src/http/admin.rs","api/src/http/openapi.rs","api/openapi.yaml","web/src/api/generated/schema.d.ts","web/app/(admin)/layout.tsx","api/tests/admin.rs"],"files_deleted":[],"verify":"make check > /tmp/admin-tokens-check.log 2>&1 (run as the SOLE command; then read the log file)","commit_message":"feat(admin): global API token audit + revoke"}
```
