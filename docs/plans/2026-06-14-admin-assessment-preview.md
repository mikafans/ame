# Implementation Plan: Admin Assessment Preview (Preview-before-Moderate)

**Status:** Planned · **Target release:** v0.2.0 · **Branch:** `release/v0.2.0`

## Goal

Give an admin a way to **inspect an assessment's actual content before a destructive
moderation action**. Today `/admin/assessments` is metadata-only (title, description,
objectives, status, creator) yet its own copy instructs "Delete problematic content if
necessary" — the moderator has to act blind.

**Kill-gate** (*what breaks for a real user without this?*): a moderator acting on a
report cannot confirm the content is actually problematic. That produces two real
failure modes — **wrongful deletion** (nuking a fine assessment off a misleading title)
and **missed violations** (abusive content hiding under an innocuous title). For a
moderation surface, "act blind" is a genuine break. This passes the gate.

## Scope

**In:**
- New admin-scoped read endpoint returning one assessment + its ordered questions
  **including answer content** (moderation must see what learners cannot), resolving
  **any** creator's assessment and **including soft-deleted** ones (so a deleted item
  can be reviewed before restore).
- Frontend preview **Drawer** on `/admin/assessments`, opened by an eye icon, with
  Delete/Restore reachable from inside it (review → decide).
- Question count surfaced in the drawer header (cheap, rides along).

**Out (kill-gate fails — do not build):**
- Bulk delete / multi-select — moderation is low-volume and individually judged.
- Admin inline edit / re-authoring — admins moderate, they don't co-author; keep the
  role boundary.
- Search debounce / live filtering — current submit-on-enter is fine.

Creator deep-link (truncated ID → `/admin/users`) is a nice-to-have; include only if
trivial, otherwise defer.

---

## 1. Backend — `GET /v1/admin/assessments/{id}`

New handler in `api/src/http/admin/assessments.rs`, registered in
`api/src/http/admin/mod.rs` alongside the existing three admin-assessment routes.

- **Auth:** `RequireScope<AdminScope>` (same as `list_assessments_admin`).
- **Resolution:** by `id` only — **no `created_by` filter** and **no `deleted_at IS NULL`
  filter** (admin sees everyone's, including soft-deleted). 404 if the id doesn't exist.
- **Response body** (`camelCase`, `ToSchema`): assessment metadata (mirror
  `AdminAssessmentEntry`: id, title, description, status, mode, createdBy,
  createdByEmail, objectives, createdAt, deletedAt) **plus** `questions: [...]` and, if
  the assessment is sectioned, `sections: [...]`.
- **Question shape:** mirror the owner-scoped detail assembly in
  `api/src/http/assessments.rs` (the query that selects `payload`, `order_index`) — emit
  `id, kind, prompt, points, orderIndex, payload`. **Keep `payload` (the full answer/
  options/exemplar)** — this is the moderation differentiator vs. the learner preview,
  which deliberately strips answers. Remember MC options are bare `Vec<String>` and
  `correct_index` lives in `payload` (per the data-model constraints).
- **No audit on read** (audit stays on the mutating delete/restore paths). Optional:
  emit an `assessment.preview_admin` audit row if we want a moderation-view trail — defer
  unless asked.
- Reuse the `COALESCE(u.email, o.email, ag.label)` creator-email join from the list
  handler so a deleted creator/agent still renders.

## 2. OpenAPI

Add the `#[utoipa::path(get, ...)]` annotation on the new handler, then `make openapi`
to regenerate `api/openapi.yaml`. The schema-drift gate must pass.

## 3. Frontend — preview drawer on `/admin/assessments`

`web/app/(admin)/admin/assessments/page.tsx`:
- Add an **eye `IconButton`** (`VisibilityOutlined`) in the Actions column, left of the
  delete/restore icon, on every row (deleted rows too).
- Open a right-side MUI **`Drawer`** that fetches `GET /v1/admin/assessments/{id}` and
  renders: header (title, mode/status chips, **question count + total points**,
  DELETED chip if soft-deleted, creator), objectives, and the question list.
- **Extract the learner preview's question-row rendering** into a shared component
  (e.g. `web/components/QuestionList.tsx`) so both the learner preview and the admin
  drawer use it. The admin variant is **answer-revealing** (render `payload`: MC options
  with the correct one marked, T/F answer, short/essay/code exemplar); the learner
  variant stays prompt-only. Gate via a prop (`revealAnswers?: boolean`).
- Move **Delete / Restore** actions into the drawer footer (in addition to the row icon),
  so the natural flow is open → review → decide. Reuse the existing confirm dialogs.
- All MUI; destructive confirm stays a Dialog with Cancel + red action (UI conventions).

## 4. Tests (test-first — pin the contract before building)

Per working-discipline rule 2, write these **first** and migrate until green.

**API integration** (`api/tests/admin.rs`, gated on `AME_RUN_DB_TESTS`):
- Admin `GET /v1/admin/assessments/{id}` returns 200 with `questions` populated
  including `payload` (assert the correct answer is present).
- Admin can read an assessment owned by **a different creator** (200, not 404/403).
- Admin can read a **soft-deleted** assessment (200, `deletedAt` non-null).
- Non-admin token → **403**.
- Unknown id → **404**.

**E2E** (`web/e2e/admin.spec.ts`):
- As admin, open `/admin/assessments`, click the eye icon on a row → drawer opens and
  shows ≥1 question prompt.
- Delete from inside the drawer → row reflects DELETED after the confirm.

## 5. CHANGELOG

On implementation, add under `## [0.2.0]` → `### Added`:

```
- Added an admin assessment preview drawer so moderators can inspect an
  assessment's questions (answers included) before deleting or restoring it,
  backed by a new admin-scoped `GET /v1/admin/assessments/{id}` endpoint.
```

---

## Task breakdown (smallest → biggest)

1. **API contract tests** (red) — add the five `admin.rs` cases above against the
   target endpoint shape.
2. **Backend endpoint** — implement `get_assessment_admin`, register route, `make openapi`;
   tests from task 1 go green.
3. **Shared `QuestionList` component** — extract from the learner preview; learner page
   unchanged (prompt-only) and still passing.
4. **Admin preview drawer** — eye icon + drawer + answer-revealing list + footer
   Delete/Restore wired to existing dialogs.
5. **E2E** — drawer-open + delete-from-drawer smoke.

Gate: `make check` per commit, `make ci` (DB + e2e) before the PR; OpenAPI drift gate
non-negotiable.
