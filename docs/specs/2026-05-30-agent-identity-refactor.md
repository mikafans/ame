# Agent / Identity Refactor — Design

**Date:** 2026-05-30
**Status:** Approved (single long-lived branch `refactor/agent-identity`, one PR)

## Goal

Turn AME from an instructor/learner platform into a single-user-role platform
where **every user drives their own agents**. A user mints scoped agent tokens,
and each agent is a personalized authoring/tutoring identity that knows the
user's level, focus, progress, and next target. Authoring moves from a dedicated
instructor UI to the agent surface. Add default rate limits, a premium tier
(higher limits + data export), and an admin panel for the operator.

This is **dev-stage**: no real users, nothing promised, the operator is the only
user. Breaking schema/API changes are acceptable; no backward-compat shims.

## Decided model

1. **Agent = sub-account.** A user (the *owner*) creates agent sub-accounts.
   Each agent is its own `tb_users` row with `owner_user_id → owner`, no
   email/password (token auth only), and cannot own further agents.
2. **Role collapse to `{user, admin, agent}`.** `learner` and `instructor`
   merge into `user`; `admin` stays; `agent` is redefined as the sub-account
   type. Instructor UI/route-group and `roles/instructor.spec.ts` are removed.
3. **Self-service tokens.** Owners create agents + tokens themselves; the
   `AME_AGENT_ACCESS_CODE` env faucet and the public `POST /v1/agents/register`
   are deleted.
4. **Split memory.** *Shared truth* (level, progress) lives on the owner and is
   read by any of their agents. *Per-agent* config/memory (focus, goal, next
   target, freeform notes) lives per agent.
