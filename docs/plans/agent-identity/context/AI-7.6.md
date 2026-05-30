# AI-7.6 — Admin UI (users, audit, moderate)

**Phase:** 7 · **Lane:** web · **Agent:** sonnet · **Depends on:** AI-6.2
**Spec:** §"API surface → Admin"
**Paths (edit only here):** `web/app/admin/`

## Objective
Operator panel for the single admin user.

## Do
- `/admin` (admin-gated route): user list with plan toggle + disable
  (`PATCH /v1/admin/users/{id}`), an audit log view (`GET /v1/admin/audit`),
  and a moderate action (`POST /v1/admin/moderate`) to unpublish public content.
- Moderate + disable are destructive → MUI Dialog (Cancel + red confirm).

## Gotchas
- Route-gate to `role = admin`; non-admins never see it in nav and get bounced.
- All MUI; no `window.confirm`; no placeholder buttons.
- Premium toggle is manual — it's just a switch calling the admin endpoint.

## Done when
- Admin can list/toggle/disable users, view audit, and unpublish content (behind
  dialogs); non-admins are blocked; `make e2e` passes.
