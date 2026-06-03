# Plan: Operational safety + Admin console (MVP launch readiness)

> Status: PLANNED. Product surface is feature-complete for a friends launch; this
> plan closes the **operate-and-watch** gap. Dispatch via
> `2026-06-03-ops-admin-mvp-tasks.jsonl`. One agent at a time (ame workflow);
> TL reviews + foreground-verifies; Haru verifies on the running stack.

## Goal

Make AME safe to run in production and operable without `curl`:

1. **Operational safety** — DB-aware readiness, Postgres backups, valkey deployed.
2. **Admin API hardening** — pagination/filters, a real health endpoint, self-protection.
3. **Admin console** — UI over the existing admin endpoints (users, audit, health).
4. **Observability** — business metrics beyond the auto HTTP layer.

## Non-goals

- No new auth model, no SSO/SAML (email+password only — unchanged).
- No new plan tiers beyond `free` / `premium`.
- No HA Postgres / multi-replica API in this pass (single-replica stays; we make
  it *recoverable*, not *highly available*).
- No general caching framework — valkey is for rate limiting only.
- No admin features doable monthly with raw SQL — this is a solo product.

## Background — current state (verified)

- **Admin backend exists** (`api/src/http/admin.rs`): `GET /v1/admin/users`
  (LIMIT 500, no pagination), `PATCH /v1/admin/users/{id}` (plan/role/disable;
  disable revokes tokens + invalidates cache; audit-logged),
  `GET /v1/admin/audit` (LIMIT 1000, no filter). Gated by `AdminScope`.
- **No admin UI** — `web/src/app/(learner)/` has no admin route; operating via curl.
- **`/healthz` is DB-blind** (`api/src/http/mod.rs:151` returns `{"status":"ok"}`)
  yet wired to *both* liveness and readiness (`deploy/k8s/base/api.yaml:57-68`).
  A pod with a dead DB stays `Ready`.
- **No Postgres backups** — `deploy/k8s/base/postgres.yaml` is a 1-replica
  StatefulSet on a 2Gi RWO PVC. No CronJob, no WAL archiving.
- **Valkey not deployed** — API depends on it (`config.rs:125` default
  `redis://127.0.0.1:6379`); no manifest in `deploy/k8s/`. Rate limiter fails
  open to in-memory (`ratelimit.rs:110`), so it's silent-degraded, not crashing.
- **Metrics**: only the auto `axum-prometheus` HTTP layer on `/metrics`
  (`mod.rs:60`). No business counters.

## Frontend anchors (for the UI tasks)

- Auth: `useAuth()` from `@/hooks/useAuth` → `user: { ..., role: string }`.
- Nav: `web/src/components/Sidebar.tsx` — `items[]` ({id,label,icon,section}),
  `sections[]`, `ICON_MAP`. Add an `"Admin"` section gated on `user.role === "admin"`.
- Route groups live under `web/src/app/(learner)/`. Add a sibling `(admin)` group
  with its own layout that redirects non-admins.
- API calls: `fetch(\`${process.env.NEXT_PUBLIC_API_URL}/v1/...\`, { credentials: "include" })`.
- UI rules (CLAUDE.md): MUI only; destructive/role changes need a MUI Dialog with
  Cancel + confirm; no `window.confirm`; no placeholder no-op buttons.

---

## Phase A — Operational safety  (branch `feat/ops-safety`)

### Task A1 — DB-aware readiness endpoint
Add `GET /readyz` that pings Postgres (`SELECT 1`) and valkey (PING), returns 200
only if both OK, else 503 with which dependency failed. Keep `/healthz` dumb
(liveness). Mount `/readyz` outside auth/RLS middleware (same as healthz).
Repoint `readinessProbe` in `api.yaml` to `/readyz`; leave `livenessProbe` on
`/healthz`. Valkey check must tolerate the in-memory fallback (report `degraded`,
still 200 if DB ok — readiness is about *serving traffic*, and the limiter
already fails open).
- Files: `api/src/http/mod.rs` (or new `api/src/http/health.rs`), `deploy/k8s/base/api.yaml`.
- Verify: `make check`; `curl :28080/readyz` returns 200 on `make dev`, 503 with DB stopped.

### Task A2 — Postgres backup CronJob + restore path
Add a k8s `CronJob` (daily) running `pg_dump` from the postgres service to a PVC
(or object storage if a bucket secret exists — start with a `backup` PVC, document
the upgrade to S3). Add `make db-backup` / `make db-restore` for local + prod-shaped
use. Write `deploy/RUNBOOK.md`: restore steps, secret rotation, admin promotion,
deploy rollback. **A backup is not done until a restore is tested** — task includes
a documented restore dry-run.
- Files created: `deploy/k8s/base/backup-cronjob.yaml`, `deploy/RUNBOOK.md`.
- Files modified: `deploy/k8s/base/kustomization.yaml`, `Makefile`.
- Verify: `kustomize build deploy/k8s/overlays/prod` succeeds; `make db-backup`
  produces a dump; documented restore into a scratch DB round-trips.

