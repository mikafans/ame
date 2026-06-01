# AI-0.3 — Remove instructor e2e spec + role string refs (web)

**Phase:** 0 · **Lane:** web · **Agent:** haiku-developer · **Depends on:** AI-0.2
**Spec:** §"Decided model" #2 (instructor UI/route-group + spec removed)
**Paths (edit only here):** `web/e2e/`, `web/app/`

## Objective
Purge frontend references to the dropped `learner`/`instructor` roles so the
web build and remaining e2e suite stay green after the backend role collapse.

## Read first
- `web/e2e/roles/instructor.spec.ts` — delete it.
- `rg "instructor|learner" web/` — every role string usage. Auth/role helpers
  may branch on these strings; collapse to `user`/`admin`/`agent`.

## Do
- Delete `roles/instructor.spec.ts`.
- Replace `learner`/`instructor` role strings with `user` where they gate UI;
  remove dead instructor-only branches. (Removing the instructor **route-group
  UI** itself is AI-7.1 — here just stop referencing the old role strings so
  nothing breaks; don't restructure pages.)

## Gotchas
- e2e auth is finicky here: a shared `loginAs` per user in one `beforeAll`,
  `addCookies` + `waitForLoadState('networkidle')` for sidebar-gated assertions.
  Don't refactor that machinery — just remove instructor cases.

## Done when
- `instructor.spec.ts` gone; no `learner`/`instructor` role strings in `web/`.
- `make e2e` green for the remaining specs.
