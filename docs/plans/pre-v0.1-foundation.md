# Pre-v0.1 — How AME reached MVP *(compressed history)*

A condensed record of the build that produced the v0.1 platform. The detailed
per-phase plans (`2026-05-20-plan-*`, the design audits, the MUI-migration and
agent-identity workspaces) were retired once the work landed in code; this is the
durable summary of *what was built and why*. Canonical spec:
[`docs/specs/2026-05-20-harus-platform-design.md`](../specs/2026-05-20-harus-platform-design.md).

## The arc, in order

1. **Schema + auth.** Postgres schema; email+password auth (no SSO/SAML); API
   tokens with **scopes fixed at login**. First admin is DB-granted (`make
   db-admin`); further admins promoted in-app.
2. **Question bank.** MC / TF / essay / code questions — MC options are **bare
   strings**. Added full-text search (`prompt_tsv` GIN) and keyset pagination;
   benched at **1M rows** (PR #12, page/cursor ~4ms vs ~210ms OFFSET).
3. **Assessments + engine.** Sessions with **server-enforced deadlines**
   (`deadline_at` from `duration_min`), grading, stats + feedback. Lifecycle
   **draft → active** (publish promotes draft questions to live).
4. **Agent surface.** `role=agent` sub-accounts + agent profiles; scoped agent
   API; served OpenAPI snapshot for tooling.
5. **Frontend.** Learner + author/agent UIs, then a **full MUI migration** (no
   Tailwind for app UI; MUI Dialogs for all confirmations).

## Two refactors that shaped the model

- **Assessment unification.** Collapsed `quiz` + `exam` into one **assessment**
  entity = *structure + mode* (always-sectioned; `practice | graded` policy),
  **not** "exam = quiz with sections." Added multi-section authoring; legacy
  quiz/exam fully removed from schema, code, and scopes.
- **Agent / identity refactor.** Agent = **sub-account** (`tb_users.owner_user_id`
  self-FK; token-only, no email/password). Roles collapsed to
  `{user, admin, agent}`. Self-service token mint (`POST /v1/me/agents`); the old
  public faucet + access code were deleted. Plans `{free, premium}` + per-plan
  quotas + tiered per-owner rate limiting; premium data export. Cross-account
  access is **owner-scoped**, enforced by row-level security on `tb_questions`.

## Decisions that stuck (and one that didn't)

- **Owner isolation over public sharing.** A `{private, unlisted, public}`
  visibility model with `tb_share_links` was built and then **removed** —
  cross-account access is sub-accounts only. No `visibility` field or
  `public.publish` scope remains.
- **Explicitly out of scope:** SSO/SAML, scheduled exams (`opens_at`/`closes_at`),
  public sharing.
- **No-lockout admin rules:** self-demote / self-disable / last-admin guards;
  admin tokens revoked on demotion (re-login required after any role/scope change).

## Security hardening pass (the last pre-v0.1 work)

Token-scope validation, hashed webhook secrets, security headers, trusted-proxy
client IP, per-account login throttle, email canonicalization, admin-token revoke
on demotion, and a router-level admin guard.

## Where the platform stood at v0.1

Authors build and publish assessments; learners take graded, deadline-enforced
sessions; agents author via API; everything is owner-isolated and
security-hardened — with the admin console foundation (users, audit, health,
soft-delete moderation) in place. Forward work is tracked in
[`ROADMAP.md`](../ROADMAP.md).
