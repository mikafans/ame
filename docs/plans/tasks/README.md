# Task Ledger

This directory turns the implementation plans in `docs/plans/` into agent-claimable JSONL tasks.

## Files

- `phases.jsonl` tracks the overall project phases.
- `plan-*.jsonl` tracks concrete tasks for each implementation plan.

## JSONL schema

Each line is one task object:

```json
{"id":"P3-002","phase":"plan-3-question-bank","source_plan":"docs/plans/2026-05-19-plan-3-bank.md","status":"todo","owner":null,"title":"Implement bank repository layer","paths":["api/src/bank","api/src/domain"],"depends_on":["P3-001"],"acceptance":["make check passes"],"notes":"Keep bank independent from attempts/sessions."}
```

## Status values

- `done` - completed and verified.
- `review` - implemented, but needs a focused review or cleanup pass before downstream work depends on it.
- `todo` - ready once dependencies are done.
- `blocked` - cannot proceed without an explicit external decision or dependency.

## Multi-agent protocol

1. Claim one task by changing only its `owner` and, if needed, `status`.
2. Work only in the listed `paths` unless the task notes say otherwise.
3. If a task needs broader edits, add a new task line instead of expanding silently.
4. Commit small, task-scoped changes with the task ID in the commit body or message.
5. Update the task line when verified.
