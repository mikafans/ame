# Progress visibility audit

The learner desk now projects durable C5 progress for every objective in each
journey, rather than showing only the first objective's snapshot.

## Certified behavior

- `/learning` retrieves the current journey detail and requests the existing
  owner-scoped objective snapshot endpoint once per objective.
- Every returned objective renders a stable progress card with its statement,
  mastery signal, and evidence count, or an explicit `No signal yet` state.
- The existing journey summary still shows the first objective's compact signal
  and the timeline/streak summaries; this change does not alter API or storage
  semantics.
- The browser derives the objective set from the live journey response, so new
  topics and template shapes do not require frontend hardcoding.

## Evidence

- `web/app/(learner)/learning/page.tsx` loads and renders all objective
  snapshots.
- `web/e2e/uiux.spec.ts` checks every objective returned by the live API using
  `objective-progress-{id}` cards.
- TypeScript checks pass.
- All three Caddy-routed local-stack Playwright journeys pass.

The C5 release gate remains open for the broader restart and cross-client
rehearsal; this slice certifies the learner-visible projection only.
