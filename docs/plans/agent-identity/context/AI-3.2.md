# AI-3.2 — Quiz visibility: PATCH + listing + cross-owner GET + /v1/explore

**Phase:** 3 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-3.1, AI-1.2
**Spec:** §"API surface → Visibility / listing / sharing"
**Paths (edit only here):** `api/src/http/quizzes.rs`, `api/src/domain/`, `api/openapi.yaml`

## Objective
Make the `visibility` column the source of truth for quiz access, and add public
discovery.

## Do
- A `Visibility` enum in `api/src/domain/` ({Private, Unlisted, Public},
  serde snake_case) — mirror the `Scope`/`Role` enum style in `domain/user.rs`.
- `PATCH /v1/quizzes/{id}` accepts `visibility`. Publishing to `public` is
  **plan-gated** (call the gate from AI-4.x if present; until then leave a
  clearly-marked TODO hook, allow `unlisted` always) and writes an audit entry
  (audit helper arrives in AI-6.2 — TODO-hook it the same way).
- Owner list filter: `WHERE visibility='public' OR created_by IN (me + my agents)`
  — use `AuthenticatedUser.owner_id` to expand "me + my agents".
- `GET /v1/quizzes/{id}`: resolve `unlisted`/`public` for **any** caller
  (read-only); `private` → 404 for non-owners.
- `GET /v1/explore`: paginated `visibility='public'` quizzes (reuse the existing
  keyset/pagination pattern from the question-bank work).

## Gotchas
- MC options stay bare `Vec<String>`.
- Don't implement the *attempt* path here — taking a quiz is AI-3.3 (sessions.rs).
- Mark the plan-gate and audit calls as integration points so AI-4.2/AI-6.2 can
  wire them without re-touching this file.

## Done when
- PATCH/list/get/explore behave per the access matrix; `make openapi` updated;
  `make check` passes.
