# Evidence contract parity certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

The HTTP learning-loop test submitted evidence without an `attemptId`, with an
unfinished attempt, and with a completed graded attempt. Missing provenance
was rejected as validation, unfinished provenance was rejected as an
inaccessible resource, and the completed attempt produced one evidence row.

The in-memory repository contract also rejects an unregistered attempt and
accepts evidence only after the exact learner/journey/activity/attempt tuple is
registered as completed.

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers missing, unfinished, and completed attempt-backed evidence through
  Caddy; the module passes with `2 passed`.
- `api/src/progress.rs` covers the same completed-attempt requirement in the
  in-memory repository.
- `api/src/progress_postgres.rs` requires the same non-null attempt ID to be a
  graded, owner- and journey/activity-matching attempt before insertion.
- `make public-docs` regenerated the canonical public artifacts; `make check`
  and `make local-uiux` pass after the contract change.

## Enforced invariant

Mastery evidence is always attempt-backed. A caller cannot create progress by
submitting only a score, and the in-memory and PostgreSQL repository contracts
reject the same invalid provenance states.
