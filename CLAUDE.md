# CLAUDE.md

...

## Database & Testing Standards

- **SQL Ambiguity**: Always fully qualify column names in SQL queries involving multiple tables (e.g., `q.id` instead of `id`) in both `SELECT` and `ORDER BY` clauses to prevent ambiguous column reference errors.
- **Query Formatting**: Use plain string literals for SQL queries; avoid `format!` unless dynamic construction is strictly necessary.
- **DB Test Stability**: When writing integration tests that filter database records, use set-membership assertions rather than exact order comparisons to avoid non-deterministic failures from index/result ordering variations.
 — Claude-specific rules

Inherits everything from `AGENTS.md`.

## Session workflow
- This project uses `.agents/` for session state. Run `/session start` at the beginning of each conversation; `/session end` before closing.
- MCP memory server is canonical when available; `.agents/CURRENT_TASK.md` is the fallback.

## When in doubt
- Defer to `docs/specs/2026-05-20-harus-platform-design.md` (the canonical redesign spec).
- May-19 plans are archived in `docs/_archive/` — do not reference them for new work.