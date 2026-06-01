# AI-4.3 — Scope-ceiling on agent create

**Phase:** 4 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-4.2, AI-1.3
**Spec:** §"Authorization → Scope ceiling"
**Paths (edit only here):** `api/src/http/me.rs`

## Objective
An agent's granted scopes must be a subset of what the owner's plan permits.

## Do
- In `POST /v1/me/agents` (AI-1.3's handler), before issuing the token, validate
  requested `scopes ⊆ plan_ceiling(owner.plan)` using the quota/plan model from
  AI-4.2.
- Free plan forbids the public-publish capability (whatever scope gates
  `visibility='public'`); over-ceiling request → `403` with the offending scope.
- Enforce the per-owner **agent count** quota here too (call `check_quota`).

## Gotchas
- Same file as AI-1.3 — coordinate (this is a follow-on edit, so AI-1.3 must be
  `done` first; they are not parallel).
- Don't duplicate the plan ceiling definition; import it from AI-4.2.

## Done when
- Over-ceiling scopes → 403; agent-count quota enforced; `make check` passes.
