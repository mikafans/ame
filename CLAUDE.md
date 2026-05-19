# CLAUDE.md — Claude-specific rules

Inherits everything from `AGENTS.md`.

## Session workflow
- This project uses `.agents/` for session state. Run `/session start` at the beginning of each conversation; `/session end` before closing.
- MCP memory server is canonical when available; `.agents/CURRENT_TASK.md` is the fallback.

## When in doubt
- Defer to `docs/specs/2026-05-19-question-exam-platform-design.md`.
- The spec self-review pass on 2026-05-19 fixed: server-side MCQ shuffle trust model, `needs_work_score` formula sign, `correct_index` field naming, `level_label` resolution rule, and `affects_rating` on sessions. Trust the current text of the spec, not earlier drafts.