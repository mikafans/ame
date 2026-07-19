# Progress ownership certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

Two independently registered learners were created through the public
onboarding endpoint. The second learner attempted to write evidence and a
recommendation using the first learner's journey, objective, and activity IDs.

Observed live output:

```text
journey=019f7b59-7e2f-7755-89c0-bd805c214f79
cross-owner evidence=404
cross-owner recommendation=404
evidence rows before=0, after rejection=0, after valid owner write=1
ownership certification: PASS
```

The `404` response deliberately does not disclose whether another learner's
resource exists. The rejected evidence request did not create a row in
`tb_mastery_evidence`; the valid owner request created exactly one row.

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers cross-owner evidence and recommendation rejection through Caddy.
- `api/src/progress.rs::tests::invalid_or_cross_subject_evidence_cannot_create_progress`
  covers the in-memory journey-owner contract.
- `cargo test --lib progress` and `uv run pytest api_tests/test_learning_loop.py -q`
  pass.
- The live API was rebuilt with `make local-up`; no database migration or
  volume reset was used.

## Enforced invariant

PostgreSQL accepts evidence only when the journey, objective, activity, their
relationship, and optional attempt all belong to the authenticated learner.
Recommendation targets use the same ownership and relationship check.