### Task A3 — Deploy valkey
Add a valkey `Deployment` + `Service` (or single-replica StatefulSet if persistence
wanted — ephemeral is fine for rate-limit counters). Wire `AME_VALKEY_URL` into the
api Deployment env pointing at the valkey service. Confirm the limiter uses valkey
(not the in-memory fallback) once deployed.
- Files created: `deploy/k8s/base/valkey.yaml`.
- Files modified: `deploy/k8s/base/kustomization.yaml`, `deploy/k8s/base/api.yaml`.
- Verify: `kustomize build deploy/k8s/overlays/prod`; after A1, `/readyz` reports
  valkey healthy (not degraded) when valkey reachable.

---

## Phase B — Admin API hardening  (branch `feat/admin-api`, depends on A1)

### Task B1 — Pagination + filters on admin lists
`GET /v1/admin/users`: add `?limit&offset&q` (q matches email/display_name),
return `{users, total}`. `GET /v1/admin/audit`: add `?limit&offset&action&actorId&targetId`,
return `{logs, total}`. Keep sane caps (limit ≤ 200). Update `api/openapi.yaml`
(or regenerate with `make openapi`).
- Files: `api/src/http/admin.rs`, `api/openapi.yaml`.
- Verify: `make check`; `make test-db` (add cases to admin integration test).

### Task B2 — Admin health/system endpoint
`GET /v1/admin/health` (AdminScope): returns DB ok, valkey ok/degraded, table
row counts (users/assessments/sessions/questions), recent quota-rejection count,
and audit-log size. Powers the UI health panel. Reuse the A1 dependency checks.
- Files: `api/src/http/admin.rs`, `api/openapi.yaml`.
- Verify: `make check`; `make test-db`.

### Task B3 — Admin self-protection guards
In `patch_user_admin`: reject an admin demoting their own role, disabling their
own account, or removing the last remaining admin. Return 422 with a field error.
Audit the rejection attempt.
- Files: `api/src/http/admin.rs`.
- Verify: `make check`; `make test-db` (add guard cases).

---

## Phase C — Admin console  (branch `feat/admin-ui`, depends on B1–B3)

### Task C1 — Admin route group + nav gating
Create `web/src/app/(admin)/layout.tsx` that redirects to `/` if
`user.role !== "admin"` (mirror the learner layout's auth gate). Add an `"Admin"`
section + entries to `Sidebar.tsx` rendered only when `user.role === "admin"`.
Add an admin landing page linking the three panels.
- Files created: `web/src/app/(admin)/layout.tsx`, `web/src/app/(admin)/admin/page.tsx`.
- Files modified: `web/src/components/Sidebar.tsx`.
- Verify: `make check`; non-admin hitting `/admin` redirects; admin sees the section.

### Task C2 — Users management page
Table backed by `GET /v1/admin/users` (server-side pagination + search). Inline
actions: change plan, change role, disable/enable — each behind a MUI Dialog
(Cancel + confirm; red confirm for disable). Calls `PATCH /v1/admin/users/{id}`.
Surface B3 guard errors inline. No placeholder buttons.
- Files created: `web/src/app/(admin)/admin/users/page.tsx`.
- Verify: `make check`; manual on `make dev`; (optional) e2e spec.

### Task C3 — Audit log viewer
Table backed by `GET /v1/admin/audit` with filters (action/actor/target) +
pagination. Read-only. Human-readable action labels.
- Files created: `web/src/app/(admin)/admin/audit/page.tsx`.
- Verify: `make check`; manual on `make dev`.

### Task C4 — System health panel
Cards from `GET /v1/admin/health`: DB/valkey status chips, row counts,
quota-rejection count, audit size. Manual refresh button.
- Files created: `web/src/app/(admin)/admin/health/page.tsx`.
- Verify: `make check`; manual on `make dev`.

---

## Phase D — Observability  (branch `feat/observability`)

### Task D1 — Business metrics
Add `metrics` counters/histograms at key points: signups, login success/fail,
session start/submit, grader latency, quota rejections, rate-limit rejections.
Expose via the existing `/metrics` recorder. Document the metric names in
`deploy/RUNBOOK.md` and add a Prometheus scrape annotation to `api.yaml`.
- Files: `api/src/...` (call sites), `deploy/k8s/base/api.yaml`, `deploy/RUNBOOK.md`.
- Verify: `make check`; `curl :28080/metrics` shows the new series after exercising flows.

---

## Sequencing & dispatch notes

- **Order:** A → B → C → D. A is independent and highest value (data-loss + false-green).
  B unblocks C. D is independent of A–C, can slot anytime after A1.
- **One agent at a time** (ame workflow); no worktrees. Use **haiku-developer** for
  implementation tasks. TL reviews each task's diff + foreground-verifies before
  the commit gate (`make check`); Haru does final verification on the running stack.
- Each task is its own commit (conventional commits). Push/PR only when Haru asks.
