# AI-7.2 — Agents management page

**Phase:** 7 · **Lane:** web · **Agent:** sonnet · **Depends on:** AI-1.3
**Spec:** §"API surface → Agents (owner-driven)"
**Paths (edit only here):** `web/app/(learner)/agent/`

## Objective
UI for an owner to manage their agent sub-accounts against `/v1/me/agents`.

## Do
- List agents (label, scopes, last_used, focus) from `GET /v1/me/agents`.
- Create: `POST /v1/me/agents {label, scopes, focusTags?}`; show the returned
  **secret exactly once** with a copy button + a "you won't see this again" note.
- Edit focus/goal/next-target via `PATCH`; revoke via `DELETE` behind a MUI
  Dialog (Cancel + red confirm) — never `window.confirm`.

## Read first
- The existing `(learner)/agent/` page (it currently renders the MCP/agent
  surface) — extend, don't rebuild from scratch.
- The typed OpenAPI client used elsewhere in `web/` for API calls.

## Gotchas
- All MUI; no Tailwind for new UI. No placeholder/no-op buttons — wire or omit.
- Secret-once: never refetch or display the secret after creation.

## Done when
- Create/list/edit/revoke all work against the live API; revoke is a MUI Dialog;
  `make e2e` passes.
