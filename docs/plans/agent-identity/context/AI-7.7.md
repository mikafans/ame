# AI-7.7 — e2e: agent create, public quiz take, visibility

**Phase:** 7 · **Lane:** web · **Agent:** sonnet · **Depends on:** AI-7.2, AI-7.3, AI-7.4
**Spec:** §"Cross-owner attempts" + §"Agents (owner-driven)"
**Paths (edit only here):** `web/e2e/`

## Objective
Lock the new flows with end-to-end coverage.

## Do
Add specs covering:
1. Owner creates an agent token on the Agents page; secret shown once.
2. A **second** user takes an owner's **public** quiz via `/explore`; the attempt
   + stats roll up to the taker (assert on the taker's progress, not the author's).
3. A **private** quiz is 404 for a non-owner.

## Read first
- Existing e2e auth pattern: one shared `loginAs` per user in a single
  `beforeAll`; `addCookies` + `waitForLoadState('networkidle')` for
  sidebar/role-gated assertions. Two users here means two distinct logins —
  don't let their `beforeAll` hooks invalidate each other's tokens (the login
  API keeps a single active token per user).

## Gotchas
- Use `bunx @playwright/cli`. Seed via `make db-seed` (API must be running).
- Keep specs independent; don't rely on AI-7.5/7.6 pages.

## Done when
- All three flows pass; `make e2e` green.
