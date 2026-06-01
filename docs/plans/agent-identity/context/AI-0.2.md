# AI-0.2 — Collapse Role enum + delete register faucet

**Phase:** 0 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-0.1
**Spec:** §"Decided model" #2 and #3
**Paths (edit only here):** `api/src/domain/user.rs`, `api/src/auth/`, `api/src/http/`

This is the **serializing bottleneck** — it changes a central type, so no other
api task runs until it is `done`. Keep it tight and green.

## Objective
1. Collapse `Role` to `{User, Admin, Agent}` (drop `Learner`, `Instructor`).
2. Delete the public agent-registration faucet entirely.

## Read first
- `api/src/domain/user.rs` — the `Role` enum (lines 8–15).
- `api/src/auth/extractor.rs:91–102` — the role string→enum mapping; today it
  already maps `"user"→Learner`. Make it map `"user"→User`, `"admin"→Admin`,
  `"agent"→Agent`, and reject anything else.
- `api/src/http/mod.rs:78–95` — the `public_limited` router; remove the
  `/v1/agents/register` route and the `AME_AGENT_ACCESS_CODE` plumbing.
- `api/src/http/agents.rs` — delete the `register` handler.

## Do
- Rewrite the `Role` enum + its serde. Then **`rg "Role::(Learner|Instructor)"`
  across `api/`** and fix every match site (quiz create gating, any
  instructor-only guard, tests). Collapse instructor/learner checks to `User`.
- Remove `agents::register`, its route, the access-code env read, and any test
  that exercised the faucet.

## Gotchas
- No `DEMO_MODE`, no Argon2 for tokens — don't reintroduce either.
- Some handlers gate "instructor or admin"; after collapse that's "user or
  admin" — but a logged-in `user` now covers what instructors did (authoring
  moved to the agent surface), so most gates simply become "authenticated".
  Use judgement per call site; note any you were unsure of in the commit body.

## Done when
- Enum is `{User, Admin, Agent}`; no `Learner`/`Instructor` anywhere in `api/`.
- `/v1/agents/register` + `AME_AGENT_ACCESS_CODE` are gone.
- `make check` passes.
