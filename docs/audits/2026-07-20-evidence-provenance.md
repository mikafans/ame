# Evidence provenance certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

A learner started an assessment attempt, then tried to create mastery evidence
before completing it and without supplying an attempt. Both requests were
rejected. The learner then answered and finished the assessment and submitted
the same evidence with the completed attempt ID.

Observed live output:

```text
journey=019f7b63-aa27-7907-a3f8-cbdb1b40a4da
missing attempt=422, unfinished attempt=404
evidence rows before=0, after rejected=0, after graded=1
evidence provenance certification: PASS
```

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers missing attempt, unfinished attempt, and valid completed-attempt
  evidence behavior through Caddy.
- `api/src/progress.rs` repository behavior continues to pass with
  `cargo test --lib progress`.
- `make local-up` rebuilt the API/web/containerized-Caddy stack without a
  migration or volume reset.
- `make openapi && make public-docs` regenerated the required `attemptId` field
  in OpenAPI, `skill.json`, and the frontend schema.

## Enforced invariant

0.3 mastery evidence must reference a completed graded attempt owned by the
learner and linked to the journey, objective, and activity. A caller cannot
advance mastery by submitting an untrusted score alone.
