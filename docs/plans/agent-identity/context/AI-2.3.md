# AI-2.3 — Update agent skills + reference client

**Phase:** 2 · **Lane:** agents · **Agent:** haiku-developer · **Depends on:** AI-2.2, AI-1.3
**Spec:** §"API surface" (agents + agent tools)
**Paths (edit only here):** `agents/`

## Objective
Keep the agent-facing docs/client in lock-step with the new surface: owner
creates agents via `/v1/me/agents`; agents call `profile.get` / `memory.*` /
`target.set`.

## Read first
- `agents/` — the existing skill markdowns + the one committed reference client
  (the dep-free `client.py`-style artifact; do **not** spawn throwaway scripts).

## Do
- Document the new tools with **exact** request/response JSON (copy from the
  handlers, don't paraphrase).
- Update the reference client to cover create-agent + the three tools, so it
  doubles as the manual test substrate.

## Gotchas
- Tell agents not to guess schemas — point them at `/openapi` / `llms.txt`.
- Keep it one committed client, not per-task urllib scripts.

## Done when
- Skills + client reflect the new surface; `make check` passes.
