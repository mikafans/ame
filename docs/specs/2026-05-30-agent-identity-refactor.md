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
5. **Visibility.** Quizzes and questions gain `{private, unlisted, public}`.
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
ALTER TABLE tb_quizzes   ADD COLUMN visibility text NOT NULL DEFAULT 'private'
  CHECK (visibility IN ('private', 'unlisted', 'public'));
ALTER TABLE tb_questions ADD COLUMN visibility text NOT NULL DEFAULT 'private'
  CHECK (visibility IN ('private', 'unlisted', 'public'));
CREATE INDEX idx_quizzes_public   ON tb_quizzes   (created_at DESC) WHERE visibility = 'public';
CREATE INDEX idx_questions_public ON tb_questions (created_at DESC) WHERE visibility = 'public';
```

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

### Visibility / listing
- List endpoints filter `WHERE visibility = 'public' OR created_by IN (me + my agents)`.
- Publishing to `public` is gated by plan + writes an audit entry.

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
3. Visibility columns + listing filters + moderation hook.
4. `plan` column + quota/limit re-keying + gates.
5. Data export.
6. Admin panel + audit log.
7. Frontend: drop instructor route-group + `roles/instructor.spec.ts`; add Agents
   page, visibility controls, account/plan page, admin UI.

## Security considerations

- Token secret shown exactly once; only `sha256` persisted (existing pattern).
- Public content is a spam/abuse surface — server-side quotas + admin moderation
  are load-bearing, not optional.
- Export is expensive — throttle and audit it.
- Agents must not escalate beyond the owner's plan/scope ceiling.
- `tb_users_agent_shape` CHECK prevents agents with passwords or nested agents.

## Out of scope (later)

Payment/Stripe integration, spaced repetition, federation/sharing between
distinct owners, per-agent rate plans distinct from the owner.
