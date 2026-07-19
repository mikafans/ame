# Deep-dive provenance certification

Status: passed on 2026-07-20 against the recommended local stack.

## Scenario

A learner created a source-backed deep dive from completed-attempt evidence.
The same learner then supplied an unknown evidence ID. A second learner tried
to reuse the first learner's evidence while creating a deep dive in the
second learner's journey. The unknown and cross-owner requests were rejected
without creating content.

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  covers the valid deep dive, unknown evidence, and cross-owner evidence paths
  through containerized Caddy; the test passes with `2 passed` for the full
  learning-loop module.
- `api/src/deep_dive.rs` covers the in-memory contract: unregistered evidence
  is rejected and registered evidence is readable by its owner.
- `api/src/deep_dive_postgres.rs` verifies evidence ownership and exact
  journey/objective/activity alignment before insertion.
- `make openapi && make public-docs` regenerates the agent description and
  canonical public manifest from the same source.

## Enforced invariant

A deep dive is accepted only when its triggering evidence exists, belongs to
the authenticated learner, and matches the submitted journey, objective, and
activity. Unknown or cross-owner evidence is treated as an inaccessible
resource and cannot create a deep dive.