5. **Visibility (quizzes only).** `tb_quizzes` gains `{private, unlisted, public}`.
   Questions and exams get **no** visibility column: questions ride inside
   whatever quiz exposes them, and exam sharing is deferred. The column is the
   **source of truth** for access; existing share-links are demoted to
   presentation config. Public/unlisted quizzes are viewable anonymously, but
   **answering requires an account** (attempts are identity-bound and roll up to
   the responder's own stats).
6. **Tiers.** `plan ∈ {free, premium}` on human accounts; agents inherit the
   owner's plan. Admin toggles premium manually — no payment integration yet.
7. **Quotas + limits.** Per-token throughput limiting + per-owner plan quotas.
8. **Export.** Premium-only `GET /v1/me/export`.
9. **Admin panel + audit log.**

## Schema deltas (migrations)

All additive except the role migration. New migrations under `db/migrations/`.

### `tb_users`
```sql
ALTER TABLE tb_users ADD COLUMN owner_user_id uuid REFERENCES tb_users(id) ON DELETE CASCADE;
ALTER TABLE tb_users ADD COLUMN plan text NOT NULL DEFAULT 'free'
  CHECK (plan IN ('free', 'premium'));
-- role migration
UPDATE tb_users SET role = 'user' WHERE role IN ('learner', 'instructor');
ALTER TABLE tb_users DROP CONSTRAINT tb_users_role_check;
ALTER TABLE tb_users ADD CONSTRAINT tb_users_role_check
  CHECK (role IN ('user', 'admin', 'agent'));
-- invariants: agents are token-only and one level deep
ALTER TABLE tb_users ADD CONSTRAINT tb_users_agent_shape CHECK (
  (role = 'agent' AND owner_user_id IS NOT NULL AND email IS NULL AND password_hash IS NULL)
  OR (role <> 'agent' AND owner_user_id IS NULL)
);
```
- `plan` only meaningful on humans; agents resolve plan via `owner_user_id`.

### `tb_agent_profiles` (new)
```sql
CREATE TABLE tb_agent_profiles (
  agent_user_id uuid PRIMARY KEY REFERENCES tb_users(id) ON DELETE CASCADE,
  label         text NOT NULL,
  focus_tags    text[] NOT NULL DEFAULT '{}',
  current_goal  text,
  next_target   text,
  memory        jsonb NOT NULL DEFAULT '{}',
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now()
);
```

### Visibility
```sql
ALTER TABLE tb_quizzes ADD COLUMN visibility text NOT NULL DEFAULT 'private'
  CHECK (visibility IN ('private', 'unlisted', 'public'));
CREATE INDEX idx_quizzes_public ON tb_quizzes (created_at DESC) WHERE visibility = 'public';
```
`private` = owner + owner's agents only. `unlisted` = anyone with the link/id
(not in public listings). `public` = discoverable in public listings. Only
quizzes are shareable — questions inherit exposure from their containing quiz,
exams are not shareable yet. The existing `tb_share_links.visibility` column is
**no longer an access gate** — share-links only carry presentation flags
(`include_score`, `include_explanation`, `include_attribution`). The
anonymous-attempt path (`tb_anonymous_attempts`, `POST /v1/quizzes/{id}/embed/attempt`)
is **removed**; embed views stay read-only.

### `tb_audit_log` (new)
```sql
CREATE TABLE tb_audit_log (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  actor_user_id  uuid REFERENCES tb_users(id) ON DELETE SET NULL,
  action         text NOT NULL,
  target_type    text,
  target_id      uuid,
  metadata       jsonb NOT NULL DEFAULT '{}',
  created_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_log_created ON tb_audit_log (created_at DESC);
```

## API surface

### Agents (owner-driven)
- `POST /v1/me/agents {label, scopes, focusTags?}` → creates the agent
  sub-account + issues its token. **Secret returned once**, `sha256` stored.
- `GET /v1/me/agents` → list owner's agents (label, scopes, last_used, focus).
- `PATCH /v1/me/agents/{id}` → update label / focus / goal / next target.
- `DELETE /v1/me/agents/{id}` → revoke token + disable sub-account.

### Agent tools (called by an agent's token via `/v1/agents/run`)
- `profile.get` → the agent's own config **plus** the owner's level
  (`tb_user_tag_ratings`), recent progress, and next target.
- `memory.set` / `memory.append` → write the agent's `memory` jsonb.
- `target.set` → update `current_goal` / `next_target`.
- existing `quiz.import`, `quiz.update`, `question.create`, `question.promote`
  remain; created content is owned by the agent and rolls up to the owner.

### Visibility / listing / sharing

**Sharing model = link visibility, not invites.** The whole sharing system is
the `{private, unlisted, public}` column — there is no per-user invitation /
access-list machinery (no invites table, no email delivery). This is the
"anyone with the link" model (Google Docs / YouTube unlisted / GitHub gist):
lowest friction for "share a quiz with friends" and zero new infrastructure.
Per-recipient private sharing (cohorts/classrooms) stays out of scope.

- **Primary share path:** set `visibility = 'unlisted'` and send the canonical
  `/quizzes/{id}` URL. "Revoke" = set back to `private`. **No opaque per-link
  tokens** — UUIDv7 ids are unguessable enough (YAGNI until a link leaks).
- `tb_share_links` is **not** the sharing mechanism — it is only the embed/OG
  presentation layer (cards on external sites, iframe embeds). Secondary.
- `PATCH /v1/quizzes/{id}` accepts `visibility`. Publishing to `public` is
  plan-gated + writes an audit entry; `unlisted` is always allowed.
- Owner-scoped quiz list filters
  `WHERE visibility = 'public' OR created_by IN (me + my agents)`.
- `GET /v1/explore` (new, public) → paginated `visibility = 'public'` quizzes.
- `GET /v1/quizzes/{id}` resolves an `unlisted`/`public` quiz for **any**
  caller (incl. anonymous) as **read-only** — prompt/options for preview, never
  another user's attempts/scores. `private` → 404 for non-owners.
- Share-links keep `POST /v1/shares` but only set presentation flags; they no
  longer grant access. Creating a share auto-promotes a `private` target to
  `unlisted` (the act of sharing is the intent to expose by link).

### Cross-owner attempts (account-required)
- Taking someone else's `public`/`unlisted` quiz goes through the **normal
  authenticated session flow** — `POST /v1/sessions {quizId}` works for any
  caller whose token can read the quiz (public/unlisted), not just the owner.
- The resulting `tb_session` + attempts belong to the **responder**; stats roll
  up to the responder, never the author.
- **Anonymous answering is gated**: the embed/preview renders, but the answer
  control requires login → the frontend redirects new visitors to signup, then
  resumes the session. The `embed/attempt` write endpoints are deleted.
- Authors get aggregate counts only (how many sessions on their public content),
  never individual responders' identities or scores.

### Export
- `GET /v1/me/export` → zip/JSON of the owner's quizzes, questions, attempts,
  stats. Premium-gated, throttled.

### Admin (`role = admin`)
- `GET /v1/admin/users`, `PATCH /v1/admin/users/{id}` (plan toggle, disable).
- `GET /v1/admin/audit`.
- `POST /v1/admin/moderate` (unpublish public content).

## Authorization

- **Ownership rollup:** "my content" / quotas / billing aggregate over
  `owner + its agents`. Resolve an agent request to its owner via
  `owner_user_id`.
- **Scope ceiling:** an agent's granted scopes must be a subset of what the
  owner's plan permits. Free plan may forbid certain scopes / public publish.
- **Shared truth read:** when an agent reads level/progress, it reads the
  owner's rows, never its own.

## Rate limits & quotas

- **Throughput:** re-key `tower_governor` from IP → token id.
- **Quotas (per owner, plan-based):** agents count, question creates/day,
  active quizzes, public publishes, export. Enforced server-side before the
  write; over-quota → `429` with a clear body.

| | Free | Premium |
|---|---|---|
| Agents (tokens) | 3 | 20 |
| Req/min per token | 60 | 300 |
| Question creates/day | 50 | 2000 |
| Public quizzes | limited | yes |
| Data export | no | yes |

(Numbers are a starting point, tune later.)

## Rollout order (phases on one branch)

0. Role collapse migration + delete `AME_AGENT_ACCESS_CODE` faucet + `/v1/agents/register`.
1. Agent sub-accounts: `owner_user_id`, `POST /v1/me/agents`, act-as-owner resolution.
2. `tb_agent_profiles` + `profile.get` / `memory.*` / `target.set` tools.
3. Quiz `visibility` column + listing filter + `/v1/explore` + cross-owner
   authenticated attempts + demote share-links to presentation + delete
   anonymous-attempt path + moderation hook.
4. `plan` column + quota/limit re-keying + gates.
5. Data export.
6. Admin panel + audit log.
7. Frontend: drop instructor route-group + `roles/instructor.spec.ts`; add Agents
   page, visibility controls (incl. a public Explore view + share-link dialog),
   signup-to-answer redirect on public/unlisted content, account/plan page,
   admin UI.

## Security considerations

- Token secret shown exactly once; only `sha256` persisted (existing pattern).
- Public content is a spam/abuse surface — server-side quotas + admin moderation
  are load-bearing, not optional.
- Export is expensive — throttle and audit it.
- Agents must not escalate beyond the owner's plan/scope ceiling.
- `tb_users_agent_shape` CHECK prevents agents with passwords or nested agents.
- Public/unlisted resolution must never leak other responders' attempts, scores,
  or identities — authors see aggregate counts only.
- Cross-owner attempts run as the responder's own session; a public quiz can
  never write to or read the author's progress rows.
- Dropping the anonymous-attempt path removes an unauthenticated write surface;
  all answer writes are now token-bound and quota-counted.

## Out of scope (later)

Payment/Stripe integration, spaced repetition, per-agent rate plans distinct
from the owner, and — for sharing specifically — per-recipient private sharing
(cohorts/invites/email delivery), question-level visibility (public question
bank), exam sharing, and opaque rotating share-link tokens. All are additive
later; none are needed for "share a quiz with friends."
