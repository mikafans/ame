# Streak provenance certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

A learner completed a graded assessment attempt and used its attempt ID as the
qualifying event. The same learner also tried an arbitrary event key; another
learner tried to reuse the completed attempt event. The valid event was retried
to verify idempotency.

Observed live output:

```text
journey=019f7b6a-bf37-76a6-8443-de9f1f3c2d0b
invalid key=422, valid=200, duplicate=200, cross-owner=404
event rows before=0, after all requests=1, same-id=True
streak provenance certification: PASS
```

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers invalid event keys, cross-owner rejection, and same-owner duplicate
  behavior through Caddy.
- `api/src/progress.rs` covers the in-memory completed-attempt registration and
  idempotency contract.
- `cargo test --lib progress` and the live API black-box suite pass.
- `make local-up` rebuilt the API/web/containerized-Caddy stack without a
  migration or volume reset.
- `docs/public/llms.txt` and generated `skill.json` document the
  `attempt:<completed-attempt-id>` event-key contract.

## Enforced invariant

A streak is accepted only for a completed graded attempt owned by the learner
and linked to the submitted journey and activity. Arbitrary agent assertions
cannot create a streak.
