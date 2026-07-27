# Review visibility audit — 2026-07-20

## Decision

Generated deep dives are not learner-ready merely because they were persisted.
The unified learner API exposes a deep dive only when `review_status` is
`approved`. Draft, review, and rejected records remain storage-side authoring
state and read as not found from both learner read paths.

## Evidence

- `InMemoryDeepDiveRepository::get` and `get_for_activity` enforce approved-only
  reads.
- `PgDeepDiveRepository::get` and `get_for_activity` apply the same predicate
  in SQL, before reconstructing a learner response.
- `DeepDiveResponse.reviewStatus` preserves the explicit status in the public
  response contract; learner-visible responses therefore report `approved`.
- `api/src/deep_dive.rs` tests the hidden `review` path and the approved happy
  path.
- `api_tests/test_learning_loop.py` asserts the HTTP response status and the
  approved discovery/read paths.

## Boundary

The create endpoint may persist unreviewed content for a future authoring or
review workflow. It must not be used as evidence that content is suitable for
learner discovery. A future review-transition endpoint must preserve the same
owner and audit constraints.
