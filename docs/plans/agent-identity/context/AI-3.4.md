# AI-3.4 — Demote share-links to presentation; delete anon-attempt path

**Phase:** 3 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-3.1
**Spec:** §"Schema deltas → Visibility" prose + §"Visibility / listing / sharing"
**Paths (edit only here):** `api/src/http/shares.rs`

## Objective
The quiz `visibility` column is now the access gate. Share-links keep only
presentation flags; the anonymous-attempt path is removed (answering requires an
account — that flow lives in AI-3.3).

## Do
- Delete the anonymous-attempt handlers + routes (`embed_attempt_*`,
  `/v1/{kind}/{id}/embed/attempt`) and all `tb_anonymous_attempts` SQL
  (`check_and_count_anon`, the INSERT). AI-3.1 drops the table.
- Keep `POST /v1/shares` and the read-only `embed_*` views, but `visibility` on
  a share no longer grants access — it's presentation only
  (`include_score`/`include_explanation`/`include_attribution`).
- Creating a share for a `private` quiz auto-promotes it to `unlisted` (sharing
  = intent to expose by link). That's a small UPDATE on `tb_quizzes`.

## Read first
- `api/src/http/shares.rs` (current file — embed + anon logic is lines ~123,
  ~390–460; router at the bottom).

## Gotchas
- Don't add per-link opaque tokens (decided out-of-scope).
- The auto-promote is the only write this file makes to `tb_quizzes`; keep it
  minimal and idempotent.

## Done when
- No anonymous-attempt code/routes remain; embed stays read-only; share visibility
  is presentation-only; private→unlisted auto-promote works. `make check` passes.
