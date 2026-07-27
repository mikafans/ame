# Streak calendar certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

A learner completed a graded attempt and submitted its event key with the
current calendar day in `Asia/Tokyo`. The same learner then submitted the
identical completed attempt with the previous day and with an unknown timezone.
The valid event was accepted; both fabricated variants were rejected.

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers the valid day, mismatched day, and invalid-timezone paths through
  containerized Caddy; the module passes with `2 passed`.
- `api/src/domain/progress.rs` centralizes IANA timezone conversion from the
  attempt's graded timestamp.
- `api/src/progress.rs` and `api/src/progress_postgres.rs` both require the
  submitted day to equal that derived local day.
- `make check` and `make local-uiux` remain the final repository/browser gates;
  the implementation was exercised after rebuilding the preserved local
  Postgres/Valkey/API/web/Caddy stack.

## Enforced invariant

A streak event is accepted only for a completed graded attempt whose
`qualifyingDay` is the attempt's actual calendar day in the supplied valid
IANA learner timezone. Clients cannot manufacture historical or future streak
days by changing request metadata.
