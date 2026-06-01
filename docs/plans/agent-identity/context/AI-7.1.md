# AI-7.1 — De-gate (learner) route group; drop instructor-only access

**Phase:** 7 · **Lane:** web · **Agent:** haiku-developer · **Depends on:** AI-0.2
**Spec:** §"Decided model" #2 + §"Goal" (authoring moves to the agent surface, single user role)
**Paths (edit only here):** `web/app/(learner)/layout.tsx`, `web/app/(learner)/author/`, `web/app/(learner)/grading/`

## Objective
There is no separate `(instructor)` route group — instructor features
(`author/`, `grading/`, `agent/`) live under `(learner)/` behind role gates.
Collapse to one role: any logged-in `user` can reach them.

## Do
- Remove instructor/role gating on `author/` and `grading/` (and their nav
  entries) so an authenticated user can access them.
- **Keep the `(learner)` directory name** — route groups in parens don't affect
  URLs, and renaming it would conflict with every other phase-7 task's paths.
  Leave a one-line note that the name is now a legacy label.

## Gotchas
- `useAuth` role checks: collapse `instructor`/`learner` → `user`. Don't rewrite
  the auth hook's plumbing, just the role conditions.
- Admin-only items (AI-7.6) stay admin-gated — don't expose those.

## Done when
- Author/grading reachable by any logged-in user; no instructor role strings
  gate the learner group; build + `make e2e` pass.
