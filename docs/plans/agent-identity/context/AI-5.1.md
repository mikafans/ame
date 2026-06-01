# AI-5.1 — GET /v1/me/export (premium, throttled, audited)

**Phase:** 5 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-4.2
**Spec:** §"API surface → Export" + §"Security considerations" (export is expensive)
**Paths (edit only here):** `api/src/http/export.rs` (new), `api/src/http/mod.rs`, `api/openapi.yaml`

## Objective
Premium-only data export of the owner's content.

## Do
- New `api/src/http/export.rs` with `GET /v1/me/export` → JSON (or zip) bundle of
  the owner's quizzes, questions, attempts, stats — aggregated over
  `owner_id + its agents`.
- **Premium-gated**: free plan → 403. Throttled (tighter than normal; reuse the
  quota/limit model from AI-4.2). Writes an audit entry (audit helper from
  AI-6.2 — TODO-hook if AI-6.2 isn't merged yet).
- Mount the route in `api/src/http/mod.rs` (the `logged` group).

## Gotchas
- Export is a heavy/abuse surface — make it throttled + audited, not a raw dump
  loop. Aggregate by `owner_id`.
- Keep it in its own module so it doesn't collide with `me.rs` (AI-1.3/4.3).

## Done when
- Premium owner gets their bundle; free → 403; throttled + audited;
  `make openapi` updated; `make check` passes.
