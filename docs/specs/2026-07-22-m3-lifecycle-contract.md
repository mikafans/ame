# M3 lifecycle contract

M3 separates instructional completion from demonstrable skill. Assessments and application tasks are different activity outcomes and must not update mastery through the same implicit path.

## Assessment lifecycle

- Assessment definition: `draft -> published -> retired`.
- Learner attempt: `in_progress -> submitted -> graded` for automatically gradeable work.
- Learner attempt requiring review: `in_progress -> submitted -> graded` with `review_status = pending` until review completes.
- Learner attempt may end as `abandoned`; abandoned work does not create evidence.

The existing assessment and attempt contracts remain the source of truth for quiz behavior.

## Task lifecycle

- Task definition: `draft -> published -> retired`.
- Submission: `in_progress -> submitted`.
- Reviewable submission: `submitted -> in_review -> reviewed` or `rejected`.
- A rejected submission may return to `in_progress` for revision.
- A submission may end as `abandoned`; it cannot later be submitted.

Task evaluation methods are explicit: `self_review`, `automatic`, `agent`, or `manual`. A task result is evidence only after the declared evaluation path reaches its terminal reviewed state; merely selecting a scenario option or saving a draft is not mastery evidence.
