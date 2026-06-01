# AI-7.4 — Explore page + signup-to-answer redirect

**Phase:** 7 · **Lane:** web · **Agent:** sonnet · **Depends on:** AI-3.2, AI-3.3
**Spec:** §"Visibility / listing / sharing" + §"Cross-owner attempts (account-required)"
**Paths (edit only here):** `web/app/explore/`

## Objective
Public discovery + the account-required answering funnel that turns shared links
into new users.

## Do
- `/explore`: paginated list of public quizzes from `GET /v1/explore`. Place it
  **outside** the auth-gated `(learner)` group so anonymous visitors can load it.
- Opening a public/unlisted quiz shows it **read-only** (preview) for anon users.
- The answer action requires login: redirect anon → signup, then **resume** the
  quiz (carry the quiz id through, start the session via `POST /v1/sessions`
  after auth). The attempt rolls up to the new user (handled server-side in AI-3.3).

## Gotchas
- Don't expose other responders' attempts/scores — preview shows the quiz, not
  results.
- This is the growth funnel: keep the signup→resume hop tight (one redirect,
  preserve intent), not a dead-end "please log in".

## Done when
- `/explore` lists public quizzes; anon can preview; answering routes through
  signup and resumes; `make e2e` passes.
