# AI-2.2 — Agent tools: profile.get / memory.* / target.set

**Phase:** 2 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-2.1, AI-1.2
**Spec:** §"API surface → Agent tools"
**Paths (edit only here):** `api/src/http/agents.rs`

## Objective
Add three tools to the agent run surface, callable by an agent's own token:
- `profile.get` → the agent's own `tb_agent_profiles` row **plus** the owner's
  shared truth: level (`tb_user_tag_ratings`), recent progress, next target.
- `memory.set` / `memory.append` → write the agent's `memory` jsonb.
- `target.set` → update `current_goal` / `next_target`.

## Read first
- `api/src/http/agents.rs` — how existing tools (`quiz.import`, `question.create`,
  …) dispatch on a tool name within `/v1/agents/run`. Follow that exact shape.
- `AuthenticatedUser.owner_id` from AI-1.2 — use it for shared-truth reads.

## Gotchas
- **Shared-truth reads hit the OWNER's rows, never the agent's** — use
  `owner_id`, not `user.id`. This is the single most important invariant here.
- `memory.append` must merge into the jsonb, not overwrite (`||` / jsonb_set).
- Don't touch the owner-facing CRUD (`/v1/me/agents`) — that's AI-1.3's file.

## Done when
- The three tools work end-to-end; shared truth resolves to the owner.
- `make check` passes (add a test asserting an agent reads owner level).
