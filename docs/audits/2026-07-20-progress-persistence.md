# Progress persistence certification

Status: passed on 2026-07-20 against the recommended local stack.

This certifies the PostgreSQL projections required by the 0.3 C5 story. It
does not claim clean-volume certification; the restart intentionally preserved
the existing PostgreSQL volume.

## Scenario

The following was run from the repository root:

```text
uv run python - <<'PY'
# Start a learner through POST /public/v1/onboarding/start.
# Record one POST /api/v1/progress/evidence.
# Read the objective snapshot and recommendation.
# Run `make local-up` without removing volumes.
# Read the same snapshot and recommendation with the original bearer token.
# Assert calculatedAt and the recommendation response are unchanged.
PY
```

Observed output:

```text
before restart: 2026-07-19T17:00:11.359127Z 019f7b52-2ece-75fb-b082-e550c58af525
after restart: 2026-07-19T17:00:11.359127Z 019f7b52-2ece-75fb-b082-e550c58af525
persistence certification: PASS
```

The database assertions were run against the live Postgres container after the
restart:

```text
tb_mastery_snapshots: evidence_count = 1, calculated_at = 2026-07-19 17:00:11.359127+00
tb_recommendations: status = proposed, recommendation_version = 1
```

## Test evidence

- `api_tests/test_learning_loop.py::test_agent_first_learning_loop_happy_evil_and_edge_paths`
  verifies repeated snapshot reads retain the same `calculatedAt`; it passes
  after the PostgreSQL projection is materialized.
- `api/src/progress_postgres.rs` repository behavior continues to pass with
  `cargo test --lib progress`.
- `make local-up` rebuilt and restarted the API/web/Caddy services without
  deleting the Postgres volume; the learner state remained available.

## Boundary

Mastery is derived from server-recorded evidence. Clients cannot submit a
trusted mastery value directly. Recommendation rows are persisted as proposed
records and repeated identical proposals are reused rather than duplicated.
