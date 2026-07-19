# Generation provenance certification

Status: passed on 2026-07-20 against a freshly recreated local PostgreSQL
volume.

## Enforced invariant

Generated questions, immutable question versions, and explanatory deep dives
must carry a `generationRunId`. The run must belong to the same learner, use
the content's declared operation (`question.compose` or `deep_dive.create`),
and reach `published` before content is stored. Missing, failed, cross-owner,
and operation-mismatched runs are rejected before content creation. The clean
0.3 baseline stores the relationship as non-null foreign keys.

## Test evidence

- `api_tests/test_learning_loop.py` covers missing, failed, and cross-owner
  question provenance plus missing deep-dive provenance; the fresh-stack
  module passes with `3 passed`.
- In-memory question and deep-dive repository tests register only published,
  operation-matching runs before accepting content.
- PostgreSQL repositories validate the owner-scoped generation lifecycle before
  inserting content and persist the run ID on question versions and deep dives.
- `make public-docs` regenerates the canonical OpenAPI and agent manifest after
  the API contract change.

## Agent workflow

An agent starts a generation run, records provider progress, transitions it to
`published`, and passes the returned `generationRunId` when creating content.
The generation run is provenance, not an authorization scope; the learner
bearer session remains the sole owner boundary.
