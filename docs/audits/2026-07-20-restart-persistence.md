# Local-stack restart persistence audit

The recommended local stack preserves learner state across a non-destructive
container restart on the same Caddy origin.

## Rehearsal

`uv run python scripts/test_local_stack_persistence.py`:

1. registers a unique learner and starts a journey through `/public/v1`;
2. runs `local_stack.py down` and `local_stack.py up` without `db-reset`,
   `down --volumes`, or any volume deletion;
3. logs in again through `/public/v1/auth/login`;
4. reads the same journey through `/api/v1/learning/journeys/{id}` and checks
   the original intent.

The rehearsal passed on 2026-07-20.

## Follow-up evidence

- The black-box API suite passed all 5 tests after the restart.
- All 4 local Caddy Playwright journeys passed after the restart, including
  returning-learner resume and pending manual review.
- The stack remains the recommended Postgres + Valkey + API + web + Caddy
  composition; no system-level Caddy or cache replacement is introduced.
