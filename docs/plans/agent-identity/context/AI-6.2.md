# AI-6.2 — Admin routes + audit log

**Phase:** 6 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-6.1, AI-4.2
**Spec:** §"API surface → Admin" + §"Authorization"
**Paths (edit only here):** `api/src/http/admin.rs`, `api/src/audit.rs` (new), `api/openapi.yaml`

## Objective
Operator panel API + the audit write helper the rest of the system calls.

## Do
- `api/src/audit.rs`: `audit(actor, action, target_type, target_id, metadata)`
  inserting into `tb_audit_log`. Small, sync-path, reusable.
- `api/src/http/admin.rs` (extend existing module), all `role = admin` gated:
  - `GET /v1/admin/users`, `PATCH /v1/admin/users/{id}` (plan toggle, disable),
  - `GET /v1/admin/audit`,
  - `POST /v1/admin/moderate` (unpublish public content → set visibility private).
- Wire `audit(...)` into the public-publish path (AI-3.2), export (AI-5.1), and
  moderate. Those tasks left TODO-hooks; fill them.

## Read first
- `api/src/http/admin.rs` (current) for the existing admin router + scope guard.

## Gotchas
- Premium toggle is **manual** here — no payment integration.
- Moderation must also write an audit row; that's the point of the trail.

## Done when
- Admin routes work + gated to admin; audit helper wired into the 3 call sites;
  `make openapi` updated; `make check` passes.
