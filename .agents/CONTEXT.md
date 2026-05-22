# CONTEXT.md — Project context for AI sessions

## Current stage
`feat/quiz-sessions-squash` — all 12 visual/functional gaps fixed and verified. Seed creates rich demo data (4 quizzes with course labels, 8 finished learner sessions). Ready for PR or next feature work.

## Reference docs (read these before making decisions)
- **`docs/specs/2026-05-20-harus-platform-design.md`** — canonical spec (473 lines). Replaces the May-19 spec.
- **`docs/plans/2026-05-20-design-gap-analysis.md`** — gap audit: design has Share + LearningObjectives features the spec doesn't cover yet. See it for the exact edits the spec needs.
- `docs/specs/2026-05-19-question-exam-platform-design.md` — historical only; do NOT edit, do NOT rely on for new work. Migration deltas are in §13 of the new spec.
- `design/README.md`, `design/api.md`, `design/tokens.md`, `design/source/src/*.jsx` — pixel-faithful UI source of truth.
- `docs/plans/` — implementation plans (currently still the May-19 ledgers; new plan markdowns + task ledgers are pending).
- `docs/v0/decisions.md` — brainstorming outcomes (historical).

## Active constraints
- No `docs/superpowers/` subdirectory. Plans live in `docs/plans/`.
- Worktrees by default for isolated work (see global CLAUDE.md).
- Sub-agents by default for independent tasks.
- Solo developer, production in Kubernetes; friends-later is in scope for schema, not UX.
- `make check` skips web (prettier/lint/type-check/bun test) when `web/node_modules` is missing — gated in the Makefile so docs-only commits don't require `bun install`.

## Module boundaries (api/src/)
`engine/` and `assess/` may depend on `bank/` and `domain/`. Reverse is forbidden. `bank/` does not know that attempts exist.

## Conventions
- Conventional Commits: `feat:`, `fix:`, `chore:`, `docs:`, `deploy:`. No emojis.
- 2-space indent for config (YAML, TOML, JSON, MD); 4-space for Rust/Python; tabs for Makefile.
- Stage specific files: `git add path/to/file`, never `git add -A` or `git add .`.
- Always ask before committing or pushing.
