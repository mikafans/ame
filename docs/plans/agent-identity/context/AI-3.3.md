# AI-3.3 — Cross-owner authenticated attempts + responder rollup

**Phase:** 3 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-3.1, AI-1.2
**Spec:** §"API surface → Cross-owner attempts (account-required)"
**Paths (edit only here):** `api/src/http/sessions.rs`

## Objective
Let any authenticated user take someone else's public/unlisted quiz through the
normal session flow, with the attempt bound to the **responder**.

## Do
- `POST /v1/sessions {quizId}`: allow when the caller can read the quiz —
  i.e. they own it, **or** its visibility is `public`/`unlisted`. Today this
  almost certainly assumes ownership; widen the authorization check only.
- The created `tb_session` + answers + resulting stats belong to the
  responder (`user.id`), not the quiz author.
- Authors get **aggregate counts only** — never responder identities or scores.
  If you add a count, expose it as a number, nothing per-user.
- `private` quiz for a non-owner → 404.

## Read first
- `sessions.rs` (~1176 LoC) start/answer/finish handlers. The seed answer
  endpoint is `POST /v1/sessions/{id}/answer` (singular). Don't rename routes.

## Gotchas
- Don't touch quiz listing/visibility writes (AI-3.2) or shares (AI-3.4).
- Reuse `AuthenticatedUser.owner_id` only where "my content" semantics apply;
  a cross-owner attempt is explicitly NOT the author's content.

## Done when
- A second user can take a public/unlisted quiz; attempt + stats roll up to
  them; private → 404. `make check` passes (add a cross-owner attempt test).
