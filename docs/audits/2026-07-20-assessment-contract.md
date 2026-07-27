# Assessment contract audit

The 0.3 assessment contract keeps practice and graded work distinguishable at
the attempt boundary used by both the browser and external agents.

## Certified behavior

- Every attempt returns `assessmentMode` from its immutable assessment:
  `practice` or `graded`.
- A newly started attempt returns `reviewStatus: not_required`.
- An objectively gradable attempt finishes as `graded` with
  `reviewStatus: complete` and a server-derived score.
- An attempt containing essay or code work finishes as `submitted` with a null
  score and `reviewStatus: pending`; item results identify `manual_review`.
- The browser labels the assessment from the attempt contract, so a practice
  result cannot be presented as an exam result after a refresh or resume.

## Evidence

- `api/src/domain/attempt.rs` defines the mode and review-state contract.
- `api/src/attempt.rs` and `api/src/attempt_postgres.rs` keep the in-memory and
  PostgreSQL repositories aligned.
- `api/src/assessment.rs` verifies that mixed assessments preserve objective
  item grades while keeping the overall result pending for manual review.
- `api/src/http/attempts.rs` exposes the fields in every attempt response.
- `api_tests/test_learning_loop.py` asserts mode and review state across start,
  finish, and persisted reads.
- `web/app/(learner)/learning/journeys/[id]/page.tsx` consumes
  `attempt.assessmentMode` for the learner-facing label.
- Generated OpenAPI, frontend schema, `skill.json`, and `llms.txt` were
  regenerated from the source contract.

The focused API behavior test and the complete Rust/frontend check are the
release evidence for this slice; the full C1-C6 release gate remains open.
