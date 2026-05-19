# CONTEXT.md — Project context for AI sessions

## Current stage
Plan 1 (Foundation) — replacing the Dioxus scaffold with Axum + Next.js + Postgres.

## Reference docs (read these before making decisions)
- `docs/specs/2026-05-19-question-exam-platform-design.md` — full spec (931 lines).
- `docs/v0/decisions.md` — brainstorming outcomes.
- `docs/v0/research-notes.md` — prior-art sources.
- `docs/plans/` — implementation plans.

## Active constraints
- No `docs/superpowers/` subdirectory. Plans live in `docs/plans/`.
- Worktrees by default for isolated work (see global CLAUDE.md).
- Sub-agents by default for independent tasks.
- Solo developer, production in Kubernetes; friends-later is in scope for schema, not UX.