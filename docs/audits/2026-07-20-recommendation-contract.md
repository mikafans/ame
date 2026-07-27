# Recommendation contract audit — 2026-07-20

## Decision

AME has one primary recommendation contract. The journey response shown to a
learner and the explicit progress recommendation endpoint both use the same
evidence-backed weakest-objective calculation.

## Behavior

- Candidates are the learner-owned activities currently `ready` or
  `in_progress`, expanded through their objective links.
- Mastery and evidence are read from PostgreSQL by the progress repository.
- Ties remain deterministic through the repository's objective ordering rules.
- `objectiveId`, `activityId`, `rationale`, and `evidenceIds` identify why the
  recommendation was selected.
- Before any evidence exists, `evidenceIds` is empty; it is never supplied by
  the caller or inferred from the latest session payload.
- Both browser and external agents observe the same result from the journey
  endpoint and the progress endpoint.

## Evidence

- `api_tests/test_learning_loop.py` compares both HTTP responses over the same
  candidate set before and after a completed assessment.
- `api/src/progress.rs` preserves deterministic in-memory recommendation
  behavior and server-derived evidence IDs.
- `api/src/http/learning.rs` delegates journey recommendations to the
  PostgreSQL progress repository instead of using a separate first-ready
  heuristic.
- Generated OpenAPI, `skill.json`, `llms.txt`, and the frontend schema are
  regenerated from the resulting contract.
