# Learner analytics semantics

`GET /api/v1/learning/journeys/{id}/analytics?timezone=<IANA name>` is
authenticated-owner-only. It never aggregates across learners.

- Current and best streak use distinct persisted qualifying events converted to
  the requested timezone. Current means the final consecutive run ends today or
  yesterday.
- Completion rate is completed curriculum activities divided by all activities
  in the journey.
- Average score is the sum of normalized graded assessment scores and completed
  reviewed task scores divided by those scored outcomes. Pending manual review,
  zero-point assessments, and abandoned attempts are excluded.
- Attempt count includes submitted and graded assessment attempts. Repeated
  answer writes do not add attempts.
- Time spent sums non-negative durations of finished learning sessions. Open and
  abandoned sessions are excluded.
- Mastery trend exposes persisted evidence events without interpolation.
- Review history is append-only: each real FSRS rating records its before/after
  due time and resulting interval.

Empty denominators return `null`, not zero percent. Every ratio returns its
numerator and denominator.

The analytics migration adds composite indexes for owner/journey review history,
finished-session duration queries, and graded attempts. On a representative
fixture, inspect plans with:

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT *
FROM tb_review_events
WHERE subject_user_id = '<owner>' AND journey_id = '<journey>'
ORDER BY reviewed_at DESC;
```
