# Streak ownership certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

Two independently registered learners were created through public onboarding.
The second learner attempted to record a qualifying event against the first
learner's journey and activity. The first learner then recorded the same event
and retried it.

Observed live output:

```text
journey=019f7b5d-af5c-7728-9366-3d94b779d89e
cross-owner streak=404
event rows before=0, after rejection=0, after owner write=1
owner write=200, duplicate=200, same-id=True
streak ownership certification: PASS
```

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers cross-owner rejection and same-owner duplicate behavior through Caddy.
- `api/src/progress.rs::tests::streak_recording_is_idempotent_and_timezone_explicit`
  covers the in-memory ownership and duplicate contract.
- `cargo test --lib progress` and `uv run pytest api_tests/test_learning_loop.py -q`
  pass.
- `make local-up` rebuilt the API/web/containerized-Caddy stack without a
  migration or volume reset.

## Enforced invariant

PostgreSQL accepts a streak event only when its journey and activity belong to
the authenticated learner and the activity belongs to that journey. Repeating
the same qualifying event remains idempotent for that owner.
