# Manual-review flow audit

AME now treats essay and code assessment work as a completed learner session
with an explicit pending-review result, without manufacturing mastery evidence.

## Certified behavior

- A manual-review assessment finishes with `status: submitted`, a null score,
  and `reviewStatus: pending`.
- The browser renders `Assessment submitted · pending review` and completes the
  learning session so the learner can continue or return later.
- Evidence is posted only for attempts whose final status is `graded`.
  Pending manual work therefore cannot create a zero-valued or otherwise false
  progress signal before review.
- The same attempt remains in history and is resumable through the learner's
  owner-scoped journey.

## Evidence

- `web/app/(learner)/learning/journeys/[id]/page.tsx` gates evidence creation on
  the server-returned attempt status.
- `web/e2e/uiux.spec.ts` creates an approved essay question and graded
  assessment through the API, then completes the real browser flow.
- The four local Caddy Playwright journeys pass, including the manual-review
  story.
- The full black-box API suite passes; the domain and attempt contracts already
  cover null scores and mixed automatic/manual grading.

Manual review completion itself is certified; an operator-facing reviewer UI
and later review decision endpoint remain outside this slice.
